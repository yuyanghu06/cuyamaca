---
name: cuyamaca-code-generation
description: Build the code generation and diff review workflow for Cuyamaca — the code model generates Arduino sketches from manifests, displays them with highlighted diffs, and synthesizes tool definitions from the sketch. Use this skill whenever the user wants to implement sketch generation, build the code view with diffs, add the approve/reject workflow, implement tool synthesis, add sketch upload support, or references "phase 4", "code generation", "sketch generation", "code view", "diff view", "approve flash", "tool synthesis", "code model integration", or "sketch upload". Also trigger when the user asks about prompting the LLM for Arduino code, structured diff display, or generating tools.json from a sketch. This skill assumes Phase 3 is complete (project system, manifest editor, component library).
---

# Phase 4 — Code Generation + Diff Review

This skill wires up the code model to generate and modify Arduino sketches from the manifest, displays the results in a code view with highlighted diffs, and synthesizes tool definitions from the generated sketch. This is the first phase where the LLM is actively used to produce output.

## What This Skill Produces

- A code generation pipeline: manifest → prompt → code model → sketch + diff
- Sketch upload support: user provides their own `.ino` file
- Code View in the main area: syntax-highlighted sketch with diff overlay
- Approve/reject workflow: user reviews changes before anything is flashed
- Tool synthesis: code model reads the sketch and produces `tools.json`
- Version history: previous sketch versions saved with diffs
- Chat View for the code model: conversational sketch modification

## Prerequisites

- Phase 3 complete: project system with manifest editing
- Phase 2 complete: at least one model provider configured in the code model slot
- A model pulled in Ollama (or an external API key configured) for the code slot

## Step 1: Sketch Generation Prompt Engineering

The code model receives the manifest and a system prompt that encodes all sketch generation rules from the CLAUDE.md. This prompt is critical — it's the contract between the manifest and the generated code.

### System prompt template

Build the system prompt in Rust as a formatted string:

```
You are an Arduino code generator. You produce complete, compilable .ino sketches.

Rules you must follow:
1. Always include Serial.begin({baud_rate}) in setup().
2. Always include a command dispatch loop in loop() that reads Serial.readStringUntil('\n'), parses the CMD: prefix, and dispatches to handler functions.
3. All Serial.print output must follow the structured format: SENSOR_ID:VALUE
4. Print sensor state at fixed intervals using millis(), not delay(). Default interval: 100ms.
5. Always include an emergency stop command (CMD:stop) that halts all actuators immediately.
6. Pin assignments must use #define or const int declarations matching the manifest exactly.
7. Do not include freeform debug strings. All serial output is structured.

The hardware manifest:
{manifest_json}

Component pin reference:
{formatted_pin_summary}

Generate a complete .ino sketch that:
- Initializes all components in setup()
- Reads and prints all sensor values at 100ms intervals in loop()
- Includes a CMD dispatch loop for controlling all actuators
- Includes CMD:stop as an emergency halt
```

### User prompt for generation

```
Generate a complete Arduino sketch for this hardware configuration. Include all sensor reading, actuator control, and the serial command dispatch loop.
```

### User prompt for modification

```
Here is the current sketch:
```
{current_sketch}
```

Modify it to: {user_instruction}

Return the complete modified sketch. Do not omit any existing functionality unless explicitly asked to remove it.
```

## Step 2: Code Model Integration

Create the code generation service:

```rust
// src-tauri/src/services/code_gen.rs

pub struct CodeGenService;

impl CodeGenService {
    pub async fn generate_sketch(
        provider: &dyn ModelProvider,
        manifest: &Manifest,
    ) -> Result<GeneratedSketch, String> {
        let system_prompt = build_system_prompt(manifest);
        let user_prompt = "Generate a complete Arduino sketch for this hardware configuration.";
        
        let request = CompletionRequest {
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: MessageContent::Text(user_prompt.to_string()),
            }],
            system_prompt: Some(system_prompt),
            temperature: Some(0.2),  // low temperature for code generation
            max_tokens: Some(4096),
            tools: None,
        };
        
        let response = provider.complete(request).await?;
        let sketch = extract_code_block(&response.content)?;
        
        Ok(GeneratedSketch {
            code: sketch,
            diff: None,  // no diff for initial generation
        })
    }

    pub async fn modify_sketch(
        provider: &dyn ModelProvider,
        manifest: &Manifest,
        current_sketch: &str,
        instruction: &str,
        conversation_history: &[ChatMessage],
    ) -> Result<GeneratedSketch, String> {
        // Build messages with conversation context
        // Include the current sketch and modification instruction
        // Parse response, extract code, compute diff against current
    }

    pub async fn synthesize_tools(
        provider: &dyn ModelProvider,
        manifest: &Manifest,
        sketch: &str,
    ) -> Result<Vec<ToolDefinition>, String> {
        // Prompt the code model to read the sketch and produce tool definitions
        // The prompt should specify the exact JSON schema from CLAUDE.md
    }
}

pub struct GeneratedSketch {
    pub code: String,
    pub diff: Option<SketchDiff>,
}

pub struct SketchDiff {
    pub added_lines: Vec<usize>,
    pub removed_lines: Vec<(usize, String)>,
    pub unified_diff: String,
}
```

