use crate::models::tools::SerialToolDefinition;
use crate::services::provider::ToolCall;
use crate::services::serial::SerialConnection;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ToolResult {
    pub tool_name: String,
    pub success: bool,
    pub output: String,
}

/// Dispatch model tool calls to serial commands or lifecycle handlers.
pub fn execute_serial_tool(
    tool_call: &ToolCall,
    tools: &[SerialToolDefinition],
    serial: &SerialConnection,
) -> Result<ToolResult, String> {
    let tool_def = tools
        .iter()
        .find(|t| t.name == tool_call.name)
        .ok_or_else(|| format!("Unknown tool: {}", tool_call.name))?;

    let cmd = build_serial_command(&tool_def.serial_command, &tool_call.arguments)?;

    serial
        .send_command(&cmd)
        .map_err(|e| format!("Failed to send command: {}", e))?;

    Ok(ToolResult {
        tool_name: tool_call.name.clone(),
        success: true,
        output: format!("Sent: {}", cmd),
    })
}

/// Read current sensor state as a lifecycle tool result.
pub fn handle_read_sensor_state(serial: &SerialConnection) -> ToolResult {
    let snapshot = serial.get_sensor_state_snapshot();
    ToolResult {
        tool_name: "read_sensor_state".into(),
        success: true,
        output: snapshot.formatted_text,
    }
}

/// Build a serial command string from a template + arguments.
/// Template: "CMD:move_forward:speed={speed}"
/// Arguments: {"speed": 80}
/// Result: "CMD:move_forward:speed=80"
fn build_serial_command(
    template: &str,
    arguments: &serde_json::Value,
) -> Result<String, String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_cmd_with_params() {
        let cmd =
            build_serial_command("CMD:move_forward:speed={speed}", &serde_json::json!({"speed": 80}))
                .unwrap();
        assert_eq!(cmd, "CMD:move_forward:speed=80");
    }

    #[test]
    fn build_cmd_multi_params() {
        let cmd = build_serial_command(
            "CMD:turn_left:degrees={degrees},speed={speed}",
            &serde_json::json!({"degrees": 45, "speed": 60}),
        )
        .unwrap();
        assert_eq!(cmd, "CMD:turn_left:degrees=45,speed=60");
    }

    #[test]
    fn build_cmd_no_params() {
        let cmd = build_serial_command("CMD:stop", &serde_json::json!({})).unwrap();
        assert_eq!(cmd, "CMD:stop");
    }
}
