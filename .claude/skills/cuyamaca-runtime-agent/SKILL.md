---
name: cuyamaca-runtime-agent
description: Build the runtime agent loop for Cuyamaca — the agentic control loop where the runtime model reads sensor context, decides tool calls, writes serial commands, and iterates until the user stops it. Use this skill whenever the user wants to implement the runtime window, build the agent loop, assemble multimodal context for the runtime model, implement tool call dispatch via serial, add the kill button, or references "phase 7", "runtime agent", "agent loop", "runtime window", "runtime model", "tool calling", "kill button", "agentic loop", "multimodal context", or "control loop". Also trigger when the user asks about feeding sensor data to a vision model, executing tool calls as serial commands, or the observe-decide-act cycle. This skill assumes Phase 6 is complete (serial communication, sensor parsing, sensor visualization).
---

# Phase 7 — Runtime Agent Loop

This skill builds the core agentic control loop: the runtime model observes sensor state, camera frames, and sensor visualizations, decides which tools to call, the backend translates those tool calls into serial commands, the board executes and reports back, and the loop repeats. This is the phase where the robot actually moves.

## What This Skill Produces

- The Runtime Window: a separate Tauri window that opens after flashing
- Multimodal context assembler: structured text + sensor viz PNG + camera frame → model input
- Tool call dispatcher: model tool calls → CMD serial commands
- Agentic loop: observe → decide → act → observe (repeats until killed)
- Chat interface in the runtime window for user prompts
- Tool call confirmation pills in the chat
- The Kill button: always-visible emergency stop
- Lifecycle tools: read_sensor_state, wait_milliseconds, get_camera_frame, end_session

## Prerequisites

- Phase 6 complete: serial connection, sensor parsing, sensor visualization all working
- Phase 2 complete: runtime model slot configured with a multimodal model
- A flashed board connected and outputting sensor data

## Step 1: Runtime Window Creation

The runtime window is a separate Tauri window that opens after a successful flash. It is independent from the project window — closing it terminates the serial session but does not close the project.

```rust
// src-tauri/src/commands/runtime.rs

#[tauri::command]
pub async fn open_runtime_window(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    // Verify:
    // 1. A sketch has been flashed
    // 2. The runtime model slot is configured
    // 3. The serial port is available
    
    // Open serial connection
    // Start sensor state collection
    
    // Create new Tauri window
    let runtime_window = tauri::WebviewWindowBuilder::new(
        &app,
        "runtime",
        tauri::WebviewUrl::App("runtime.html".into()),
    )
    .title("Cuyamaca — Runtime")
    .inner_size(1100.0, 700.0)
    .min_inner_size(800.0, 500.0)
    .build()
    .map_err(|e| e.to_string())?;
    
    Ok(())
}
```

The runtime window uses a separate HTML entry point (`runtime.html`) that renders the runtime-specific layout. This keeps the project window and runtime window as independent React roots.

### Runtime window layout

```
┌─────────────────────────────────────┬──────────────────┐
│                                     │  Serial Monitor  │
│  Chat (model + tool call pills)     │  (raw output)    │
│                                     │                  │
│                                     │ ──────────────── │
│                                     │  Sensor State    │
│                                     │  (parsed, live)  │
│                                     │                  │
│                                     │ ──────────────── │
│                                     │  Sensor Viz      │
│  ─────────────────────────────────  │  (images)        │
│  [Input capsule]          [KILL]    │                  │
└─────────────────────────────────────┴──────────────────┘
```

Left side (flex, ~65%): chat interface with tool call pills.
Right side (fixed, ~35%): serial monitor, sensor state, sensor visualizations stacked vertically with adjustable splits.

## Step 2: Multimodal Context Assembly

The context assembler builds the input for each runtime model turn. It combines structured text, sensor visualization images, and camera frames into a single `CompletionRequest`.

