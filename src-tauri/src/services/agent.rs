use crate::models::manifest::Manifest;
use crate::models::tools::SerialToolDefinition;
use crate::services::context;
use crate::services::provider::{ChatMessage, CompletionResponse, MessageContent, ModelProvider};
use crate::services::sensor_viz::SensorVizRenderer;
use crate::services::serial::SerialConnection;
use crate::services::tool_dispatch::{self, ToolResult};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::ipc::Channel;

#[allow(dead_code)]
const MAX_TOOL_ITERATIONS: usize = 10;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "event", content = "data")]
pub enum AgentEvent {
    ModelResponse(String),
    ToolCallStarted {
        tool_name: String,
        arguments: serde_json::Value,
    },
    ToolCallCompleted {
        tool_name: String,
        success: bool,
        output: String,
    },
    TurnComplete,
    SessionEnded,
    #[allow(dead_code)]
    Error(String),
}

/// Run a single agent turn: user message → model → tool calls → repeat → done.
///
/// This function blocks the command handler until the turn is complete.
/// Kill can interrupt it via the running flag.
#[allow(dead_code)]
pub async fn run_turn(
    user_message: &str,
    model: Arc<dyn ModelProvider>,
    serial: &SerialConnection,
    tools: &[SerialToolDefinition],
    conversation: &mut Vec<ChatMessage>,
    manifest: &Manifest,
    running: &Arc<AtomicBool>,
    on_event: &Channel<AgentEvent>,
) -> Result<(), String> {
    let mut iteration = 0;

    // The user message for context assembly. On the first iteration this is the
    // actual user message; on subsequent iterations (after tool calls) we send
    // a follow-up prompt asking the model to continue.
    let mut current_message = user_message.to_string();
    let mut is_first = true;

    loop {
        if !running.load(Ordering::SeqCst) {
            return Err("Killed by user".into());
        }

        if iteration >= MAX_TOOL_ITERATIONS {
            let _ = on_event.send(AgentEvent::ModelResponse(
                "Reached maximum tool call iterations. Stopping.".into(),
            ));
            break;
        }

        // 1. Collect current sensor context
        let sensor_snapshot = serial.get_sensor_state_snapshot();
        let sensor_viz = {
            let state = serial.sensor_state().read().unwrap();
            SensorVizRenderer::render(&state)
        };

        // 2. Assemble the completion request
        let sensor_state_ref = serial.sensor_state().read().unwrap();
        let request = context::assemble(
            &sensor_state_ref,
            sensor_viz.as_deref(),
            None, // Camera frames — Phase 7 wires this, but actual capture is future
            tools,
            conversation,
            &current_message,
            manifest,
        );
        drop(sensor_state_ref);
        let _ = sensor_snapshot; // used above, drop the binding

        // 3. Call the runtime model (non-streaming for reliable tool call parsing)
        let response: CompletionResponse = model.complete(request).await?;

        // 4. Add user message to conversation history (only on first iteration)
        if is_first {
            conversation.push(ChatMessage {
                role: "user".into(),
                content: MessageContent::Text(user_message.to_string()),
            });
            is_first = false;
        }

        // 5. Process text response
        if !response.content.is_empty() {
            let _ = on_event.send(AgentEvent::ModelResponse(response.content.clone()));
            conversation.push(ChatMessage {
                role: "assistant".into(),
                content: MessageContent::Text(response.content.clone()),
            });
        }

        // 6. Process tool calls
        let tool_calls = response.tool_calls.unwrap_or_default();
        if tool_calls.is_empty() {
            break; // Turn done — model responded with text only
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

            let result: ToolResult = match tc.name.as_str() {
                "read_sensor_state" => tool_dispatch::handle_read_sensor_state(serial),
                "wait_milliseconds" => {
                    let ms = tc
                        .arguments
                        .get("milliseconds")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(1000);
                    tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
                    ToolResult {
                        tool_name: "wait_milliseconds".into(),
                        success: true,
                        output: format!("Waited {}ms", ms),
                    }
                }
                "end_session" => {
                    session_ended = true;
                    ToolResult {
                        tool_name: "end_session".into(),
                        success: true,
                        output: "Session ended by model".into(),
                    }
                }
                _ => match tool_dispatch::execute_serial_tool(tc, tools, serial) {
                    Ok(r) => r,
                    Err(e) => ToolResult {
                        tool_name: tc.name.clone(),
                        success: false,
                        output: e,
                    },
                },
            };

            let _ = on_event.send(AgentEvent::ToolCallCompleted {
                tool_name: result.tool_name.clone(),
                success: result.success,
                output: result.output.clone(),
            });

            // Add tool result to conversation
            conversation.push(ChatMessage {
                role: "tool".into(),
                content: MessageContent::Text(
                    serde_json::to_string(&result).unwrap_or_default(),
                ),
            });

            if session_ended {
                let _ = on_event.send(AgentEvent::SessionEnded);
                running.store(false, Ordering::SeqCst);
                return Ok(());
            }
        }

        // Brief delay for board to process commands and update sensors
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;

        // Continue the loop with a follow-up prompt
        current_message =
            "Tool calls completed. Observe the updated sensor state and continue or respond."
                .into();
        iteration += 1;
    }

    let _ = on_event.send(AgentEvent::TurnComplete);
    Ok(())
}
