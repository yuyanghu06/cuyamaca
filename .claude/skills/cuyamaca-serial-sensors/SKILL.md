---
name: cuyamaca-serial-sensors
description: Build serial communication, structured output parsing, sensor state management, and sensor visualization rendering for Cuyamaca. Use this skill whenever the user wants to implement serial port reading/writing, parse structured sensor output, build the sensor state panel, render sensor visualization images, manage the serial connection lifecycle, or references "phase 6", "serial communication", "serial port", "sensor parsing", "sensor state", "sensor visualization", "structured output", "serial monitor", or "serial reader". Also trigger when the user asks about the SENSOR_ID:VALUE protocol, concurrent serial read/write, sensor image rendering, or real-time state updates. This skill assumes Phase 5 is complete (arduino-cli integration, compile and flash working).
---

# Phase 6 — Serial Communication + Sensor Parsing

This skill builds the serial communication layer: opening a serial connection after flashing, reading structured output, parsing it into typed sensor state, rendering spatial sensor data as visualization images, and writing commands to the board. This is the data pipeline that feeds the runtime model in Phase 7.

## What This Skill Produces

- Serial port connection manager: open, close, reconnect
- Concurrent serial reader/writer (read sensor data while writing commands)
- Structured output parser: `SENSOR_ID:VALUE` lines → typed sensor state
- Sensor state store: latest values for all sensors, updated in real-time
- Serial monitor: raw output streamed to the frontend
- Sensor visualization renderer: PNG images from spatial/array sensor data
- Tauri commands for serial operations
- Frontend components: serial monitor panel, sensor state panel, sensor visualization panel

## Prerequisites

- Phase 5 complete: sketch flashed to a connected board
- A flashed board outputting structured serial data at the configured baud rate
- The `serialport` crate already added in Phase 3

## Step 1: Serial Connection Manager

```rust
// src-tauri/src/services/serial.rs

use serialport::SerialPort;
use std::sync::Arc;
use tokio::sync::{Mutex, broadcast, mpsc};

pub struct SerialManager {
    port: Arc<Mutex<Option<Box<dyn SerialPort>>>>,
    port_name: String,
    baud_rate: u32,
    /// Broadcast channel for raw serial lines → serial monitor UI
    raw_tx: broadcast::Sender<String>,
    /// mpsc channel for parsed sensor updates → sensor state store
    sensor_tx: mpsc::Sender<SensorReading>,
    /// mpsc channel for outgoing commands → serial writer
    command_tx: mpsc::Sender<String>,
    /// Flag to signal the reader/writer loops to stop
    running: Arc<std::sync::atomic::AtomicBool>,
}
```

### Opening the connection

```rust
impl SerialManager {
    pub fn open(port_name: &str, baud_rate: u32) -> Result<Self, String> {
        let port = serialport::new(port_name, baud_rate)
            .timeout(std::time::Duration::from_millis(100))
            .open()
            .map_err(|e| format!("Failed to open {}: {}", port_name, e))?;
        
        let (raw_tx, _) = broadcast::channel(256);
        let (sensor_tx, _sensor_rx) = mpsc::channel(128);
        let (command_tx, _command_rx) = mpsc::channel(64);
        
        let manager = SerialManager {
            port: Arc::new(Mutex::new(Some(port))),
            port_name: port_name.to_string(),
            baud_rate,
            raw_tx,
            sensor_tx,
            command_tx,
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        };
        
        Ok(manager)
    }
}
```

### Concurrent read/write

The serial port must support reading sensor data and writing commands simultaneously. Since `serialport` is synchronous and most serial APIs aren't thread-safe for concurrent read/write on the same handle, use a dedicated thread for reading and the mpsc channel for writing:

```rust
impl SerialManager {
    pub fn start(&self) -> Result<(), String> {
        self.running.store(true, std::sync::atomic::Ordering::SeqCst);
        
        // Clone what the reader thread needs
        let port = Arc::clone(&self.port);
        let raw_tx = self.raw_tx.clone();
        let sensor_tx = self.sensor_tx.clone();
        let running = Arc::clone(&self.running);
        
        // Reader thread — reads from serial, parses, broadcasts
        std::thread::spawn(move || {
            let mut buffer = String::new();
            let mut byte_buf = [0u8; 1024];
            
            while running.load(std::sync::atomic::Ordering::SeqCst) {
                let mut port_guard = port.blocking_lock();
                if let Some(ref mut port) = *port_guard {
                    match port.read(&mut byte_buf) {
                        Ok(n) if n > 0 => {
                            buffer.push_str(&String::from_utf8_lossy(&byte_buf[..n]));
                            // Process complete lines
                            while let Some(newline_pos) = buffer.find('\n') {
                                let line = buffer[..newline_pos].trim().to_string();
                                buffer = buffer[newline_pos + 1..].to_string();
                                
                                if !line.is_empty() {
                                    // Broadcast raw line to serial monitor
                                    let _ = raw_tx.send(line.clone());
                                    
                                    // Parse structured output
                                    if let Some(reading) = parse_sensor_line(&line) {
                                        let _ = sensor_tx.blocking_send(reading);
                                    }
                                }
                            }
                        }
                        Ok(_) => {} // no data
                        Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {} // timeout, continue
                        Err(e) => {
                            eprintln!("Serial read error: {}", e);
                            break;
                        }
                    }
                }
                drop(port_guard);
            }
        });
        
        // Writer thread — receives commands and writes to serial
        let port = Arc::clone(&self.port);
        let running = Arc::clone(&self.running);
        let mut command_rx = // take the receiver from the channel
        
        std::thread::spawn(move || {
            while running.load(std::sync::atomic::Ordering::SeqCst) {
                if let Ok(command) = command_rx.try_recv() {
                    let mut port_guard = port.blocking_lock();
                    if let Some(ref mut port) = *port_guard {
                        let cmd = format!("{}\n", command);
                        let _ = port.write_all(cmd.as_bytes());
                        let _ = port.flush();
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        });
        
        Ok(())
    }
    
    pub fn send_command(&self, command: &str) -> Result<(), String> {
        self.command_tx.blocking_send(command.to_string())
            .map_err(|e| e.to_string())
    }
    
    pub fn stop(&self) {
        self.running.store(false, std::sync::atomic::Ordering::SeqCst);
        // Send CMD:stop to the board as emergency halt
        let _ = self.send_command("CMD:stop");
        // Close the port
        let mut port_guard = self.port.blocking_lock();
        *port_guard = None;
    }
}
```

**Important:** The locking strategy above is simplified. In production, consider using separate cloned port handles for read and write (some serial port implementations support this) or a single-threaded event loop with a select! macro over read readiness and incoming commands.

## Step 2: Structured Output Parser

Parse `SENSOR_ID:VALUE` lines into typed readings:

```rust
#[derive(Debug, Clone, Serialize)]
pub struct SensorReading {
    pub sensor_id: String,
    pub values: Vec<f64>,
    pub raw: String,
    pub timestamp: u64, // millis since connection opened
}

fn parse_sensor_line(line: &str) -> Option<SensorReading> {
    let colon_pos = line.find(':')?;
    let sensor_id = line[..colon_pos].trim().to_string();
    let value_str = line[colon_pos + 1..].trim();
    
    // Skip CMD echo lines
    if sensor_id == "CMD" {
        return None;
    }
    
    // Parse comma-separated values
    let values: Vec<f64> = value_str
        .split(',')
        .filter_map(|v| v.trim().parse().ok())
        .collect();
    
    // Handle boolean sensors (0/1)
    if values.is_empty() {
        // Try parsing as integer for bump switches etc.
        if let Ok(v) = value_str.parse::<i32>() {
            return Some(SensorReading {
                sensor_id,
                values: vec![v as f64],
                raw: line.to_string(),
                timestamp: now_millis(),
            });
        }
        // Could be a string state like "moving_forward"
        return Some(SensorReading {
            sensor_id,
            values: vec![],
            raw: line.to_string(),
            timestamp: now_millis(),
        });
    }
    
    Some(SensorReading {
        sensor_id,
        values,
        raw: line.to_string(),
        timestamp: now_millis(),
    })
}
```

## Step 3: Sensor State Store

The sensor state store aggregates the latest reading for each sensor ID and provides formatted output for the runtime model context:

```rust
// src-tauri/src/services/sensor_state.rs

use std::collections::HashMap;

pub struct SensorStateStore {
    state: HashMap<String, SensorReading>,
    history: HashMap<String, Vec<SensorReading>>, // last N readings for time-series viz
    manifest_components: Vec<Component>,           // for labeling and unit inference
}

impl SensorStateStore {
    pub fn update(&mut self, reading: SensorReading) {
        // Add to history (keep last 20 readings per sensor for time-series)
        let history = self.history.entry(reading.sensor_id.clone()).or_default();
        history.push(reading.clone());
        if history.len() > 20 {
            history.remove(0);
        }
        // Update latest
        self.state.insert(reading.sensor_id.clone(), reading);
    }
    
    /// Format the current state as a text block for model context
    pub fn format_for_model(&self) -> String {
        let mut output = String::from("sensor state (sampled at 100ms intervals):\n");
        
        for component in &self.manifest_components {
            if let Some(reading) = self.state.get(&component.id) {
                let label = &component.label;
                let formatted = self.format_reading(component, reading);
                output.push_str(&format!("- {} ({}): {}\n", label, component.id, formatted));
            }
        }
        
        output
    }
    
    fn format_reading(&self, component: &Component, reading: &SensorReading) -> String {
        match component.component_type.as_str() {
            "ultrasonic" => format!("{}cm", reading.values.first().unwrap_or(&0.0)),
            "bump_switch" => if reading.values.first() == Some(&1.0) { "triggered".into() } else { "clear".into() },
            "encoder" => format!("{} ticks", reading.values.first().unwrap_or(&0.0) as i64),
            "imu" => format!("x={:.2} y={:.2} z={:.2}", 
                reading.values.get(0).unwrap_or(&0.0),
                reading.values.get(1).unwrap_or(&0.0),
                reading.values.get(2).unwrap_or(&0.0)),
            "temp_humidity" => format!("{}°C", reading.values.first().unwrap_or(&0.0)),
            _ => reading.raw.clone(),
        }
    }
}
```