```rust
// src-tauri/src/services/context.rs

pub struct ContextAssembler;

impl ContextAssembler {
    pub fn assemble(
        sensor_state: &SensorStateStore,
        sensor_viz: Option<&[u8]>,      // PNG bytes
        camera_frame: Option<&[u8]>,     // JPEG bytes
        tools: &[ToolDefinition],
        conversation: &[ChatMessage],
        user_message: &str,
        manifest: &Manifest,
    ) -> CompletionRequest {
        let system_prompt = build_runtime_system_prompt(manifest, tools);
        
        let mut messages = conversation.to_vec();
        
        // Build the user turn with multimodal content
        let mut content_parts = Vec::new();
        
        // 1. Structured sensor state as text
        let sensor_text = sensor_state.format_for_model();
        content_parts.push(ContentPart::Text {
            text: sensor_text,
        });
        
        // 2. Sensor visualization image (if spatial sensors present)
        if let Some(viz_bytes) = sensor_viz {
            let base64 = base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                viz_bytes,
            );
            content_parts.push(ContentPart::Image {
                data: base64,
                media_type: "image/png".to_string(),
            });
        }
        
        // 3. Camera frame (if camera component present)
        if let Some(frame_bytes) = camera_frame {
            let base64 = base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                frame_bytes,
            );
            content_parts.push(ContentPart::Image {
                data: base64,
                media_type: "image/jpeg".to_string(),
            });
        }
        
        // 4. User message
        content_parts.push(ContentPart::Text {
            text: user_message.to_string(),
        });
        
        messages.push(ChatMessage {
            role: "user".to_string(),
            content: MessageContent::Multimodal(content_parts),
        });
        
        CompletionRequest {
            messages,
            system_prompt: Some(system_prompt),
            temperature: Some(0.3),
            max_tokens: Some(1024),
            tools: Some(tools.to_vec()),
        }
    }
}
```

### Runtime system prompt

```
You are controlling a robot through serial commands. You observe sensor data and camera images, decide what actions to take, and call the available tools to control the hardware.

Hardware: {manifest_summary}

Rules:
- Always check sensor data before and after actions
- If any sensor indicates danger (obstacle too close, tilt too steep), call stop immediately
- Explain what you observe and why you're taking each action
- If you're unsure about a sensor reading, call read_sensor_state to get a fresh reading
- Never move without checking distance sensors first

You can call multiple tools in sequence. After each tool call, you'll receive updated sensor data.
```

## Step 3: Tool Call Dispatcher

When the runtime model returns tool calls, the dispatcher translates them into serial commands:

```rust
// src-tauri/src/services/tool_dispatch.rs

pub struct ToolDispatcher {
    tools: Vec<ToolDefinition>,
    serial: Arc<SerialManager>,
}

impl ToolDispatcher {
    pub async fn execute(&self, tool_call: &ToolCall) -> Result<ToolResult, String> {
        match tool_call.name.as_str() {
            // Lifecycle tools — handled by the app, not serial
            "read_sensor_state" => self.handle_read_sensor_state().await,
            "wait_milliseconds" => self.handle_wait(tool_call).await,
            "get_camera_frame" => self.handle_get_camera_frame().await,
            "end_session" => self.handle_end_session().await,
            
            // Domain tools — translated to serial commands
            _ => self.handle_serial_tool(tool_call).await,
        }
    }
    
    async fn handle_serial_tool(&self, tool_call: &ToolCall) -> Result<ToolResult, String> {
        // Find the tool definition
        let tool_def = self.tools.iter()
            .find(|t| t.name == tool_call.name)
            .ok_or_else(|| format!("Unknown tool: {}", tool_call.name))?;
        
        // Build the CMD string from the tool's serial_command template
        let cmd = build_serial_command(&tool_def.serial_command, &tool_call.arguments)?;
        
        // Write to serial
        self.serial.send_command(&cmd)?;
        
        // Wait briefly for the board to acknowledge
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        
        Ok(ToolResult {
            tool_name: tool_call.name.clone(),
            success: true,
            output: format!("Sent: {}", cmd),
        })
    }
    
    async fn handle_wait(&self, tool_call: &ToolCall) -> Result<ToolResult, String> {
        let ms = tool_call.arguments["milliseconds"]
            .as_u64()
            .unwrap_or(1000);
        tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
        Ok(ToolResult {
            tool_name: "wait_milliseconds".to_string(),
            success: true,
            output: format!("Waited {}ms", ms),
        })
    }
    
    async fn handle_end_session(&self) -> Result<ToolResult, String> {
        // Signal the agent loop to terminate
        Ok(ToolResult {
            tool_name: "end_session".to_string(),
            success: true,
            output: "Session ended by model".to_string(),
        })
    }
}

fn build_serial_command(template: &str, arguments: &serde_json::Value) -> Result<String, String> {
    // template: "CMD:move_forward:speed={speed}"
    // arguments: {"speed": 80}
    // result: "CMD:move_forward:speed=80"
    
    let mut cmd = template.to_string();
    if let Some(obj) = arguments.as_object() {
        for (key, value) in obj {
            let placeholder = format!("{{{}}}", key);
            let value_str = match value {
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Bool(b) => b.to_string(),
                _ => value.to_string(),
            };
            cmd = cmd.replace(&placeholder, &value_str);
        }
    }
    Ok(cmd)
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolResult {
    pub tool_name: String,
    pub success: bool,
    pub output: String,
}
```