### Code block extraction

The model's response will contain the sketch wrapped in markdown code fences. Extract it:

```rust
fn extract_code_block(response: &str) -> Result<String, String> {
    // Find ```cpp or ```arduino or ```ino or plain ``` blocks
    // Extract the content between the fences
    // If no code fence found, treat the entire response as code (fallback)
}
```

### Diff computation

Use a diff library to compute the difference between the old and new sketch:

```toml
# Cargo.toml
similar = "2"
```

The `similar` crate provides unified diff output. Compute line-level diffs and track which lines were added, removed, or modified.

## Step 3: Sketch Upload Support

Users can upload their own `.ino` file instead of generating from the manifest. The uploaded sketch replaces the current sketch in the project.

```rust
#[tauri::command]
pub async fn upload_sketch(
    state: tauri::State<'_, AppState>,
    sketch_content: String,
) -> Result<(), String> {
    // Set as the active sketch
    // Save to project directory as sketch.ino
    // Trigger tool synthesis on the uploaded sketch
}
```

On the frontend, provide a file picker or drag-and-drop zone in the Code View for `.ino` files. Read the file content in TypeScript and pass it via the Tauri command.

When a sketch is uploaded, the code model should still be used to:
1. Verify it follows the structured serial output convention
2. Add a command dispatch loop if one is missing
3. Synthesize tool definitions

Show the user what changes the model would make as a diff before applying.

## Step 4: Tool Synthesis

After a sketch is generated or uploaded, the code model reads it and produces `tools.json` — the tool definitions that the runtime model will use to control the hardware.

### Tool synthesis prompt

```
Read this Arduino sketch and produce a JSON array of tool definitions.

Each tool represents a serial command the sketch can receive. For each dispatchable command in the CMD: handler, create a tool with:
- name: snake_case matching the function name
- description: plain English explanation of what this tool does, written for someone who has never seen the sketch
- parameters: object mapping parameter names to {type, range, default, required}
- serial_command: the exact CMD string template with {param} placeholders

Also include these lifecycle tools (not serial commands, managed by the app):
- read_sensor_state: returns current parsed sensor values
- wait_milliseconds: pauses for a specified duration
- end_session: terminates the control loop

The sketch:
```
{sketch}
```

Respond with ONLY the JSON array, no explanation.
```

Parse the response as `Vec<ToolDefinition>` and save as `tools.json` in the project directory.

## Step 5: Version History

Every time a sketch is modified (not on initial generation), save the previous version:

```rust
fn save_sketch_version(project_path: &Path, sketch: &str) -> Result<(), String> {
    let history_dir = project_path.join("history");
    std::fs::create_dir_all(&history_dir).map_err(|e| e.to_string())?;
    
    let version = count_versions(&history_dir) + 1;
    let filename = format!("sketch_v{}.ino", version);
    std::fs::write(history_dir.join(filename), sketch).map_err(|e| e.to_string())?;
    
    Ok(())
}
```

## Step 6: Tauri Commands

```rust
#[tauri::command]
pub async fn generate_sketch(
    state: tauri::State<'_, AppState>,
) -> Result<GeneratedSketchResponse, String> {
    // Get the active project's manifest
    // Get the code model provider
    // Call CodeGenService::generate_sketch
    // Return the sketch code (don't save yet — user must approve)
}

#[tauri::command]
pub async fn modify_sketch(
    state: tauri::State<'_, AppState>,
    instruction: String,
) -> Result<GeneratedSketchResponse, String> {
    // Get current sketch + manifest
    // Call CodeGenService::modify_sketch
    // Compute diff against current
    // Return sketch code + diff (don't save yet)
}

#[tauri::command]
pub async fn approve_sketch(
    state: tauri::State<'_, AppState>,
    sketch_code: String,
) -> Result<(), String> {
    // Save current sketch to version history
    // Write new sketch to sketch.ino
    // Run tool synthesis
    // Save tools.json
    // Update project state
}

#[tauri::command]
pub async fn reject_sketch(
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    // Discard the pending sketch, keep the current one
}

#[tauri::command]
pub async fn get_sketch(
    state: tauri::State<'_, AppState>,
) -> Result<Option<String>, String> {
    // Return the current sketch content
}

#[tauri::command]
pub async fn get_tools(
    state: tauri::State<'_, AppState>,
) -> Result<Option<Vec<ToolDefinition>>, String> {
    // Return the current tools.json content
}
```

## Step 7: Build the Code View

Replace the CodeView placeholder with the real code display.

### Layout

```
┌─────────────────────────────────────────┐
│  [Approve & Flash]  [Reject]      │
│  ─── only visible when pending diff ─── │
├─────────────────────────────────────────┤
│                                         │
│  1  #include <Servo.h>                  │
│  2                                      │
│  3  // Pin definitions                  │
│  4+ const int MOTOR_L_PWM = 5;          │  ← added (cyan highlight)
│  5+ const int MOTOR_L_DIR_A = 2;        │  ← added
│  6  const int MOTOR_L_DIR_B = 3;        │
│  7- const int OLD_PIN = 4;              │  ← removed (red highlight)
│  8                                      │
│  ... (scrollable)                       │
│                                         │
└─────────────────────────────────────────┘
```