## Step 4: Sensor Visualization Renderer

Spatial and array sensors are rendered as PNG images for the runtime model's multimodal input.

```rust
// src-tauri/src/services/sensor_viz.rs

pub struct SensorVizRenderer;

impl SensorVizRenderer {
    /// Render all spatial sensors into a single composite PNG
    pub fn render(
        state: &SensorStateStore,
        manifest: &Manifest,
    ) -> Option<Vec<u8>> {
        let spatial_sensors = manifest.components.iter()
            .filter(|c| is_spatial_sensor(&c.component_type))
            .collect::<Vec<_>>();
        
        if spatial_sensors.is_empty() {
            return None;
        }
        
        // Use the `image` crate to create a PNG
        // Render each spatial sensor as a sub-image and composite them
    }
}

fn is_spatial_sensor(component_type: &str) -> bool {
    matches!(component_type, 
        "line_sensor_array" | "imu" | "encoder" // time-series for IMU, drift comparison for encoders
    )
}
```

Add to `Cargo.toml`:
```toml
image = "0.25"
imageproc = "0.25"  # for drawing primitives
rusttype = "0.9"    # for text rendering on images
```

### Visualization types

**Line sensor array** (`LINE:00011000`): Render as a horizontal bar of rectangles. Active sensors (1) are cyan, inactive (0) are dark. Scale to a readable size (e.g., 200×40px per array).

**IMU time-series** (`ACCEL:x,y,z GYRO:x,y,z`): Render the last 2 seconds of readings as a line chart. Three colored lines for X/Y/Z axes. Use the history buffer from the sensor state store.

**Encoder drift** (`ENC_L:ticks ENC_R:ticks`): Render as two vertical bars showing cumulative ticks for left and right. Highlight the difference if they diverge significantly.

**Multi-ultrasonic sweep** (if multiple ultrasonic sensors at different angles): Render a top-down arc view with distance arcs radiating from the center.