## Step 4: The Agent Loop

The agent loop is the core runtime cycle. It runs as a background task, orchestrated by the Rust backend:

```rust
// src-tauri/src/services/agent.rs

pub struct AgentLoop {
    runtime_model: Box<dyn ModelProvider>,
    tool_dispatcher: ToolDispatcher,
    context_assembler: ContextAssembler,
    sensor_state: Arc<Mutex<SensorStateStore>>,
    camera: Option<CameraService>,
    sensor_viz: SensorVizRenderer,
    conversation: Vec<ChatMessage>,
    manifest: Manifest,
    tools: Vec<ToolDefinition>,
    running: Arc<AtomicBool>,
    event_tx: mpsc::Sender<AgentEvent>,
}

impl AgentLoop {
    pub async fn run_turn(&mut self, user_message: &str) -> Result<(), String> {
        self.running.store(true, Ordering::SeqCst);
        
        loop {
            if !self.running.load(Ordering::SeqCst) {
                break; // killed by user
            }
            
            // 1. Collect current context
            let sensor_state = self.sensor_state.lock().await;
            let sensor_viz = self.sensor_viz.render(&sensor_state, &self.manifest);
            let camera_frame = if let Some(ref cam) = self.camera {
                cam.capture_frame().await.ok()
            } else {
                None
            };
            
            // 2. Assemble the completion request
            let request = ContextAssembler::assemble(
                &sensor_state,
                sensor_viz.as_deref(),
                camera_frame.as_deref(),
                &self.tools,
                &self.conversation,
                user_message,
                &self.manifest,
            );
            drop(sensor_state);
            
            // 3. Call the runtime model
            let response = self.runtime_model.complete(request).await?;
            
            // 4. Process the response
            if let Some(text) = &response.content.as_str().filter(|s| !s.is_empty()) {
                // Model has a text response — send to chat UI
                self.event_tx.send(AgentEvent::ModelResponse(text.to_string())).await
                    .map_err(|e| e.to_string())?;
                
                self.conversation.push(ChatMessage {
                    role: "assistant".to_string(),
                    content: MessageContent::Text(text.to_string()),
                });
            }
            
            // 5. Execute tool calls if any
            if let Some(tool_calls) = &response.tool_calls {
                if tool_calls.is_empty() {
                    break; // No more tool calls — turn is done
                }
                
                for tool_call in tool_calls {
                    // Notify UI of tool call
                    self.event_tx.send(AgentEvent::ToolCallStarted {
                        tool_name: tool_call.name.clone(),
                        arguments: tool_call.arguments.clone(),
                    }).await.map_err(|e| e.to_string())?;
                    
                    // Execute
                    let result = self.tool_dispatcher.execute(tool_call).await?;
                    
                    // Check for end_session
                    if tool_call.name == "end_session" {
                        self.event_tx.send(AgentEvent::SessionEnded).await
                            .map_err(|e| e.to_string())?;
                        self.running.store(false, Ordering::SeqCst);
                        break;
                    }
                    
                    // Notify UI of result
                    self.event_tx.send(AgentEvent::ToolCallCompleted {
                        tool_name: tool_call.name.clone(),
                        success: result.success,
                        output: result.output.clone(),
                    }).await.map_err(|e| e.to_string())?;
                    
                    // Add tool result to conversation for next iteration
                    self.conversation.push(ChatMessage {
                        role: "tool".to_string(),
                        content: MessageContent::Text(
                            serde_json::to_string(&result).unwrap_or_default()
                        ),
                    });
                }
                
                // Continue the loop — model may want to call more tools
                // after observing the results
                continue;
            }
            
            // No tool calls and model responded with text — turn is done
            break;
        }
        
        Ok(())
    }
    
    pub fn kill(&self) {
        self.running.store(false, Ordering::SeqCst);
        // Send emergency stop to the board
        let _ = self.tool_dispatcher.serial.send_command("CMD:stop");
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "event", content = "data")]
pub enum AgentEvent {
    ModelResponse(String),
    ToolCallStarted { tool_name: String, arguments: serde_json::Value },
    ToolCallCompleted { tool_name: String, success: bool, output: String },
    SessionEnded,
    Error(String),
}
```