### Syntax highlighting

Use a lightweight syntax highlighting approach. Options:
- Parse C/C++ keywords and apply CSS classes manually (simplest)
- Use a library like Prism.js loaded from CDN

Apply highlighting classes:
- Keywords (`void`, `int`, `const`, `if`, `while`, `for`, `return`): cyan
- Strings: amber
- Comments: secondary text color
- Numbers: green
- Preprocessor directives (`#include`, `#define`): purple

### Diff overlay

When a pending sketch modification exists, overlay the diff:
- Added lines: left border in cyan, subtle cyan background tint (`var(--cyan-bg)`)
- Removed lines: left border in red, subtle red background tint (`var(--red-bg)`), shown with strikethrough
- Unchanged lines: no special treatment

Line numbers in monospace, dimmed. The code content in monospace at 13px.

### Approve/Reject bar

A floating bar at the top of the Code View, visible only when there's a pending diff:
- "Approve & Flash" button: Glass Standard with green accent. Calls `approve_sketch` and then triggers flashing (Phase 5).
- "Reject" button: Glass Standard with red accent. Calls `reject_sketch` and reverts the display.

When no pending diff exists, the bar is hidden and the code is displayed read-only.

### Empty state

When no sketch exists yet, show:
- "No sketch yet" message
- "Generate from Manifest" button — calls `generate_sketch` using the code model
- "Upload Sketch" button — opens a file picker for `.ino` files
- Both options lead to the same approve/reject flow

## Step 8: Build the Code Chat View

The Chat View (from the sidebar nav) becomes a conversational interface with the code model. This is where users ask for sketch modifications in natural language.

The flow:
1. User types "Add a PID controller for the left motor" in the chat
2. The message is sent to `modify_sketch` with the instruction
3. The code model receives the current sketch + manifest + instruction
4. The response comes back as a modified sketch with diff
5. The Code View auto-switches to show the diff with approve/reject buttons
6. The chat shows a confirmation pill: "Sketch modified — review in Code view"

The chat maintains conversation history so the code model has context from previous exchanges. Store this history in the Rust backend state, scoped to the active project.

### Chat UI

Same layout as Sierra's chat (message bubbles, input capsule) but with the dark industrial styling:
- User messages: Glass Strong with purple tint
- AI messages: Glass Standard
- Code snippets in AI messages: monospace with syntax highlighting
- Input capsule: Glass Standard, placeholder "Describe what to change..."

The chat is NOT streaming for code generation — the full response is shown at once after the model finishes. Show a loading state (pulsing input capsule border + "Generating..." text in a temporary AI bubble) while the code model works.

## Step 9: Verify

1. Configure a code model (Ollama or external) in the code model slot (Phase 2 infrastructure)
2. Create a project with several components in the manifest
3. Navigate to Code View — see the empty state with "Generate from Manifest" button
4. Click "Generate" — loading state appears, then the generated sketch displays with syntax highlighting
5. Review the code — it should follow all sketch generation rules (structured output, CMD dispatch, millis-based timing)
6. Click "Approve & Flash" — the sketch saves to `sketch.ino`, tools.json is generated
7. Navigate to Chat View, type "Add a blink pattern for the LED when stopped"
8. The code model produces a modified sketch, Code View shows the diff
9. Approve or reject the change
10. Upload a custom `.ino` file — the code model analyzes it and suggests modifications if needed
11. Check `tools.json` — it should contain tool definitions matching the sketch's CMD handlers

## Common Issues

**Code model produces incomplete sketches:** Lower the temperature (0.1-0.2) and increase max_tokens. Include explicit instructions to produce the COMPLETE sketch, not just the changed parts.

**Diff is wrong or misaligned:** Use the `similar` crate's line-level diff, not character-level. Normalize line endings before comparing.

**Tool synthesis returns invalid JSON:** The model may wrap JSON in markdown fences or add explanation text. Strip everything outside the JSON array. Validate against the schema before saving.

**Large sketches exceed context window:** For very complex hardware setups, the sketch + manifest + conversation history may exceed the model's context. Add truncation logic that keeps the manifest and most recent sketch in full but trims older conversation turns.

## What NOT to Do

- Do not flash the sketch to the board in this phase. The "Approve & Flash" button saves the sketch but actual flashing is Phase 5.
- Do not let the user edit the sketch directly in the Code View. All modifications go through the code model. The Code View is read-only.
- Do not use streaming for code generation. The full sketch needs to arrive before the diff can be computed. Use the non-streaming `complete` method.
- Do not skip tool synthesis after sketch approval. The `tools.json` must always be in sync with the current sketch.
- Do not send camera frames or sensor data to the code model. The code model is text-only. Multimodal input is for the runtime model in Phase 7.