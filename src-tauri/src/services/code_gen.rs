use crate::models::manifest::Manifest;
use crate::models::tools::{
    DiffLine, DiffStatus, GeneratedSketchResponse, SerialToolDefinition,
};
use crate::services::provider::{ChatMessage, CompletionRequest, MessageContent, ModelProvider};
use similar::{ChangeTag, TextDiff};
use std::sync::OnceLock;

// Prompt templates loaded from src-tauri/prompts/ at startup
static GENERATE_PROMPT: OnceLock<String> = OnceLock::new();
static CHAT_PROMPT: OnceLock<String> = OnceLock::new();
static REVIEW_PROMPT: OnceLock<String> = OnceLock::new();
static RUNTIME_PROMPT: OnceLock<String> = OnceLock::new();

fn load_prompt(name: &str) -> String {
    // Try loading from the prompts directory adjacent to the binary
    let candidates = [
        // Running from src-tauri/target/debug or release
        format!("../../prompts/{}.md", name),
        format!("../prompts/{}.md", name),
        format!("prompts/{}.md", name),
        // Absolute from working dir
        format!("{}/prompts/{}.md", env!("CARGO_MANIFEST_DIR"), name),
    ];
    for path in &candidates {
        if let Ok(text) = std::fs::read_to_string(path) {
            return text;
        }
    }
    // Fallback: embedded at compile time
    match name {
        "generate" => include_str!("../../prompts/generate.md").to_string(),
        "chat" => include_str!("../../prompts/chat.md").to_string(),
        "review" => include_str!("../../prompts/review.md").to_string(),
        "runtime" => include_str!("../../prompts/runtime.md").to_string(),
        _ => String::new(),
    }
}

pub fn get_generate_prompt() -> &'static str {
    GENERATE_PROMPT.get_or_init(|| load_prompt("generate"))
}

pub fn get_chat_prompt() -> &'static str {
    CHAT_PROMPT.get_or_init(|| load_prompt("chat"))
}

pub fn get_review_prompt() -> &'static str {
    REVIEW_PROMPT.get_or_init(|| load_prompt("review"))
}

pub fn get_runtime_prompt() -> &'static str {
    RUNTIME_PROMPT.get_or_init(|| load_prompt("runtime"))
}

pub struct CodeGenService;