### Loop behavior

The agent loop is NOT autonomous by default. It runs one "turn" per user message:

1. User sends a message (e.g., "Move forward until you detect an obstacle")
2. The model observes sensor state + images
3. The model calls tool(s) (e.g., `move_forward`)
4. The board executes, sensor state updates
5. The model observes the new state
6. If the model calls more tools, loop back to step 4
7. When the model responds with text only (no tool calls), the turn is done
8. The user can send another message for the next turn

The model may iterate multiple times within a single turn — calling tools, observing results, calling more tools. This is the "agentic" part. But a new user message is required to start a new turn. The model does not autonomously decide to keep acting after finishing a turn.

## Step 5: Tauri Commands for Runtime

```rust
#[tauri::command]
pub async fn runtime_send_message(
    state: tauri::State<'_, AppState>,
    message: String,
    on_event: Channel<AgentEvent>,
) -> Result<(), String> {
    // Get the agent loop from state
    // Run a turn with the user's message
    // Stream AgentEvents to the frontend via Channel
}

#[tauri::command]
pub async fn runtime_kill(
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    // Kill the agent loop
    // Send CMD:stop to the board
    // Close serial connection
    // Close the runtime window
}
```

## Step 6: Runtime Chat UI

The left side of the runtime window is a chat interface similar to the code chat (Phase 4) but with additional elements:

### Message types

- **User messages:** Glass Strong with purple tint, right-aligned
- **Model responses:** Glass Standard, left-aligned. The model explains what it observes and why it's acting.
- **Tool call pills:** Inline capsule-shaped indicators showing each tool call:

```
┌───────────────────────────────────────┐
│  I can see clear space ahead. Moving  │
│  forward at 60% speed.                │
│                                       │
│  ┌─ ◉ move_forward speed=60 ──────┐  │
│  └─────────────────────────────────┘  │
│                                       │
│  Obstacle detected at 12cm. Stopping. │
│                                       │
│  ┌─ ◉ stop ───────────────────────┐  │
│  └─────────────────────────────────┘  │
│                                       │
│  I've stopped. The front distance     │
│  sensor reads 12cm. Should I turn     │
│  and find an alternate path?          │
└───────────────────────────────────────┘
```

Tool call pill styling:
- Cyan-tinted glass for serial write commands
- Green border + checkmark for successful execution
- Red border + X for failed execution
- Shows: tool name + key parameters
- Brief pulse animation on creation (1s ease-out)

### Input capsule

Same design as the code chat input but with the Kill button adjacent:
- Input capsule: Glass Standard, placeholder "Tell the robot what to do..."
- Kill button: large, red-tinted glass, always visible, always enabled. Text: "KILL" or a stop icon.
- Keyboard shortcut: Escape triggers kill

While the agent loop is running (executing a turn), the input is disabled and shows a pulsing cyan border.

### Loading/running state

While the model is thinking:
- A small pulsing dot appears in the chat area
- The input capsule border pulses in cyan
- The Kill button remains active

## Step 7: Kill Button Implementation

The Kill button is the most important safety control. It must be always reachable, always functional, and always fast.

