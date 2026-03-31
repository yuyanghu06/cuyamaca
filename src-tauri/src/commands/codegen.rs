use crate::models::tools::{GeneratedSketchResponse, SerialToolDefinition, ToolRegistry};
use crate::services::code_gen::{
    extract_code_block_pub as extract_code_block, extract_text_outside_code, has_sketch_code,
    compute_diff, save_sketch_version, CodeGenService,
};
use crate::services::provider::{ChatMessage, CompletionRequest, MessageContent, StreamChunk};
use crate::AppState;
use std::fs;
use std::sync::Mutex;
use tauri::ipc::Channel;
use tokio::sync::mpsc;

pub struct ConversationState {
    pub history: Vec<ChatMessage>,
}

impl ConversationState {
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
        }
    }
}

// Conversation history stored per-app (scoped to active project implicitly)
static CONVERSATION: std::sync::LazyLock<Mutex<ConversationState>> =
    std::sync::LazyLock::new(|| Mutex::new(ConversationState::new()));

#[tauri::command]
pub async fn generate_sketch(
    state: tauri::State<'_, AppState>,
    instruction: Option<String>,
) -> Result<GeneratedSketchResponse, String> {
    let manifest = {
        let active = state
            .active_project
            .lock()
            .map_err(|e| e.to_string())?;
        let project = active.as_ref().ok_or("No active project")?;
        project.manifest.clone()
    };

    let extra_prompt = state.code_gen_prompt.lock().map_err(|e| e.to_string())?.clone();
    let provider = state.model_manager.code_model().await?;
    CodeGenService::generate_sketch(
        provider.as_ref(),
        &manifest,
        Some(extra_prompt.as_str()).filter(|s| !s.is_empty()),
        instruction.as_deref().filter(|s| !s.trim().is_empty()),
    ).await
}

#[tauri::command]
pub async fn modify_sketch(
    state: tauri::State<'_, AppState>,
    instruction: String,
) -> Result<GeneratedSketchResponse, String> {
    let (manifest, current_sketch) = {
        let active = state
            .active_project
            .lock()
            .map_err(|e| e.to_string())?;
        let project = active.as_ref().ok_or("No active project")?;
        let sketch = project
            .sketch
            .as_ref()
            .ok_or("No sketch to modify")?
            .clone();
        (project.manifest.clone(), sketch)
    };

    let history = {
        let conv = CONVERSATION.lock().map_err(|e| e.to_string())?;
        conv.history.clone()
    };

    let extra_prompt = state.code_gen_prompt.lock().map_err(|e| e.to_string())?.clone();
    let provider = state.model_manager.code_model().await?;
    CodeGenService::modify_sketch(
        provider.as_ref(),
        &manifest,
        &current_sketch,
        &instruction,
        &history,
        Some(extra_prompt.as_str()).filter(|s| !s.is_empty()),
    )
    .await
}