impl CodeGenService {
    pub async fn generate_sketch(
        provider: &dyn ModelProvider,
        manifest: &Manifest,
    ) -> Result<GeneratedSketchResponse, String> {
        let system_prompt = build_system_prompt(manifest);
        let user_prompt =
            "Generate a complete Arduino sketch for this hardware configuration. Include all sensor reading, actuator control, and the serial command dispatch loop.".to_string();

        let request = CompletionRequest {
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: MessageContent::Text(user_prompt),
            }],
            system_prompt: Some(system_prompt),
            temperature: Some(0.2),
            max_tokens: Some(4096),
            tools: None,
        };

        let response = provider.complete(request).await?;
        let sketch = extract_code_block(&response.content);

        Ok(GeneratedSketchResponse {
            code: sketch,
            diff: None,
        })
    }

    pub async fn modify_sketch(
        provider: &dyn ModelProvider,
        manifest: &Manifest,
        current_sketch: &str,
        instruction: &str,
        conversation_history: &[ChatMessage],
    ) -> Result<GeneratedSketchResponse, String> {
        let system_prompt = build_system_prompt(manifest);

        let mut messages: Vec<ChatMessage> = conversation_history.to_vec();

        let user_content = format!(
            "Here is the current sketch:\n```cpp\n{}\n```\n\nModify it to: {}\n\nReturn the complete modified sketch. Do not omit any existing functionality unless explicitly asked to remove it.",
            current_sketch, instruction
        );

        messages.push(ChatMessage {
            role: "user".to_string(),
            content: MessageContent::Text(user_content),
        });

        let request = CompletionRequest {
            messages,
            system_prompt: Some(system_prompt),
            temperature: Some(0.2),
            max_tokens: Some(4096),
            tools: None,
        };

        let response = provider.complete(request).await?;
        let new_sketch = extract_code_block(&response.content);
        let diff = compute_diff(current_sketch, &new_sketch);

        Ok(GeneratedSketchResponse {
            code: new_sketch,
            diff: Some(diff),
        })
    }

    pub async fn synthesize_tools(
        provider: &dyn ModelProvider,
        manifest: &Manifest,
        sketch: &str,
    ) -> Result<Vec<SerialToolDefinition>, String> {
        let system_prompt = "You are a tool definition synthesizer. You read Arduino sketches and produce JSON tool definitions.".to_string();

        let user_prompt = format!(
            r#"Read this Arduino sketch and produce a JSON array of tool definitions.

Each tool represents a serial command the sketch can receive. For each dispatchable command in the CMD: handler, create a tool with:
- name: snake_case matching the function name
- description: plain English explanation of what this tool does, written for someone who has never seen the sketch
- parameters: object mapping parameter names to {{"type": "...", "range": "...", "default": ..., "required": true/false}}
- serial_command: the exact CMD string template with {{param}} placeholders

The hardware manifest for context:
```json
{}
```

The sketch:
```cpp
{}
```

Respond with ONLY the JSON array, no explanation."#,
            serde_json::to_string_pretty(manifest).unwrap_or_default(),
            sketch
        );

        let request = CompletionRequest {
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: MessageContent::Text(user_prompt),
            }],
            system_prompt: Some(system_prompt),
            temperature: Some(0.1),
            max_tokens: Some(2048),
            tools: None,
        };

        let response = provider.complete(request).await?;
        let json_str = extract_json_array(&response.content);
        let tools: Vec<SerialToolDefinition> = serde_json::from_str(&json_str)
            .map_err(|e| format!("Failed to parse tool definitions: {}. Raw: {}", e, json_str))?;

        Ok(tools)
    }

    pub async fn chat_modify(
        provider: &dyn ModelProvider,
        manifest: &Manifest,
        current_sketch: Option<&str>,
        messages: &[ChatMessage],
    ) -> Result<(String, Option<GeneratedSketchResponse>), String> {
        let system_prompt = build_chat_system_prompt(manifest, current_sketch);

        let request = CompletionRequest {
            messages: messages.to_vec(),
            system_prompt: Some(system_prompt),
            temperature: Some(0.3),
            max_tokens: Some(4096),
            tools: None,
        };

        let response = provider.complete(request).await?;

        // Check if the response contains a code block (sketch modification)
        if response.content.contains("```") && has_sketch_code(&response.content) {
            let new_sketch = extract_code_block(&response.content);
            let diff = current_sketch.map(|old| compute_diff(old, &new_sketch));

            let sketch_response = GeneratedSketchResponse {
                code: new_sketch,
                diff,
            };

            // Extract any text outside the code block as the chat message
            let text = extract_text_outside_code(&response.content);
            Ok((text, Some(sketch_response)))
        } else {
            Ok((response.content, None))
        }
    }
}

fn build_system_prompt(manifest: &Manifest) -> String {
    let manifest_json = serde_json::to_string_pretty(manifest).unwrap_or_default();
    let pin_summary = build_pin_summary(manifest);
    let template = get_generate_prompt();
    template
        .replace("{baud}", &manifest.baud_rate.to_string())
        .replace("{manifest}", &manifest_json)
        .replace("{pins}", &pin_summary)
}

fn build_chat_system_prompt(manifest: &Manifest, current_sketch: Option<&str>) -> String {
    build_chat_system_prompt_pub(manifest, current_sketch)
}

pub fn build_chat_system_prompt_pub(manifest: &Manifest, current_sketch: Option<&str>) -> String {
    let manifest_json = serde_json::to_string_pretty(manifest).unwrap_or_default();
    let pin_summary = build_pin_summary(manifest);
    let sketch_section = match current_sketch {
        Some(sketch) => format!(
            "The current sketch:\n```cpp\n{}\n```\n\nWhen the user asks you to modify the sketch, return the COMPLETE modified sketch in a ```cpp code fence. You may also include explanation text outside the code fence. Do not omit any existing functionality unless explicitly asked to remove it.",
            sketch
        ),
        None => "No sketch exists yet. If the user asks you to generate one, return the complete sketch in a ```cpp code fence.".to_string(),
    };
    let template = get_chat_prompt();
    template
        .replace("{baud}", &manifest.baud_rate.to_string())
        .replace("{manifest}", &manifest_json)
        .replace("{pins}", &pin_summary)
        .replace("{sketch_section}", &sketch_section)
}

fn build_pin_summary(manifest: &Manifest) -> String {
    let mut lines = Vec::new();
    for comp in &manifest.components {
        let pins: Vec<String> = comp
            .pins
            .iter()
            .map(|(name, pin)| format!("{}: pin {}", name, pin))
            .collect();
        let pin_str = if pins.is_empty() {
            comp.connection.clone().unwrap_or_else(|| "no pins".to_string())
        } else {
            pins.join(", ")
        };
        lines.push(format!(
            "- {} ({}, {}): {}",
            comp.label, comp.id, comp.component_type, pin_str
        ));
    }
    lines.join("\n")
}