```rust
// Kill is NOT an async operation — it must complete immediately
pub fn kill_runtime(state: &AppState) {
    // 1. Set the running flag to false (stops the agent loop)
    if let Some(ref agent) = state.agent_loop {
        agent.kill();
    }
    
    // 2. Send CMD:stop to the board (synchronous serial write)
    if let Some(ref serial) = state.serial_manager {
        serial.send_command("CMD:stop").ok(); // ignore errors — best effort
    }
    
    // 3. Close serial connection
    if let Some(ref serial) = state.serial_manager {
        serial.stop();
    }
}
```

**Escape key binding:** Register a global keyboard shortcut in the runtime window:

```rust
// In the runtime window setup
runtime_window.on_window_event(move |event| {
    if let tauri::WindowEvent::KeyboardInput { event, .. } = event {
        if event.physical_key == PhysicalKey::Code(KeyCode::Escape) {
            kill_runtime(&state);
        }
    }
});
```

Also bind it on the frontend side as a backup:

```typescript
useEffect(() => {
  const handler = (e: KeyboardEvent) => {
    if (e.key === "Escape") {
      invoke("runtime_kill");
    }
  };
  window.addEventListener("keydown", handler);
  return () => window.removeEventListener("keydown", handler);
}, []);
```

## Step 8: Connect Flash → Runtime Transition

Update the flash success flow from Phase 5:

After a successful flash:
1. Show "Flashed successfully" for 2 seconds
2. Show a "Start Runtime" button
3. On click, call `open_runtime_window`
4. The runtime window opens with serial connected and sensors streaming
5. The user can type their first message to begin controlling the robot

Do NOT auto-start the runtime. The user explicitly transitions by clicking.

## Step 9: Verify

1. Flash a sketch to a board with sensors and actuators
2. Click "Start Runtime" — the runtime window opens
3. Serial monitor shows live sensor data on the right panel
4. Sensor state panel shows parsed values
5. Type "Move forward slowly" — the model calls `move_forward` with low speed
6. A cyan tool call pill appears in the chat
7. The board moves, sensor values update
8. The model observes new sensor data and responds with text
9. Type "Stop" — the model calls `stop`
10. Press Escape — immediate emergency stop, all motors halt
11. Click the Kill button — same as Escape
12. Close the runtime window — serial connection closes, board gets CMD:stop

### Multi-tool-call verification

Type "Explore the area — move forward, check for obstacles, turn if blocked."
- The model should issue multiple tool calls per turn
- Each tool call shows as a pill in the chat
- The model reads updated sensor data between calls
- The conversation flows naturally with explanations

## Common Issues

**Model doesn't call tools:** Ensure the tools are included in the `CompletionRequest`. Check that the tool definitions match the provider's expected format. Ollama uses a different tool format than OpenAI — the provider trait must translate.

**Agent loop never terminates a turn:** Add a maximum iteration count (e.g., 10 tool calls per turn). If the model keeps calling tools, force-stop and tell the user.

**Sensor data is stale in model context:** The context is assembled at the start of each model call. If the model calls a tool and wants fresh data, it should call `read_sensor_state`. The context assembler uses the latest data at assembly time.

**Camera frames are too large:** Resize JPEG frames to 320×240 before including in context. Large images consume too many tokens.

**Runtime model is text-only:** The settings UI should have warned the user in Phase 2. If a text-only model is used, camera frames and sensor viz are silently dropped. The model still works with structured text sensor data.

## What NOT to Do

- Do not make the agent loop autonomous. It runs turns triggered by user messages. The model does not independently decide to start new turns.
- Do not stream the runtime model's response token-by-token. Use the non-streaming `complete` method so the full response + tool calls arrive together. Streaming makes tool call parsing unreliable.
- Do not let the Kill button be covered, disabled, or inaccessible. It must be visible and functional at all times in the runtime window.
- Do not skip the CMD:stop on kill. Even if the serial connection is broken, attempt the stop command. Best effort.
- Do not keep the serial connection open after the runtime window closes. Clean up everything on window close.
- Do not persist runtime conversation history to disk. It's ephemeral — when the runtime window closes, the conversation is gone.