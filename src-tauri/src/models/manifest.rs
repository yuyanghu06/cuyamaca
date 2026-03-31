use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub project: String,
    pub board: String,
    pub serial_port: String,
    pub baud_rate: u32,
    pub components: Vec<Component>,
}

impl Manifest {
    pub fn new(project: &str, board: &str) -> Self {
        Self {
            project: project.to_string(),
            board: board.to_string(),
            serial_port: String::new(),
            baud_rate: 115200,
            components: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    pub id: String,
    pub component_type: String,
    pub pins: HashMap<String, u8>,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtype: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}
