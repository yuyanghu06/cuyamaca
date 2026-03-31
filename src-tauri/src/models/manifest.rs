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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_manifest_new_defaults() {
        let m = Manifest::new("my-robot", "arduino:avr:uno");
        assert_eq!(m.project, "my-robot");
        assert_eq!(m.board, "arduino:avr:uno");
        assert_eq!(m.serial_port, "");
        assert_eq!(m.baud_rate, 115200);
        assert!(m.components.is_empty());
    }

    #[test]
    fn test_manifest_serialize_roundtrip() {
        let mut m = Manifest::new("test", "esp32:esp32:esp32");
        m.serial_port = "/dev/ttyUSB0".into();
        m.baud_rate = 9600;
        m.components.push(Component {
            id: "led1".into(),
            component_type: "led".into(),
            pins: HashMap::from([("signal".into(), 13)]),
            label: "Status LED".into(),
            subtype: None,
            connection: None,
            resolution: None,
            format: None,
        });

        let json = serde_json::to_string(&m).unwrap();
        let deserialized: Manifest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.project, "test");
        assert_eq!(deserialized.baud_rate, 9600);
        assert_eq!(deserialized.components.len(), 1);
        assert_eq!(deserialized.components[0].id, "led1");
    }

    #[test]
    fn test_component_optional_fields_skip_serializing() {
        let comp = Component {
            id: "motor".into(),
            component_type: "dc_motor".into(),
            pins: HashMap::new(),
            label: "Motor".into(),
            subtype: None,
            connection: None,
            resolution: None,
            format: None,
        };
        let json = serde_json::to_string(&comp).unwrap();
        assert!(!json.contains("subtype"));
        assert!(!json.contains("connection"));
        assert!(!json.contains("resolution"));
    }

    #[test]
    fn test_component_with_optional_fields() {
        let comp = Component {
            id: "cam".into(),
            component_type: "camera".into(),
            pins: HashMap::new(),
            label: "Camera".into(),
            subtype: Some("esp32-cam".into()),
            connection: Some("wifi".into()),
            resolution: Some("320x240".into()),
            format: Some("jpeg".into()),
        };
        let json = serde_json::to_string(&comp).unwrap();
        assert!(json.contains("esp32-cam"));
        assert!(json.contains("wifi"));
        assert!(json.contains("320x240"));
        assert!(json.contains("jpeg"));
    }
}
