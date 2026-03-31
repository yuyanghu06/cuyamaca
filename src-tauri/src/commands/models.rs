use crate::services::keystore;
use crate::services::model_manager::{ProviderType, SlotConfig};
use crate::services::ollama::OllamaProvider;
use crate::services::provider::{ChatMessage, CompletionRequest, MessageContent, ModelProvider};
use crate::AppState;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;
use tauri_plugin_store::StoreExt;

#[derive(Debug, Serialize)]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub requires_key: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SlotConfigResponse {
    pub provider: String,
    pub model: String,
    pub multimodal_warning: bool,
}

#[tauri::command]
pub fn list_providers() -> Vec<ProviderInfo> {
    vec![
        ProviderInfo {
            id: "ollama".to_string(),
            name: "Ollama (Local)".to_string(),
            requires_key: false,
        },
        ProviderInfo {
            id: "openai".to_string(),
            name: "OpenAI".to_string(),
            requires_key: true,
        },
        ProviderInfo {
            id: "anthropic".to_string(),
            name: "Anthropic".to_string(),
            requires_key: true,
        },
        ProviderInfo {
            id: "google".to_string(),
            name: "Google".to_string(),
            requires_key: true,
        },
        ProviderInfo {
            id: "mistral".to_string(),
            name: "Mistral".to_string(),
            requires_key: true,
        },
    ]
}

#[tauri::command]
pub async fn configure_model_slot(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    slot: String,
    provider: String,
    model: String,
    api_key: Option<String>,
) -> Result<SlotConfigResponse, String> {
    let provider_type: ProviderType = provider.parse()?;

    // Store API key in keychain if provided
    if let Some(ref key) = api_key {
        if provider_type != ProviderType::Ollama {
            keystore::store_api_key(&provider, key)?;
        }
    }

    // Resolve API key: use provided key or fetch from keychain
    let resolved_key = match api_key {
        Some(key) => Some(key),
        None if provider_type != ProviderType::Ollama => keystore::get_api_key(&provider)?,
        _ => None,
    };

    let config = SlotConfig {
        provider: provider_type,
        model: model.clone(),
    };

    let multimodal_warning = match slot.as_str() {
        "code" => {
            state.model_manager.configure_code_model(config, resolved_key).await?;
            false
        }
        "runtime" => {
            let supports_multimodal = state
                .model_manager
                .configure_runtime_model(config, resolved_key)
                .await?;
            !supports_multimodal
        }
        _ => return Err(format!("Invalid slot: {}. Use 'code' or 'runtime'", slot)),
    };

    // Persist provider + model so they survive restarts
    let store = app
        .store("settings.json")
        .map_err(|e| format!("Failed to open settings store: {}", e))?;
    store.set(
        format!("model_config_{}", slot),
        serde_json::json!({ "provider": provider, "model": model }),
    );
    store
        .save()
        .map_err(|e| format!("Failed to save settings: {}", e))?;

    Ok(SlotConfigResponse {
        provider,
        model,
        multimodal_warning,
    })
}

#[tauri::command]
pub async fn get_slot_config(
    state: tauri::State<'_, AppState>,
    slot: String,
) -> Result<Option<SlotConfigResponse>, String> {
    let config = match slot.as_str() {
        "code" => state.model_manager.code_config().await,
        "runtime" => state.model_manager.runtime_config().await,
        _ => return Err(format!("Invalid slot: {}", slot)),
    };

    Ok(config.map(|c| SlotConfigResponse {
        provider: c.provider.to_string(),
        model: c.model,
        multimodal_warning: false,
    }))
}

#[derive(Debug, Serialize)]
pub struct ModelTestResult {
    pub ok: bool,
    pub message: String,
}

