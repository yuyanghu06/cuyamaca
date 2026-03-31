use crate::models::manifest::Component;
use serde::Serialize;
use std::collections::{HashMap, VecDeque};

#[derive(Debug, Clone)]
pub struct SensorReading {
    pub sensor_id: String,
    pub values: Vec<f64>,
    pub raw_value: String,
    pub raw_line: String,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SensorUpdate {
    pub sensor_id: String,
    pub values: Vec<f64>,
    pub formatted: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SensorSnapshot {
    pub sensor_id: String,
    pub label: String,
    pub component_type: String,
    pub values: Vec<f64>,
    pub formatted: String,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SensorStateSnapshot {
    pub sensors: Vec<SensorSnapshot>,
    pub formatted_text: String,
}

pub struct SensorStateStore {
    latest: HashMap<String, SensorReading>,
    history: HashMap<String, VecDeque<SensorReading>>,
    components: Vec<Component>,
}

impl SensorStateStore {
    pub fn new(components: Vec<Component>) -> Self {
        Self {
            latest: HashMap::new(),
            history: HashMap::new(),
            components,
        }
    }

    /// Update state with a new reading and return the formatted value string.
    pub fn update_and_format(&mut self, reading: &SensorReading) -> String {
        let history = self.history.entry(reading.sensor_id.clone()).or_default();
        history.push_back(reading.clone());
        if history.len() > 20 {
            history.pop_front();
        }

        self.latest.insert(reading.sensor_id.clone(), reading.clone());

        let component = self.components.iter().find(|c| c.id == reading.sensor_id);
        Self::format_reading(component, reading)
    }

    pub fn snapshot(&self) -> SensorStateSnapshot {
        let sensors: Vec<SensorSnapshot> = self
            .components
            .iter()
            .filter_map(|comp| {
                let reading = self.latest.get(&comp.id)?;
                Some(SensorSnapshot {
                    sensor_id: comp.id.clone(),
                    label: comp.label.clone(),
                    component_type: comp.component_type.clone(),
                    values: reading.values.clone(),
                    formatted: Self::format_reading(Some(comp), reading),
                    timestamp_ms: reading.timestamp_ms,
                })
            })
            .collect();

        let formatted_text = self.format_for_model();

        SensorStateSnapshot {
            sensors,
            formatted_text,
        }
    }

    /// Format the current state as a text block for model context.
    pub fn format_for_model(&self) -> String {
        let mut output = String::from("sensor state (sampled at 100ms intervals):\n");

        for component in &self.components {
            if let Some(reading) = self.latest.get(&component.id) {
                let formatted = Self::format_reading(Some(component), reading);
                output.push_str(&format!(
                    "- {} ({}): {}\n",
                    component.label, component.id, formatted
                ));
            }
        }

        output
    }

    /// Get history for a specific sensor (used by visualization renderer).
    pub fn get_history(&self, sensor_id: &str) -> Option<&VecDeque<SensorReading>> {
        self.history.get(sensor_id)
    }

    /// Get latest reading for a sensor.
    pub fn get_latest(&self, sensor_id: &str) -> Option<&SensorReading> {
        self.latest.get(sensor_id)
    }

    pub fn components(&self) -> &[Component] {
        &self.components
    }

    fn format_reading(component: Option<&Component>, reading: &SensorReading) -> String {
        let comp_type = component.map(|c| c.component_type.as_str()).unwrap_or("");

        match comp_type {
            "ultrasonic" | "ir_distance" => {
                format!("{}cm", reading.values.first().unwrap_or(&0.0))
            }
            "bump_switch" => {
                if reading.values.first() == Some(&1.0) {
                    "triggered".into()
                } else {
                    "clear".into()
                }
            }
            "encoder" => {
                format!("{} ticks", *reading.values.first().unwrap_or(&0.0) as i64)
            }
            "imu" => {
                format!(
                    "x={:.2} y={:.2} z={:.2}",
                    reading.values.first().unwrap_or(&0.0),
                    reading.values.get(1).unwrap_or(&0.0),
                    reading.values.get(2).unwrap_or(&0.0)
                )
            }
            "magnetometer" => {
                format!("{:.1}°", reading.values.first().unwrap_or(&0.0))
            }
            "temp_humidity" => {
                format!("{:.1}°C", reading.values.first().unwrap_or(&0.0))
            }
            "barometer" => {
                format!("{:.1} hPa", reading.values.first().unwrap_or(&0.0))
            }
            "light" => {
                format!("{} lux", *reading.values.first().unwrap_or(&0.0) as i64)
            }
            "servo" => {
                format!("{}°", *reading.values.first().unwrap_or(&0.0) as i64)
            }
            "line_sensor_array" => reading.raw_value.clone(),
            "lidar_serial" => {
                if reading.values.len() >= 2 {
                    format!(
                        "{}cm (strength: {})",
                        reading.values[0] as i64, reading.values[1] as i64
                    )
                } else {
                    format!("{}cm", reading.values.first().unwrap_or(&0.0))
                }
            }
            _ => {
                if reading.values.is_empty() {
                    reading.raw_value.clone()
                } else if reading.values.len() == 1 {
                    format!("{}", reading.values[0])
                } else {
                    reading
                        .values
                        .iter()
                        .map(|v| format!("{:.2}", v))
                        .collect::<Vec<_>>()
                        .join(", ")
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_component(id: &str, comp_type: &str) -> Component {
        Component {
            id: id.to_string(),
            component_type: comp_type.to_string(),
            pins: HashMap::new(),
            label: id.to_string(),
            subtype: None,
            connection: None,
            resolution: None,
            format: None,
        }
    }

    fn make_reading(id: &str, values: Vec<f64>, raw: &str) -> SensorReading {
        SensorReading {
            sensor_id: id.to_string(),
            values,
            raw_value: raw.to_string(),
            raw_line: format!("{}:{}", id, raw),
            timestamp_ms: 1000,
        }
    }

    #[test]
    fn test_format_ultrasonic() {
        let comp = make_component("dist", "ultrasonic");
        let reading = make_reading("dist", vec![14.0], "14");
        assert_eq!(SensorStateStore::format_reading(Some(&comp), &reading), "14cm");
    }

    #[test]
    fn test_format_bump_switch_triggered() {
        let comp = make_component("bump", "bump_switch");
        let reading = make_reading("bump", vec![1.0], "1");
        assert_eq!(SensorStateStore::format_reading(Some(&comp), &reading), "triggered");
    }

    #[test]
    fn test_format_bump_switch_clear() {
        let comp = make_component("bump", "bump_switch");
        let reading = make_reading("bump", vec![0.0], "0");
        assert_eq!(SensorStateStore::format_reading(Some(&comp), &reading), "clear");
    }

    #[test]
    fn test_format_encoder() {
        let comp = make_component("enc", "encoder");
        let reading = make_reading("enc", vec![1842.0], "1842");
        assert_eq!(SensorStateStore::format_reading(Some(&comp), &reading), "1842 ticks");
    }

    #[test]
    fn test_format_imu() {
        let comp = make_component("accel", "imu");
        let reading = make_reading("accel", vec![0.12, 0.03, 9.81], "0.12,0.03,9.81");
        let result = SensorStateStore::format_reading(Some(&comp), &reading);
        assert!(result.contains("x=0.12"));
        assert!(result.contains("y=0.03"));
        assert!(result.contains("z=9.81"));
    }

    #[test]
    fn test_format_magnetometer() {
        let comp = make_component("heading", "magnetometer");
        let reading = make_reading("heading", vec![247.3], "247.3");
        assert_eq!(SensorStateStore::format_reading(Some(&comp), &reading), "247.3°");
    }

    #[test]
    fn test_format_temp_humidity() {
        let comp = make_component("temp", "temp_humidity");
        let reading = make_reading("temp", vec![22.4], "22.4");
        assert_eq!(SensorStateStore::format_reading(Some(&comp), &reading), "22.4°C");
    }

    #[test]
    fn test_format_barometer() {
        let comp = make_component("pres", "barometer");
        let reading = make_reading("pres", vec![1013.2], "1013.2");
        assert_eq!(SensorStateStore::format_reading(Some(&comp), &reading), "1013.2 hPa");
    }

    #[test]
    fn test_format_light() {
        let comp = make_component("lux", "light");
        let reading = make_reading("lux", vec![340.0], "340");
        assert_eq!(SensorStateStore::format_reading(Some(&comp), &reading), "340 lux");
    }

    #[test]
    fn test_format_servo() {
        let comp = make_component("servo", "servo");
        let reading = make_reading("servo", vec![90.0], "90");
        assert_eq!(SensorStateStore::format_reading(Some(&comp), &reading), "90°");
    }

    #[test]
    fn test_format_line_sensor_array() {
        let comp = make_component("line", "line_sensor_array");
        let reading = make_reading("line", vec![], "00011000");
        assert_eq!(SensorStateStore::format_reading(Some(&comp), &reading), "00011000");
    }

    #[test]
    fn test_format_lidar_with_strength() {
        let comp = make_component("lidar", "lidar_serial");
        let reading = make_reading("lidar", vec![142.0, 800.0], "142,800");
        assert_eq!(
            SensorStateStore::format_reading(Some(&comp), &reading),
            "142cm (strength: 800)"
        );
    }

    #[test]
    fn test_format_unknown_type_single_value() {
        let reading = make_reading("mystery", vec![42.0], "42");
        assert_eq!(SensorStateStore::format_reading(None, &reading), "42");
    }

    #[test]
    fn test_format_unknown_type_multi_value() {
        let reading = make_reading("mystery", vec![1.5, 2.5], "1.5,2.5");
        assert_eq!(SensorStateStore::format_reading(None, &reading), "1.50, 2.50");
    }

    #[test]
    fn test_store_update_and_snapshot() {
        let comp = make_component("dist", "ultrasonic");
        let mut store = SensorStateStore::new(vec![comp]);
        let reading = make_reading("dist", vec![25.0], "25");
        let formatted = store.update_and_format(&reading);
        assert_eq!(formatted, "25cm");

        let snapshot = store.snapshot();
        assert_eq!(snapshot.sensors.len(), 1);
        assert_eq!(snapshot.sensors[0].formatted, "25cm");
    }

    #[test]
    fn test_format_for_model() {
        let comp = make_component("dist", "ultrasonic");
        let mut store = SensorStateStore::new(vec![comp]);
        store.update_and_format(&make_reading("dist", vec![14.0], "14"));
        let text = store.format_for_model();
        assert!(text.contains("dist"));
        assert!(text.contains("14cm"));
    }
}