fn extract_code_block(response: &str) -> String {
    extract_code_block_pub(response)
}

pub fn extract_code_block_pub(response: &str) -> String {
    // Try to find ```cpp, ```arduino, ```ino, or plain ``` blocks
    let patterns = ["```cpp", "```arduino", "```ino", "```c", "```"];

    for pattern in &patterns {
        if let Some(start_idx) = response.find(pattern) {
            let code_start = start_idx + pattern.len();
            // Skip to next line after the opening fence
            let code_start = response[code_start..]
                .find('\n')
                .map(|i| code_start + i + 1)
                .unwrap_or(code_start);

            if let Some(end_idx) = response[code_start..].find("```") {
                return response[code_start..code_start + end_idx]
                    .trim()
                    .to_string();
            }
        }
    }

    // Fallback: treat entire response as code
    response.trim().to_string()
}

fn extract_json_array(response: &str) -> String {
    // Try to find JSON inside code fences first
    let stripped = extract_from_code_fence(response);

    // Find the outermost [ ... ]
    if let Some(start) = stripped.find('[') {
        let mut depth = 0;
        for (i, ch) in stripped[start..].chars().enumerate() {
            match ch {
                '[' => depth += 1,
                ']' => {
                    depth -= 1;
                    if depth == 0 {
                        return stripped[start..start + i + 1].to_string();
                    }
                }
                _ => {}
            }
        }
    }

    stripped
}

fn extract_from_code_fence(response: &str) -> String {
    let patterns = ["```json", "```"];
    for pattern in &patterns {
        if let Some(start_idx) = response.find(pattern) {
            let code_start = start_idx + pattern.len();
            let code_start = response[code_start..]
                .find('\n')
                .map(|i| code_start + i + 1)
                .unwrap_or(code_start);

            if let Some(end_idx) = response[code_start..].find("```") {
                return response[code_start..code_start + end_idx]
                    .trim()
                    .to_string();
            }
        }
    }
    response.to_string()
}

pub fn has_sketch_code(response: &str) -> bool {
    let code = extract_code_block(response);
    // Check for typical Arduino code markers
    code.contains("void setup()") || code.contains("void loop()") || code.contains("Serial.")
}

pub fn extract_text_outside_code(response: &str) -> String {
    let mut result = String::new();
    let mut in_code = false;

    for line in response.lines() {
        if line.starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if !in_code {
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str(line);
        }
    }

    result.trim().to_string()
}

pub fn compute_diff(old: &str, new: &str) -> Vec<DiffLine> {
    let diff = TextDiff::from_lines(old, new);
    let mut lines = Vec::new();
    let mut new_line_num: usize = 0;
    let mut old_line_num: usize = 0;

    for change in diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Equal => {
                old_line_num += 1;
                new_line_num += 1;
                lines.push(DiffLine {
                    line_number: new_line_num,
                    content: change.value().trim_end_matches('\n').to_string(),
                    status: DiffStatus::Unchanged,
                });
            }
            ChangeTag::Delete => {
                old_line_num += 1;
                lines.push(DiffLine {
                    line_number: old_line_num,
                    content: change.value().trim_end_matches('\n').to_string(),
                    status: DiffStatus::Removed,
                });
            }
            ChangeTag::Insert => {
                new_line_num += 1;
                lines.push(DiffLine {
                    line_number: new_line_num,
                    content: change.value().trim_end_matches('\n').to_string(),
                    status: DiffStatus::Added,
                });
            }
        }
    }

    lines
}