#[tauri::command]
pub async fn check_model_health(
    state: tauri::State<'_, AppState>,
    slot: String,
) -> Result<ModelTestResult, String> {
    let provider = match slot.as_str() {
        "code" => state.model_manager.code_model().await,
        "runtime" => state.model_manager.runtime_model().await,
        _ => return Err(format!("Invalid slot: {}", slot)),
    };

    let provider = match provider {
        Ok(p) => p,
        Err(e) => return Ok(ModelTestResult { ok: false, message: format!("Not configured: {}", e) }),
    };

    // First do a quick health check
    if !provider.is_healthy().await {
        return Ok(ModelTestResult {
            ok: false,
            message: "Cannot reach model endpoint. Make sure Ollama is running.".to_string(),
        });
    }

    // Send a minimal test completion to verify the model actually responds
    let request = CompletionRequest {
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: MessageContent::Text("Reply with only the word: OK".to_string()),
        }],
        system_prompt: Some("You are a test assistant. Follow instructions exactly.".to_string()),
        temperature: Some(0.0),
        max_tokens: Some(8),
        tools: None,
    };

    match provider.complete(request).await {
        Ok(resp) if !resp.content.is_empty() => Ok(ModelTestResult {
            ok: true,
            message: format!("Success! Model responded: \"{}\"", resp.content.trim()),
        }),
        Ok(_) => Ok(ModelTestResult {
            ok: false,
            message: "Model returned an empty response.".to_string(),
        }),
        Err(e) => Ok(ModelTestResult {
            ok: false,
            message: format!("Failure: {}", e),
        }),
    }
}

#[tauri::command]
pub async fn check_ollama_health() -> Result<bool, String> {
    let provider = OllamaProvider::new("".to_string(), None);
    Ok(provider.is_healthy().await)
}

#[tauri::command]
pub async fn list_ollama_models() -> Result<Vec<crate::services::provider::ModelInfo>, String> {
    let provider = OllamaProvider::new("".to_string(), None);
    provider.list_models().await
}

#[tauri::command]
pub async fn store_api_key(provider: String, key: String) -> Result<(), String> {
    keystore::store_api_key(&provider, &key)
}

#[tauri::command]
pub async fn has_api_key(provider: String) -> Result<bool, String> {
    Ok(keystore::get_api_key(&provider)?.is_some())
}

// ── Ollama model management ──

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "event", content = "data")]
pub enum PullProgress {
    Started,
    Downloading { completed: u64, total: u64 },
    Verifying,
    Succeeded,
    Failed { error: String },
}

#[tauri::command]
pub async fn pull_ollama_model(
    model: String,
    on_progress: Channel<PullProgress>,
) -> Result<(), String> {
    let client = reqwest::Client::new();
    let _ = on_progress.send(PullProgress::Started);

    let resp = client
        .post("http://localhost:11434/api/pull")
        .json(&serde_json::json!({ "name": model, "stream": true }))
        .timeout(std::time::Duration::from_secs(3600))
        .send()
        .await
        .map_err(|e| format!("Failed to connect to Ollama: {}", e))?;

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        let _ = on_progress.send(PullProgress::Failed { error: text.clone() });
        return Err(text);
    }

    let mut stream = resp.bytes_stream();
    let mut buffer = Vec::new();

    while let Some(chunk) = stream.next().await {
        let bytes = chunk.map_err(|e| format!("Stream error: {}", e))?;
        buffer.extend_from_slice(&bytes);

        while let Some(pos) = buffer.iter().position(|&b| b == b'\n') {
            let line: Vec<u8> = buffer.drain(..=pos).collect();
            let trimmed = String::from_utf8_lossy(&line).trim().to_string();
            if trimmed.is_empty() {
                continue;
            }

            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&trimmed) {
                if let Some(error) = data["error"].as_str() {
                    let _ = on_progress.send(PullProgress::Failed {
                        error: error.to_string(),
                    });
                    return Err(error.to_string());
                }

                let status = data["status"].as_str().unwrap_or("");
                if status.contains("pulling") || status.contains("downloading") {
                    let completed = data["completed"].as_u64().unwrap_or(0);
                    let total = data["total"].as_u64().unwrap_or(0);
                    if total > 0 {
                        let _ = on_progress.send(PullProgress::Downloading {
                            completed,
                            total,
                        });
                    }
                } else if status.contains("verifying") || status.contains("writing") {
                    let _ = on_progress.send(PullProgress::Verifying);
                } else if status == "success" {
                    let _ = on_progress.send(PullProgress::Succeeded);
                }
            }
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn delete_ollama_model(model: String) -> Result<(), String> {
    let client = reqwest::Client::new();
    let resp = client
        .delete("http://localhost:11434/api/delete")
        .json(&serde_json::json!({ "name": model }))
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| format!("Failed to delete model: {}", e))?;

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Delete failed: {}", text));
    }

    Ok(())
}
