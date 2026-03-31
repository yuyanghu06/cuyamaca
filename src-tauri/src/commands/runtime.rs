use crate::models::tools::ToolRegistry;
use crate::services::agent::AgentEvent;
use crate::services::context;
use crate::services::provider::ChatMessage;
use crate::services::serial::SerialConnection;
use crate::AppState;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::ipc::Channel;
use tauri::Manager;

/// Runtime state stored in AppState — created when runtime starts, dropped when killed.
pub struct RuntimeSession {
    pub running: Arc<AtomicBool>,
    pub conversation: Vec<ChatMessage>,
    pub tools: Vec<crate::models::tools::SerialToolDefinition>,
}

#[tauri::command]
pub async fn open_runtime_window(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    // 1. Verify we have an active project with a sketch and tools
    let (manifest, project_path) = {
        let project = state.active_project.lock().map_err(|e| e.to_string())?;
        let p = project.as_ref().ok_or("No active project")?;
        if p.sketch.is_none() {
            return Err("No sketch has been approved. Flash first.".into());
        }
        (p.manifest.clone(), p.path.clone())
    };

    // 2. Load tool definitions
    let tools_path = project_path.join("tools.json");
    let tools = if tools_path.exists() {
        let data = std::fs::read_to_string(&tools_path)
            .map_err(|e| format!("Failed to read tools.json: {}", e))?;
        let registry: ToolRegistry =
            serde_json::from_str(&data).map_err(|e| format!("Invalid tools.json: {}", e))?;
        registry.tools
    } else {
        Vec::new()
    };

    // 3. Open serial connection if needed
    let needs_serial = {
        let serial = state.serial.lock().map_err(|e| e.to_string())?;
        serial.is_none()
    };
    if needs_serial {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        let conn = SerialConnection::open(
            &manifest.serial_port,
            manifest.baud_rate,
            manifest.components.clone(),
        )?;
        let mut serial = state.serial.lock().map_err(|e| e.to_string())?;
        *serial = Some(conn);
    }

    // 4. Create runtime session
    {
        let mut rt = state.runtime.lock().map_err(|e| e.to_string())?;
        *rt = Some(RuntimeSession {
            running: Arc::new(AtomicBool::new(true)),
            conversation: Vec::new(),
            tools,
        });
    }

    // 5. Create the runtime window
    let _runtime_window = tauri::WebviewWindowBuilder::new(
        &app,
        "runtime",
        tauri::WebviewUrl::App("runtime.html".into()),
    )
    .title("Cuyamaca — Runtime")
    .inner_size(1100.0, 700.0)
    .min_inner_size(800.0, 500.0)
    .build()
    .map_err(|e| format!("Failed to create runtime window: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn runtime_send_message(
    state: tauri::State<'_, AppState>,
    message: String,
    on_event: Channel<AgentEvent>,
) -> Result<(), String> {
    // Gather everything we need, releasing locks before async work
    let model = state.model_manager.runtime_model().await?;

    let (running, mut conversation, tools, manifest) = {
        let rt = state.runtime.lock().map_err(|e| e.to_string())?;
        let session = rt.as_ref().ok_or("No runtime session active")?;

        let project = state.active_project.lock().map_err(|e| e.to_string())?;
        let manifest = project
            .as_ref()
            .ok_or("No active project")?
            .manifest
            .clone();

        (
            Arc::clone(&session.running),
            session.conversation.clone(),
            session.tools.clone(),
            manifest,
        )
    };

    // Verify serial is available
    {
        let serial = state.serial.lock().map_err(|e| e.to_string())?;
        if serial.is_none() {
            return Err("No serial connection".into());
        }
    }

    running.store(true, Ordering::SeqCst);

    // Inline the agent loop here to avoid holding MutexGuard across awaits.
    // Each serial operation uses a scoped lock/unlock pattern.

    let mut iteration = 0;
    let max_iterations = 10;
    let mut current_message = message.clone();
    let mut is_first = true;

    loop {
        if !running.load(Ordering::SeqCst) {
            return Err("Killed by user".into());
        }
        if iteration >= max_iterations {
            let _ = on_event.send(AgentEvent::ModelResponse(
                "Reached maximum tool call iterations. Stopping.".into(),
            ));
            break;
        }

        // Assemble context (brief lock)
        let request = {
            let serial_guard = state.serial.lock().map_err(|e| e.to_string())?;
            let conn = serial_guard.as_ref().ok_or("Serial disconnected")?;
            let sensor_state_lock = conn.sensor_state().read().unwrap();
            let viz = crate::services::sensor_viz::SensorVizRenderer::render(&sensor_state_lock);
            let req = context::assemble(
                &sensor_state_lock,
                viz.as_deref(),
                None,
                &tools,
                &conversation,
                &current_message,
                &manifest,
            );
            drop(sensor_state_lock);
            req
        };

        // Call the model (async — no locks held)
        let response = model.complete(request).await?;

        // Add user message to conversation (first iteration only)
        if is_first {
            conversation.push(ChatMessage {
                role: "user".into(),
                content: crate::services::provider::MessageContent::Text(message.clone()),
            });
            is_first = false;
        }

        // Process text response
        if !response.content.is_empty() {
            let _ = on_event.send(AgentEvent::ModelResponse(response.content.clone()));
            conversation.push(ChatMessage {
                role: "assistant".into(),
                content: crate::services::provider::MessageContent::Text(
                    response.content.clone(),
                ),
            });
        }

        // Process tool calls
        let tool_calls = response.tool_calls.unwrap_or_default();
        if tool_calls.is_empty() {
            break;
        }

        let mut session_ended = false;

        for tc in &tool_calls {
            if !running.load(Ordering::SeqCst) {
                return Err("Killed by user".into());
            }

            let _ = on_event.send(AgentEvent::ToolCallStarted {
                tool_name: tc.name.clone(),
                arguments: tc.arguments.clone(),
            });

            let result = match tc.name.as_str() {
                "read_sensor_state" => {
                    let serial_guard = state.serial.lock().map_err(|e| e.to_string())?;
                    let conn = serial_guard.as_ref().ok_or("Serial disconnected")?;
                    crate::services::tool_dispatch::handle_read_sensor_state(conn)
                }
                "wait_milliseconds" => {
                    let ms = tc
                        .arguments
                        .get("milliseconds")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(1000);
                    tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
                    crate::services::tool_dispatch::ToolResult {
                        tool_name: "wait_milliseconds".into(),
                        success: true,
                        output: format!("Waited {}ms", ms),
                    }
                }
                "end_session" => {
                    session_ended = true;
                    crate::services::tool_dispatch::ToolResult {
                        tool_name: "end_session".into(),
                        success: true,
                        output: "Session ended by model".into(),
                    }
                }
                _ => {
                    let serial_guard = state.serial.lock().map_err(|e| e.to_string())?;
                    let conn = serial_guard.as_ref().ok_or("Serial disconnected")?;
                    match crate::services::tool_dispatch::execute_serial_tool(tc, &tools, conn) {
                        Ok(r) => r,
                        Err(e) => crate::services::tool_dispatch::ToolResult {
                            tool_name: tc.name.clone(),
                            success: false,
                            output: e,
                        },
                    }
                }
            };

            let _ = on_event.send(AgentEvent::ToolCallCompleted {
                tool_name: result.tool_name.clone(),
                success: result.success,
                output: result.output.clone(),
            });

            conversation.push(ChatMessage {
                role: "tool".into(),
                content: crate::services::provider::MessageContent::Text(
                    serde_json::to_string(&result).unwrap_or_default(),
                ),
            });

            if session_ended {
                let _ = on_event.send(AgentEvent::SessionEnded);
                running.store(false, Ordering::SeqCst);
                // Update session conversation
                if let Ok(mut rt) = state.runtime.lock() {
                    if let Some(session) = rt.as_mut() {
                        session.conversation = conversation;
                    }
                }
                return Ok(());
            }
        }

        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        current_message =
            "Tool calls completed. Observe updated sensor state and continue or respond.".into();
        iteration += 1;
    }

    let _ = on_event.send(AgentEvent::TurnComplete);

    // Persist conversation back to runtime session
    if let Ok(mut rt) = state.runtime.lock() {
        if let Some(session) = rt.as_mut() {
            session.conversation = conversation;
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn runtime_kill(state: tauri::State<'_, AppState>) -> Result<(), String> {
    // 1. Signal the agent loop to stop
    {
        let rt = state.runtime.lock().map_err(|e| e.to_string())?;
        if let Some(session) = rt.as_ref() {
            session.running.store(false, Ordering::SeqCst);
        }
    }

    // 2. Send emergency stop and close serial
    {
        let serial = state.serial.lock().map_err(|e| e.to_string())?;
        if let Some(conn) = serial.as_ref() {
            conn.stop(); // sends CMD:stop
        }
        // Don't drop the connection yet — close_serial does that
    }

    // 3. Clear runtime session
    {
        let mut rt = state.runtime.lock().map_err(|e| e.to_string())?;
        *rt = None;
    }

    Ok(())
}

#[tauri::command]
pub async fn close_runtime_window(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    // Kill the agent if running
    runtime_kill(state).await?;

    // Close the window
    if let Some(window) = app.get_webview_window("runtime") {
        window.close().map_err(|e| e.to_string())?;
    }

    Ok(())
}