Each visualization should be:
- Clear enough for a vision model to interpret
- Labeled with sensor names and current values
- Rendered on a dark background (matching the app's aesthetic)
- 400×300px or smaller to avoid context bloat

## Step 5: Camera Frame Capture

If the manifest includes a camera component (e.g., ESP32-CAM), the Rust backend captures frames over WiFi:

```rust
// src-tauri/src/services/camera.rs

pub struct CameraService {
    stream_url: String,    // e.g., http://192.168.1.100:81/stream
    snapshot_url: String,  // e.g., http://192.168.1.100/capture
    client: reqwest::Client,
}

impl CameraService {
    pub async fn capture_frame(&self) -> Result<Vec<u8>, String> {
        let response = self.client
            .get(&self.snapshot_url)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        
        let bytes = response.bytes().await.map_err(|e| e.to_string())?;
        Ok(bytes.to_vec())
    }
}
```

The camera component's manifest entry specifies connection type and URL. For ESP32-CAM, the standard snapshot endpoint is `/capture` on the board's IP. The runtime model receives these frames as JPEG image inputs.

If no camera component exists in the manifest, skip this entirely.

## Step 6: Tauri Commands

```rust
#[tauri::command]
pub async fn open_serial(
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    // Get port and baud from active project's manifest
    // Open serial connection via SerialManager
    // Start reader/writer threads
}

#[tauri::command]
pub async fn close_serial(
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    // Send CMD:stop
    // Close the serial connection
}

#[tauri::command]
pub async fn send_serial_command(
    state: tauri::State<'_, AppState>,
    command: String,
) -> Result<(), String> {
    // Write a command string to the serial port
}

#[tauri::command]
pub async fn get_sensor_state(
    state: tauri::State<'_, AppState>,
) -> Result<SensorStateSnapshot, String> {
    // Return the current sensor state as a structured object
}

#[tauri::command]
pub async fn subscribe_serial(
    state: tauri::State<'_, AppState>,
    on_event: Channel<SerialEvent>,
) -> Result<(), String> {
    // Subscribe to raw serial lines and parsed sensor updates
    // Stream events to frontend via Channel
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "event", content = "data")]
pub enum SerialEvent {
    RawLine(String),
    SensorUpdate { sensor_id: String, values: Vec<f64>, formatted: String },
    Disconnected { error: String },
}
```

## Step 7: Frontend Components

### Serial Monitor Component

A scrolling text view showing raw serial output. Monospace font, 11px, secondary text color. Lines matching the structured format are dimmed slightly. Unrecognized lines appear at full brightness.

```
┌─────────────────────────────┐
│  Serial Monitor         [⏸] │
│  ─────────────────────────  │
│  DIST_FRONT:14              │  ← dimmed (parsed)
│  ENC_L:1842                 │  ← dimmed
│  ENC_R:1836                 │  ← dimmed
│  DEBUG: motor stalled       │  ← full brightness (unparsed)
│  STATE:moving_forward       │  ← dimmed
│  ▮                          │  ← auto-scroll cursor
└─────────────────────────────┘
```

Include a pause/resume toggle to freeze scrolling for inspection. Buffer the last 1000 lines in frontend state.

### Sensor State Panel

Live parsed sensor values, updated in real-time:

```
┌─────────────────────────────┐
│  Sensor State               │
│  ─────────────────────────  │
│  Front Distance    14 cm    │  ← cyan flash on change
│  Left Encoder      1842     │
│  Right Encoder     1836     │
│  Left Bumper       clear    │
│  Right Bumper      triggered│  ← amber text when triggered
│  State             moving   │
└─────────────────────────────┘
```

Each row: component label (left, secondary text) + formatted value (right, sensor color). Values briefly flash cyan when they change (200ms CSS transition).

### Sensor Visualization Panel

Displays the rendered PNG visualization images:

```
┌─────────────────────────────┐
│  Visualizations             │
│  ─────────────────────────  │
│  ┌───────────────────────┐  │
│  │  [IMU time-series     │  │
│  │   line chart]         │  │
│  └───────────────────────┘  │
│  ┌───────────────────────┐  │
│  │  [Encoder drift bars] │  │
│  └───────────────────────┘  │
└─────────────────────────────┘
```

Update images at the same interval as sensor state (100ms renders would be too expensive — throttle to every 500ms or 1s).

These three panels (serial monitor, sensor state, sensor visualization) form the right side of the Runtime Window that opens in Phase 7.

## Step 8: Verify

1. Flash a sketch to a board that outputs structured serial data
2. Call `open_serial` — the connection opens at the manifest's baud rate
3. `subscribe_serial` starts streaming events to the frontend
4. Raw serial lines appear in the serial monitor component
5. Parsed sensor values appear in the sensor state panel and update in real-time
6. `send_serial_command("CMD:stop")` writes to the board and the board responds
7. Sensor visualization images render for spatial sensors (test with a line sensor array or IMU)
8. `close_serial` sends CMD:stop and closes the connection cleanly
9. Disconnecting the USB cable triggers a `Disconnected` event
10. The formatted sensor state text (`format_for_model()`) produces readable output

## Common Issues

**Serial port busy after flashing:** arduino-cli may hold the port briefly after uploading. Add a 1-2 second delay between flash completion and serial connection opening.

**Garbled serial output:** Baud rate mismatch. Ensure the manifest's baud rate matches `Serial.begin()` in the sketch. Also check that the serial buffer doesn't contain leftover data from a previous session — flush on open.

**High CPU usage from serial reader:** The read loop with a 100ms timeout is fine for most cases. If the board outputs data faster than processing, increase the read buffer or throttle the broadcast channel.

**Image rendering performance:** Don't render visualization PNGs on every sensor update. Throttle to 2-5 renders per second. Use double buffering — render the next image while the current one is displayed.

**macOS serial port names:** macOS uses `/dev/cu.*` for outgoing and `/dev/tty.*` for incoming. Use `cu.*` for Arduino communication. The port may also change if the USB cable is reconnected.

## What NOT to Do

- Do not build the runtime agent loop or the runtime window. This phase only builds the serial data pipeline. Phase 7 assembles the context and runs the agentic loop.
- Do not send tool calls through serial in this phase. The `send_serial_command` is for testing raw commands. Tool call dispatch comes in Phase 7.
- Do not try to parse serial data before the connection is opened. The parser should gracefully handle garbled data during connection initialization.
- Do not render sensor visualizations in the frontend. The Rust backend renders PNGs — the frontend only displays the resulting images. This keeps the multimodal context assembly in one place.
- Do not cache sensor state across serial sessions. When the connection closes, the sensor state store resets.