pub fn save_sketch_version(project_path: &std::path::Path, sketch: &str) -> Result<u32, String> {
    let history_dir = project_path.join("history");
    std::fs::create_dir_all(&history_dir)
        .map_err(|e| format!("Failed to create history directory: {}", e))?;

    let mut version = 0u32;
    loop {
        version += 1;
        let filename = format!("sketch_v{}.ino", version);
        if !history_dir.join(&filename).exists() {
            break;
        }
    }

    let filename = format!("sketch_v{}.ino", version);
    std::fs::write(history_dir.join(&filename), sketch)
        .map_err(|e| format!("Failed to save sketch version: {}", e))?;

    Ok(version)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── extract_code_block_pub ──

    #[test]
    fn test_extract_cpp_code_block() {
        let input = "Here is your code:\n```cpp\nvoid setup() {}\nvoid loop() {}\n```\nDone.";
        assert_eq!(
            extract_code_block_pub(input),
            "void setup() {}\nvoid loop() {}"
        );
    }

    #[test]
    fn test_extract_arduino_code_block() {
        let input = "```arduino\nSerial.begin(9600);\n```";
        assert_eq!(extract_code_block_pub(input), "Serial.begin(9600);");
    }

    #[test]
    fn test_extract_ino_code_block() {
        let input = "```ino\nint x = 5;\n```";
        assert_eq!(extract_code_block_pub(input), "int x = 5;");
    }

    #[test]
    fn test_extract_plain_code_block() {
        let input = "```\nplain code\n```";
        assert_eq!(extract_code_block_pub(input), "plain code");
    }

    #[test]
    fn test_extract_no_fence_returns_entire() {
        let input = "void setup() { }";
        assert_eq!(extract_code_block_pub(input), "void setup() { }");
    }

    // ── extract_json_array ──

    #[test]
    fn test_extract_json_array_from_fence() {
        let input = "```json\n[{\"name\": \"stop\"}]\n```";
        let result = extract_json_array(input);
        assert_eq!(result, "[{\"name\": \"stop\"}]");
    }

    #[test]
    fn test_extract_json_array_nested() {
        let input = "[{\"params\": [{\"type\": \"int\"}]}]";
        let result = extract_json_array(input);
        assert_eq!(result, "[{\"params\": [{\"type\": \"int\"}]}]");
    }

    #[test]
    fn test_extract_json_array_with_surrounding_text() {
        let input = "Here are the tools:\n[{\"name\": \"move\"}]\nThat's all.";
        let result = extract_json_array(input);
        assert_eq!(result, "[{\"name\": \"move\"}]");
    }

    // ── has_sketch_code ──

    #[test]
    fn test_has_sketch_code_with_setup() {
        assert!(has_sketch_code("```cpp\nvoid setup() { }\n```"));
    }

    #[test]
    fn test_has_sketch_code_with_loop() {
        assert!(has_sketch_code("```cpp\nvoid loop() { }\n```"));
    }

    #[test]
    fn test_has_sketch_code_with_serial() {
        assert!(has_sketch_code("```cpp\nSerial.begin(9600);\n```"));
    }

    #[test]
    fn test_no_sketch_code_plain_text() {
        assert!(!has_sketch_code("This is just a description."));
    }

    // ── extract_text_outside_code ──

    #[test]
    fn test_extract_text_outside_code() {
        let input = "Before\n```\ncode here\n```\nAfter";
        assert_eq!(extract_text_outside_code(input), "Before\nAfter");
    }

    #[test]
    fn test_extract_text_only_text() {
        let input = "Just text";
        assert_eq!(extract_text_outside_code(input), "Just text");
    }

    #[test]
    fn test_extract_text_between_blocks() {
        let input = "```\nblock1\n```\nMiddle\n```\nblock2\n```";
        assert_eq!(extract_text_outside_code(input), "Middle");
    }

    // ── compute_diff ──

    #[test]
    fn test_diff_identical() {
        let lines = compute_diff("a\nb\nc\n", "a\nb\nc\n");
        assert!(lines.iter().all(|l| l.status == DiffStatus::Unchanged));
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn test_diff_all_added() {
        let lines = compute_diff("", "a\nb\n");
        assert!(lines.iter().all(|l| l.status == DiffStatus::Added));
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn test_diff_all_removed() {
        let lines = compute_diff("a\nb\n", "");
        assert!(lines.iter().all(|l| l.status == DiffStatus::Removed));
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn test_diff_mixed_changes() {
        let old = "line1\nline2\nline3\n";
        let new = "line1\nmodified\nline3\n";
        let lines = compute_diff(old, new);
        // line1 unchanged, line2 removed, modified added, line3 unchanged
        let added = lines.iter().filter(|l| l.status == DiffStatus::Added).count();
        let removed = lines.iter().filter(|l| l.status == DiffStatus::Removed).count();
        let unchanged = lines.iter().filter(|l| l.status == DiffStatus::Unchanged).count();
        assert!(added >= 1);
        assert!(removed >= 1);
        assert!(unchanged >= 1);
    }

    #[test]
    fn test_diff_content_correct() {
        let old = "hello\n";
        let new = "hello\nworld\n";
        let lines = compute_diff(old, new);
        let added_line = lines.iter().find(|l| l.status == DiffStatus::Added).unwrap();
        assert_eq!(added_line.content, "world");
    }
}