#[tauri::command]
pub async fn approve_sketch(
    state: tauri::State<'_, AppState>,
    sketch_code: String,
) -> Result<(), String> {
    // Scope 1: update project state and get what we need for tool synthesis
    let (manifest, project_path) = {
        let mut active = state
            .active_project
            .lock()
            .map_err(|e| e.to_string())?;
        let project = active.as_mut().ok_or("No active project")?;

        // Save current sketch to version history if one exists
        if let Some(ref old_sketch) = project.sketch {
            save_sketch_version(&project.path, old_sketch)?;
        }

        // Write new sketch to disk
        fs::write(project.path.join("sketch.ino"), &sketch_code)
            .map_err(|e| format!("Failed to write sketch: {}", e))?;

        project.sketch = Some(sketch_code.clone());

        (project.manifest.clone(), project.path.clone())
    }; // MutexGuard dropped here

    // Scope 2: tool synthesis (async, lock-free)
    match state.model_manager.code_model().await {
        Ok(provider) => {
            match CodeGenService::synthesize_tools(provider.as_ref(), &manifest, &sketch_code)
                .await
            {
                Ok(tools) => {
                    let registry = ToolRegistry { tools };
                    let json = serde_json::to_string_pretty(&registry)
                        .map_err(|e| format!("Failed to serialize tools: {}", e))?;
                    fs::write(project_path.join("tools.json"), json)
                        .map_err(|e| format!("Failed to write tools.json: {}", e))?;

                    // Update project state
                    let mut active = state
                        .active_project
                        .lock()
                        .map_err(|e| e.to_string())?;
                    if let Some(ref mut p) = *active {
                        p.has_tools = true;
                    }
                }
                Err(e) => {
                    eprintln!("Tool synthesis failed (non-fatal): {}", e);
                }
            }
        }
        Err(_) => {
            eprintln!("Code model not configured — skipping tool synthesis");
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn reject_sketch(
    _state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    // Nothing to do server-side — the frontend discards the pending sketch
    Ok(())
}

#[tauri::command]
pub async fn upload_sketch(
    state: tauri::State<'_, AppState>,
    sketch_content: String,
) -> Result<GeneratedSketchResponse, String> {
    let (manifest, current_sketch) = {
        let active = state
            .active_project
            .lock()
            .map_err(|e| e.to_string())?;
        let project = active.as_ref().ok_or("No active project")?;
        (project.manifest.clone(), project.sketch.clone())
    };

    // Compute diff against current sketch if one exists
    let diff = current_sketch
        .as_ref()
        .map(|old| crate::services::code_gen::compute_diff(old, &sketch_content));

    // Try to have the code model verify and potentially modify the uploaded sketch
    match state.model_manager.code_model().await {
        Ok(provider) => {
            let system_prompt = crate::services::code_gen::get_review_prompt().to_string();

            let user_prompt = format!(
                "Review this sketch for a project with this manifest:\n```json\n{}\n```\n\nSketch:\n```cpp\n{}\n```",
                serde_json::to_string_pretty(&manifest).unwrap_or_default(),
                sketch_content
            );

            let request = crate::services::provider::CompletionRequest {
                messages: vec![ChatMessage {
                    role: "user".to_string(),
                    content: MessageContent::Text(user_prompt),
                }],
                system_prompt: Some(system_prompt),
                temperature: Some(0.2),
                max_tokens: Some(4096),
                tools: None,
            };

            match provider.complete(request).await {
                Ok(response) => {
                    if response.content.contains("SKETCH_OK") {
                        // Sketch is fine as-is
                        Ok(GeneratedSketchResponse {
                            code: sketch_content,
                            diff,
                        })
                    } else {
                        // Model suggested modifications
                        let modified = crate::services::code_gen::extract_code_block_pub(&response.content);
                        let new_diff = current_sketch
                            .as_ref()
                            .map(|old| crate::services::code_gen::compute_diff(old, &modified));
                        Ok(GeneratedSketchResponse {
                            code: modified,
                            diff: new_diff.or(diff),
                        })
                    }
                }
                Err(_) => {
                    // Model unavailable — just use the sketch as-is
                    Ok(GeneratedSketchResponse {
                        code: sketch_content,
                        diff,
                    })
                }
            }
        }
        Err(_) => {
            // No code model configured — use as-is
            Ok(GeneratedSketchResponse {
                code: sketch_content,
                diff,
            })
        }
    }
}

#[tauri::command]
pub async fn get_sketch(
    state: tauri::State<'_, AppState>,
) -> Result<Option<String>, String> {
    let active = state
        .active_project
        .lock()
        .map_err(|e| e.to_string())?;
    let project = active.as_ref().ok_or("No active project")?;
    Ok(project.sketch.clone())
}

#[tauri::command]
pub async fn get_tools(
    state: tauri::State<'_, AppState>,
) -> Result<Option<Vec<SerialToolDefinition>>, String> {
    let active = state
        .active_project
        .lock()
        .map_err(|e| e.to_string())?;
    let project = active.as_ref().ok_or("No active project")?;

    let tools_path = project.path.join("tools.json");
    if !tools_path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&tools_path)
        .map_err(|e| format!("Failed to read tools.json: {}", e))?;
    let registry: ToolRegistry = serde_json::from_str(&content)
        .map_err(|e| format!("Invalid tools.json: {}", e))?;

    Ok(Some(registry.tools))
}

#[tauri::command]
pub async fn send_chat_message(
    state: tauri::State<'_, AppState>,
    message: String,
) -> Result<ChatResponse, String> {
    let (manifest, current_sketch) = {
        let active = state
            .active_project
            .lock()
            .map_err(|e| e.to_string())?;
        let project = active.as_ref().ok_or("No active project")?;
        (project.manifest.clone(), project.sketch.clone())
    };

    // Add user message to conversation history
    {
        let mut conv = CONVERSATION.lock().map_err(|e| e.to_string())?;
        conv.history.push(ChatMessage {
            role: "user".to_string(),
            content: MessageContent::Text(message.clone()),
        });
    }

    let extra_prompt = state.code_gen_prompt.lock().map_err(|e| e.to_string())?.clone();
    let provider = state.model_manager.code_model().await?;

    let history = {
        let conv = CONVERSATION.lock().map_err(|e| e.to_string())?;
        conv.history.clone()
    };

    let (text, sketch) = CodeGenService::chat_modify(
        provider.as_ref(),
        &manifest,
        current_sketch.as_deref(),
        &history,
        Some(extra_prompt.as_str()).filter(|s| !s.is_empty()),
    )
    .await?;

    // Add assistant response to conversation history
    {
        let mut conv = CONVERSATION.lock().map_err(|e| e.to_string())?;
        conv.history.push(ChatMessage {
            role: "assistant".to_string(),
            content: MessageContent::Text(text.clone()),
        });
    }

    Ok(ChatResponse {
        text,
        sketch,
    })
}

#[tauri::command]
pub async fn clear_chat_history() -> Result<(), String> {
    let mut conv = CONVERSATION.lock().map_err(|e| e.to_string())?;
    conv.history.clear();
    Ok(())
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChatResponse {
    pub text: String,
    pub sketch: Option<GeneratedSketchResponse>,
}

#[derive(Clone, serde::Serialize)]
#[serde(tag = "type", content = "data")]
pub enum ChatStreamEvent {
    Token(String),
    Complete(ChatResponse),
    Error(String),
}

#[tauri::command]
pub async fn stream_chat_message(
    state: tauri::State<'_, AppState>,
    message: String,
    channel: Channel<ChatStreamEvent>,
) -> Result<(), String> {
    let (manifest, current_sketch) = {
        let active = state
            .active_project
            .lock()
            .map_err(|e| e.to_string())?;
        let project = active.as_ref().ok_or("No active project")?;
        (project.manifest.clone(), project.sketch.clone())
    };

    // Add user message to conversation history
    {
        let mut conv = CONVERSATION.lock().map_err(|e| e.to_string())?;
        conv.history.push(ChatMessage {
            role: "user".to_string(),
            content: MessageContent::Text(message.clone()),
        });
    }

    let extra_prompt = state.code_gen_prompt.lock().map_err(|e| e.to_string())?.clone();
    let provider = state.model_manager.code_model().await?;

    let history = {
        let conv = CONVERSATION.lock().map_err(|e| e.to_string())?;
        conv.history.clone()
    };

    let system_prompt = {
        use crate::services::code_gen::build_chat_system_prompt_pub;
        build_chat_system_prompt_pub(
            &manifest,
            current_sketch.as_deref(),
            Some(extra_prompt.as_str()).filter(|s| !s.is_empty()),
        )
    };

    let request = CompletionRequest {
        messages: history,
        system_prompt: Some(system_prompt),
        temperature: Some(0.3),
        max_tokens: Some(4096),
        tools: None,
    };

    let (tx, mut rx) = mpsc::channel::<StreamChunk>(128);

    // Spawn task to forward stream chunks to channel
    let chan = channel.clone();
    let collector_handle = tokio::spawn(async move {
        let mut full_text = String::new();
        while let Some(chunk) = rx.recv().await {
            if !chunk.content.is_empty() {
                full_text.push_str(&chunk.content);
                let _ = chan.send(ChatStreamEvent::Token(chunk.content));
            }
        }
        full_text
    });

    // Run streaming provider
    let stream_result = provider.complete_stream(request, tx).await;

    // Wait for collector to finish accumulating
    let full_text = collector_handle.await.unwrap_or_default();

    if let Err(e) = stream_result {
        let _ = channel.send(ChatStreamEvent::Error(e.clone()));
        return Err(e);
    }

    // Parse text for sketch code block
    let (text, sketch) = if full_text.contains("```") && has_sketch_code(&full_text) {
        let new_sketch = extract_code_block(&full_text);
        let diff = current_sketch.as_deref().map(|old| compute_diff(old, &new_sketch));
        let text = extract_text_outside_code(&full_text);
        (text, Some(GeneratedSketchResponse { code: new_sketch, diff }))
    } else {
        (full_text.clone(), None)
    };

    // Add assistant response to history
    {
        let mut conv = CONVERSATION.lock().map_err(|e| e.to_string())?;
        conv.history.push(ChatMessage {
            role: "assistant".to_string(),
            content: MessageContent::Text(text.clone()),
        });
    }

    let _ = channel.send(ChatStreamEvent::Complete(ChatResponse { text, sketch }));

    Ok(())
}
