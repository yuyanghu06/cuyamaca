use crate::models::manifest::Component;
use crate::services::sensor_state::{SensorReading, SensorStateStore, SensorUpdate};
use serialport::SerialPort;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, mpsc};

/// Active serial connection with concurrent reader/writer threads.
pub struct SerialConnection {
    pub port_name: String,
    pub baud_rate: u32,
    raw_tx: broadcast::Sender<String>,
    sensor_update_tx: broadcast::Sender<SensorUpdate>,
    command_tx: mpsc::Sender<String>,
    running: Arc<AtomicBool>,
    sensor_state: Arc<std::sync::RwLock<SensorStateStore>>,
    start_time: Instant,
}

impl SerialConnection {
    /// Open a serial connection and start reader/writer threads.
    pub fn open(port_name: &str, baud_rate: u32, components: Vec<Component>) -> Result<Self, String> {
        let port = serialport::new(port_name, baud_rate)
            .timeout(Duration::from_millis(100))
            .open()
            .map_err(|e| format!("Failed to open {}: {}", port_name, e))?;

        let write_port = port
            .try_clone()
            .map_err(|e| format!("Failed to clone serial port for writer: {}", e))?;

        let (raw_tx, _) = broadcast::channel(512);
        let (sensor_update_tx, _) = broadcast::channel(256);
        let (command_tx, command_rx) = mpsc::channel(64);
        let running = Arc::new(AtomicBool::new(true));
        let sensor_state = Arc::new(std::sync::RwLock::new(SensorStateStore::new(components)));
        let start_time = Instant::now();

        // Spawn reader thread
        {
            let running = Arc::clone(&running);
            let raw_tx = raw_tx.clone();
            let sensor_update_tx = sensor_update_tx.clone();
            let sensor_state = Arc::clone(&sensor_state);

            std::thread::spawn(move || {
                reader_loop(port, running, raw_tx, sensor_update_tx, sensor_state, start_time);
            });
        }

        // Spawn writer thread
        {
            let running = Arc::clone(&running);
            std::thread::spawn(move || {
                writer_loop(write_port, running, command_rx);
            });
        }

        Ok(SerialConnection {
            port_name: port_name.to_string(),
            baud_rate,
            raw_tx,
            sensor_update_tx,
            command_tx,
            running,
            sensor_state,
            start_time,
        })
    }

    /// Send a command string to the board (e.g., "CMD:stop").
    pub fn send_command(&self, command: &str) -> Result<(), String> {
        self.command_tx
            .try_send(command.to_string())
            .map_err(|e| format!("Failed to queue command: {}", e))
    }

    /// Subscribe to raw serial lines (for serial monitor).
    pub fn subscribe_raw(&self) -> broadcast::Receiver<String> {
        self.raw_tx.subscribe()
    }

    /// Subscribe to parsed sensor updates.
    pub fn subscribe_sensors(&self) -> broadcast::Receiver<SensorUpdate> {
        self.sensor_update_tx.subscribe()
    }

    /// Get a snapshot of the current sensor state.
    pub fn get_sensor_state_snapshot(
        &self,
    ) -> crate::services::sensor_state::SensorStateSnapshot {
        let state = self.sensor_state.read().unwrap();
        state.snapshot()
    }

    /// Get a reference to the sensor state store (for visualization rendering).
    pub fn sensor_state(&self) -> &Arc<std::sync::RwLock<SensorStateStore>> {
        &self.sensor_state
    }

    /// Check if the connection is still running.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    pub fn running_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.running)
    }

    pub fn elapsed_ms(&self) -> u64 {
        self.start_time.elapsed().as_millis() as u64
    }

    /// Stop the connection — sends CMD:stop and signals threads to exit.
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
        let _ = self.command_tx.try_send("CMD:stop".to_string());
    }
}

impl Drop for SerialConnection {
    fn drop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

// ── Reader thread ────────────────────────────────────────

fn reader_loop(
    mut port: Box<dyn SerialPort>,
    running: Arc<AtomicBool>,
    raw_tx: broadcast::Sender<String>,
    sensor_update_tx: broadcast::Sender<SensorUpdate>,
    sensor_state: Arc<std::sync::RwLock<SensorStateStore>>,
    start_time: Instant,
) {
    let mut buffer = String::new();
    let mut byte_buf = [0u8; 1024];

    while running.load(Ordering::Relaxed) {
        match port.read(&mut byte_buf) {
            Ok(n) if n > 0 => {
                buffer.push_str(&String::from_utf8_lossy(&byte_buf[..n]));

                while let Some(newline_pos) = buffer.find('\n') {
                    let line = buffer[..newline_pos]
                        .trim_end_matches('\r')
                        .trim()
                        .to_string();
                    buffer = buffer[newline_pos + 1..].to_string();

                    if line.is_empty() {
                        continue;
                    }

                    // Broadcast raw line to serial monitor subscribers
                    let _ = raw_tx.send(line.clone());

                    // Try to parse as structured sensor output
                    if let Some(reading) = parse_sensor_line(&line, start_time) {
                        let formatted = {
                            let mut state = sensor_state.write().unwrap();
                            state.update_and_format(&reading)
                        };

                        let _ = sensor_update_tx.send(SensorUpdate {
                            sensor_id: reading.sensor_id.clone(),
                            values: reading.values.clone(),
                            formatted,
                        });
                    }
                }
            }
            Ok(_) => {} // no data this cycle
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                // Normal timeout, continue reading
            }
            Err(e) => {
                eprintln!("Serial read error: {}", e);
                running.store(false, Ordering::SeqCst);
                let _ = raw_tx.send(format!("ERROR: Serial disconnected: {}", e));
                break;
            }
        }
    }
}

