use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerialToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: HashMap<String, ToolParameter>,
    pub serial_command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameter {
    #[serde(rename = "type")]
    pub param_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRegistry {
    pub tools: Vec<SerialToolDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedSketchResponse {
    pub code: String,
    pub diff: Option<Vec<DiffLine>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffLine {
    pub line_number: usize,
    pub content: String,
    pub status: DiffStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiffStatus {
    Added,
    Removed,
    Unchanged,
}
