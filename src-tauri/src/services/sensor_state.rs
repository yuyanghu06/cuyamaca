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