// ── Writer thread ────────────────────────────────────────

fn writer_loop(
    mut port: Box<dyn SerialPort>,
    running: Arc<AtomicBool>,
    mut command_rx: mpsc::Receiver<String>,
) {
    while running.load(Ordering::Relaxed) {
        match command_rx.try_recv() {
            Ok(cmd) => {
                let cmd = if cmd.ends_with('\n') {
                    cmd
                } else {
                    format!("{}\n", cmd)
                };
                if let Err(e) = port.write_all(cmd.as_bytes()) {
                    eprintln!("Serial write error: {}", e);
                }
                let _ = port.flush();
            }
            Err(mpsc::error::TryRecvError::Empty) => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(mpsc::error::TryRecvError::Disconnected) => break,
        }
    }
}

// ── Parser ───────────────────────────────────────────────

/// Parse a line matching the `SENSOR_ID:VALUE` protocol.
fn parse_sensor_line(line: &str, start_time: Instant) -> Option<SensorReading> {
    let colon_pos = line.find(':')?;
    let sensor_id = line[..colon_pos].trim().to_string();
    let value_str = line[colon_pos + 1..].trim().to_string();

    // Skip CMD echo lines and ERROR lines
    if sensor_id == "CMD" || sensor_id == "ERROR" || sensor_id == "DEBUG" {
        return None;
    }

    // Sensor IDs should be uppercase alphanumeric + underscore
    if !sensor_id
        .chars()
        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
    {
        return None;
    }

    let timestamp_ms = start_time.elapsed().as_millis() as u64;

    // Check for binary string first (line sensor array: "00011000")
    // Must be all 0s and 1s with length > 1 (to distinguish from simple "0" or "1")
    if value_str.len() > 1
        && value_str.chars().all(|c| c == '0' || c == '1')
    {
        return Some(SensorReading {
            sensor_id,
            values: vec![],
            raw_value: value_str,
            raw_line: line.to_string(),
            timestamp_ms,
        });
    }

    // Try to parse comma-separated numeric values
    let values: Vec<f64> = value_str
        .split(',')
        .filter_map(|v| v.trim().parse::<f64>().ok())
        .collect();

    // If no numeric values parsed, it could be a string state
    if values.is_empty() {
        // Integer value (bump switches etc.)
        if let Ok(v) = value_str.parse::<i64>() {
            return Some(SensorReading {
                sensor_id,
                values: vec![v as f64],
                raw_value: value_str,
                raw_line: line.to_string(),
                timestamp_ms,
            });
        }

        // String state (e.g., "moving_forward")
        return Some(SensorReading {
            sensor_id,
            values: vec![],
            raw_value: value_str,
            raw_line: line.to_string(),
            timestamp_ms,
        });
    }

    Some(SensorReading {
        sensor_id,
        values,
        raw_value: value_str,
        raw_line: line.to_string(),
        timestamp_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_scalar_sensor() {
        let start = Instant::now();
        let r = parse_sensor_line("DIST_FRONT:14", start).unwrap();
        assert_eq!(r.sensor_id, "DIST_FRONT");
        assert_eq!(r.values, vec![14.0]);
    }

    #[test]
    fn parse_multi_axis() {
        let start = Instant::now();
        let r = parse_sensor_line("ACCEL:0.12,0.03,9.81", start).unwrap();
        assert_eq!(r.sensor_id, "ACCEL");
        assert_eq!(r.values.len(), 3);
        assert!((r.values[2] - 9.81).abs() < 0.001);
    }

    #[test]
    fn parse_binary_string() {
        let start = Instant::now();
        let r = parse_sensor_line("LINE:00011000", start).unwrap();
        assert_eq!(r.sensor_id, "LINE");
        assert!(r.values.is_empty());
        assert_eq!(r.raw_value, "00011000");
    }

    #[test]
    fn parse_string_state() {
        let start = Instant::now();
        let r = parse_sensor_line("STATE:moving_forward", start).unwrap();
        assert_eq!(r.sensor_id, "STATE");
        assert!(r.values.is_empty());
        assert_eq!(r.raw_value, "moving_forward");
    }

    #[test]
    fn skip_cmd_lines() {
        let start = Instant::now();
        assert!(parse_sensor_line("CMD:stop", start).is_none());
    }

    #[test]
    fn skip_non_structured() {
        let start = Instant::now();
        // Lowercase sensor IDs are rejected
        assert!(parse_sensor_line("debug:test", start).is_none());
        // No colon
        assert!(parse_sensor_line("just some text", start).is_none());
    }
}
