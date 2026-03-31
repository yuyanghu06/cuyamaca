use crate::models::manifest::Manifest;
use crate::models::tools::SerialToolDefinition;
use crate::services::provider::{
    ChatMessage, CompletionRequest, ContentPart, MessageContent, ToolDefinition,
};
use crate::services::sensor_state::SensorStateStore;
use base64::Engine;

/// Assemble a multimodal CompletionRequest for one runtime model turn.
pub fn assemble(
    sensor_state: &SensorStateStore,
    sensor_viz_png: Option<&[u8]>,
    camera_frame_jpg: Option<&[u8]>,
    tools: &[SerialToolDefinition],
    conversation: &[ChatMessage],
    user_message: &str,
    manifest: &Manifest,
) -> CompletionRequest {
    let system_prompt = build_system_prompt(manifest, tools);

    let tool_defs = build_tool_definitions(tools);

    let mut messages: Vec<ChatMessage> = conversation.to_vec();

    // Build the user turn with multimodal content
    let mut parts: Vec<ContentPart> = Vec::new();

    // 1. Structured sensor state as text
    let sensor_text = sensor_state.format_for_model();
    if !sensor_text.is_empty() {
        parts.push(ContentPart::Text { text: sensor_text });
    }

    // 2. Sensor visualization PNG
    if let Some(viz) = sensor_viz_png {
        let b64 = base64::engine::general_purpose::STANDARD.encode(viz);
        parts.push(ContentPart::Image {
            data: b64,
            media_type: "image/png".into(),
        });
    }

    // 3. Camera frame JPEG
    if let Some(frame) = camera_frame_jpg {
        let b64 = base64::engine::general_purpose::STANDARD.encode(frame);
        parts.push(ContentPart::Image {
            data: b64,
            media_type: "image/jpeg".into(),
        });
    }

    // 4. User message
    parts.push(ContentPart::Text {
        text: user_message.to_string(),
    });

    messages.push(ChatMessage {
        role: "user".into(),
        content: if parts.len() == 1 {
            // Text-only: use simple string content for broader compatibility
            MessageContent::Text(user_message.to_string())
        } else {
            MessageContent::Multimodal(parts)
        },
    });

    CompletionRequest {
        messages,
        system_prompt: Some(system_prompt),
        temperature: Some(0.3),
        max_tokens: Some(1024),
        tools: if tool_defs.is_empty() {
            None
        } else {
            Some(tool_defs)
        },
    }
}

fn build_system_prompt(manifest: &Manifest, tools: &[SerialToolDefinition]) -> String {
    let mut prompt = String::from(
        "You are controlling a robot through serial commands. \
         You observe sensor data and camera images, decide what actions to take, \
         and call the available tools to control the hardware.\n\n",
    );

    // Manifest summary
    prompt.push_str(&format!("Hardware: {} on {}\n", manifest.project, manifest.board));
    if !manifest.components.is_empty() {
        prompt.push_str("Components:\n");
        for c in &manifest.components {
            prompt.push_str(&format!("- {} ({}) — {}\n", c.id, c.component_type, c.label));
        }
    }

    prompt.push_str("\nAvailable tools:\n");
    for t in tools {
        prompt.push_str(&format!("- {}: {}\n", t.name, t.description));
    }
    prompt.push_str("- read_sensor_state: Get a fresh sensor state reading\n");
    prompt.push_str("- wait_milliseconds: Pause for a given number of milliseconds\n");
    prompt.push_str("- end_session: End the runtime session\n");

    prompt.push_str(
        "\nRules:\n\
         - Always check sensor data before and after actions\n\
         - If any sensor indicates danger (obstacle too close, tilt too steep), call stop immediately\n\
         - Explain what you observe and why you're taking each action\n\
         - If you're unsure about a sensor reading, call read_sensor_state for a fresh reading\n\
         - Never move without checking distance sensors first\n\
         - You can call multiple tools in sequence within a single turn\n",
    );

    prompt
}

fn build_tool_definitions(tools: &[SerialToolDefinition]) -> Vec<ToolDefinition> {
    let mut defs: Vec<ToolDefinition> = tools
        .iter()
        .map(|t| {
            // Convert SerialToolDefinition parameters to JSON Schema for the model
            let mut properties = serde_json::Map::new();
            let mut required = Vec::new();

            for (name, param) in &t.parameters {
                let mut prop = serde_json::Map::new();
                prop.insert(
                    "type".into(),
                    serde_json::Value::String(match param.param_type.as_str() {
                        "integer" => "integer".into(),
                        "number" | "float" => "number".into(),
                        "boolean" | "bool" => "boolean".into(),
                        _ => "string".into(),
                    }),
                );
                if let Some(desc) = &param.range {
                    prop.insert(
                        "description".into(),
                        serde_json::Value::String(format!("Range: {}", desc)),
                    );
                }
                properties.insert(name.clone(), serde_json::Value::Object(prop));
                if param.required {
                    required.push(serde_json::Value::String(name.clone()));
                }
            }

            ToolDefinition {
                name: t.name.clone(),
                description: t.description.clone(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": properties,
                    "required": required
                }),
            }
        })
        .collect();

    // Add lifecycle tools
    defs.push(ToolDefinition {
        name: "read_sensor_state".into(),
        description: "Get the current parsed sensor state as text".into(),
        parameters: serde_json::json!({"type": "object", "properties": {}}),
    });
    defs.push(ToolDefinition {
        name: "wait_milliseconds".into(),
        description: "Pause the agent loop for a specified duration".into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "milliseconds": {"type": "integer", "description": "Duration in ms"}
            },
            "required": ["milliseconds"]
        }),
    });
    defs.push(ToolDefinition {
        name: "end_session".into(),
        description: "End the runtime session and close the runtime window".into(),
        parameters: serde_json::json!({"type": "object", "properties": {}}),
    });

    defs
}
