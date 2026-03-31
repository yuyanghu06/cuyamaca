---
name: cuyamaca-arduino-flash
description: Integrate arduino-cli into Cuyamaca for compiling and flashing Arduino sketches to boards. Use this skill whenever the user wants to add board flashing, integrate arduino-cli, implement the compile and upload workflow, detect connected boards, manage arduino-cli as a child process, or references "phase 5", "arduino-cli", "flash", "compile", "upload sketch", "board detection", or "flashing workflow". Also trigger when the user asks about arduino-cli commands, board FQBN strings, core installation, or the compile-flash pipeline. This skill assumes Phase 4 is complete (code generation, sketch approval, tools.json).
---

# Phase 5 — Arduino-CLI Integration + Flashing

This skill adds the ability to compile and flash approved sketches to Arduino boards using arduino-cli. The Rust backend manages arduino-cli as a child process, handles board/core detection, and provides a compile → flash pipeline triggered by the "Approve & Flash" button.

## What This Skill Produces

- arduino-cli detection and auto-installation on first run
- Board detection: enumerate connected boards with their FQBN
- Core management: install required Arduino cores automatically
- Compile pipeline: invoke arduino-cli to compile the sketch
- Flash pipeline: upload the compiled sketch to the board
- Compile/flash status UI: progress indicator with error display
- The "Approve & Flash" button in Code View becomes fully functional
- Sidebar health indicator for arduino-cli wired up

## Prerequisites

- Phase 4 complete: sketch generation, approval, and tools.json working
- A physical Arduino board connected via USB (for testing)
- USB drivers installed for the board (CH340 driver for clone boards)

## Step 1: arduino-cli Detection and Installation

On app startup, check if arduino-cli is installed and accessible:

```rust
// src-tauri/src/services/arduino.rs

pub struct ArduinoService {
    cli_path: PathBuf,
}

impl ArduinoService {
    pub async fn detect() -> Result<ArduinoService, String> {
        // Try to find arduino-cli in PATH
        let output = tokio::process::Command::new("arduino-cli")
            .arg("version")
            .output()
            .await;
        
        match output {
            Ok(out) if out.status.success() => {
                Ok(ArduinoService {
                    cli_path: PathBuf::from("arduino-cli"),
                })
            }
            _ => Err("arduino-cli not found".to_string()),
        }
    }

    pub async fn install() -> Result<ArduinoService, String> {
        // Platform-specific installation
        // macOS: brew install arduino-cli OR direct binary download
        // Windows: winget install Arduino.ArduinoCLI OR direct download
        // Download from: https://github.com/arduino/arduino-cli/releases
    }
}
```

### Auto-installation flow

If arduino-cli is not found:

1. Show a prompt in the UI: "arduino-cli is required but not installed. Install now?"
2. On confirmation, download the platform-appropriate binary
3. Place it in the app's data directory (not in system PATH — avoid permission issues)
4. Use the full path to invoke it going forward

**macOS:** Download the macOS arm64 or amd64 binary from GitHub releases. Make it executable with `chmod +x`.

**Windows:** Download the Windows zip from GitHub releases. Extract the `.exe`.

Store the path to the installed binary in the app's config store so it persists across restarts.

## Step 2: Board Detection

```rust
impl ArduinoService {
    pub async fn list_boards(&self) -> Result<Vec<DetectedBoard>, String> {
        let output = self.run_cli(&["board", "list", "--format", "json"]).await?;
        let boards: Vec<DetectedBoard> = serde_json::from_str(&output)?;
        Ok(boards)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedBoard {
    pub port: String,           // /dev/cu.usbmodem14201 or COM3
    pub fqbn: Option<String>,   // arduino:avr:uno (None if unrecognized)
    pub board_name: Option<String>, // "Arduino Uno"
    pub protocol: String,       // "serial"
}
```

`arduino-cli board list --format json` returns a JSON array of connected boards with their port, FQBN (Fully Qualified Board Name), and protocol.

Use this to:
- Auto-populate the serial port dropdown in the Manifest View (supplement the `serialport` crate enumeration from Phase 3)
- Auto-detect the board FQBN so the user doesn't have to remember it
- Verify the board is connected before attempting to flash

## Step 3: Core Management

Arduino cores must be installed before a board can be compiled for. For example, an Arduino Uno needs the `arduino:avr` core.

```rust
impl ArduinoService {
    pub async fn ensure_core_installed(&self, fqbn: &str) -> Result<(), String> {
        let core = extract_core_from_fqbn(fqbn)?; // "arduino:avr:uno" → "arduino:avr"
        
        // Check if already installed
        let installed = self.run_cli(&["core", "list", "--format", "json"]).await?;
        if core_is_installed(&installed, &core) {
            return Ok(());
        }
        
        // Update index and install
        self.run_cli(&["core", "update-index"]).await?;
        self.run_cli(&["core", "install", &core]).await?;
        
        Ok(())
    }
}

fn extract_core_from_fqbn(fqbn: &str) -> Result<String, String> {
    // "arduino:avr:uno" → "arduino:avr"
    let parts: Vec<&str> = fqbn.splitn(3, ':').collect();
    if parts.len() < 2 {
        return Err(format!("Invalid FQBN: {}", fqbn));
    }
    Ok(format!("{}:{}", parts[0], parts[1]))
}
```

For ESP32 boards, the user needs to add the ESP32 board manager URL first:
```
arduino-cli config add board_manager.additional_urls https://raw.githubusercontent.com/espressif/arduino-esp32/gh-pages/package_esp32_index.json
```

Handle this automatically when the FQBN starts with `esp32:`.

## Step 4: Compile Pipeline

```rust
impl ArduinoService {
    pub async fn compile(
        &self,
        sketch_path: &Path,
        fqbn: &str,
        on_progress: impl Fn(CompileProgress) + Send + 'static,
    ) -> Result<CompileResult, String> {
        // Ensure the core is installed
        self.ensure_core_installed(fqbn).await?;
        
        on_progress(CompileProgress::Started);
        
        let output = self.run_cli(&[
            "compile",
            "--fqbn", fqbn,
            sketch_path.to_str().unwrap(),
            "--format", "json",
        ]).await;
        
        match output {
            Ok(json) => {
                on_progress(CompileProgress::Succeeded);
                Ok(CompileResult::from_json(&json)?)
            }
            Err(e) => {
                on_progress(CompileProgress::Failed(e.clone()));
                Err(e)
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum CompileProgress {
    Started,
    Succeeded,
    Failed(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct CompileResult {
    pub binary_size: u64,
    pub max_size: u64,
    pub warnings: Vec<String>,
}
```

The sketch must be in a directory named after itself for arduino-cli:
```
{temp_dir}/sketch/sketch.ino
```

Create a temporary directory, copy the sketch there, compile, and clean up.

## Step 5: Flash Pipeline

```rust
impl ArduinoService {
    pub async fn flash(
        &self,
        sketch_path: &Path,
        fqbn: &str,
        port: &str,
        on_progress: impl Fn(FlashProgress) + Send + 'static,
    ) -> Result<(), String> {
        on_progress(FlashProgress::Compiling);
        
        // Compile first (arduino-cli upload can compile + upload in one step)
        let output = self.run_cli(&[
            "upload",
            "--fqbn", fqbn,
            "--port", port,
            sketch_path.to_str().unwrap(),
            "--format", "json",
        ]).await;
        
        match output {
            Ok(_) => {
                on_progress(FlashProgress::Succeeded);
                Ok(())
            }
            Err(e) => {
                on_progress(FlashProgress::Failed(e.clone()));
                Err(e)
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum FlashProgress {
    Compiling,
    Uploading,
    Succeeded,
    Failed(String),
}
```

`arduino-cli upload` compiles and uploads in one step. Use this instead of separate compile + upload unless you need to cache the compiled binary.

## Step 6: CLI Runner Utility

All arduino-cli invocations go through a shared runner:

```rust
impl ArduinoService {
    async fn run_cli(&self, args: &[&str]) -> Result<String, String> {
        let output = tokio::process::Command::new(&self.cli_path)
            .args(args)
            .output()
            .await
            .map_err(|e| format!("Failed to run arduino-cli: {}", e))?;
        
        if output.status.success() {
            String::from_utf8(output.stdout)
                .map_err(|e| e.to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("arduino-cli error: {}", stderr))
        }
    }
}
```

Key considerations:
- Set a reasonable timeout (60s for compile, 30s for upload)
- Capture both stdout and stderr
- Parse JSON output when `--format json` is used
- Handle the case where arduino-cli hangs (e.g., waiting for board reset)

## Step 7: Tauri Commands

```rust
#[tauri::command]
pub async fn detect_arduino_cli(
    state: tauri::State<'_, AppState>,
) -> Result<bool, String> {
    // Check if arduino-cli is available
    // Update health status
}

#[tauri::command]
pub async fn install_arduino_cli(
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    // Download and install arduino-cli
}

#[tauri::command]
pub async fn detect_boards(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<DetectedBoard>, String> {
    // Run arduino-cli board list
}

#[tauri::command]
pub async fn flash_sketch(
    state: tauri::State<'_, AppState>,
    on_progress: Channel<FlashEvent>,
) -> Result<(), String> {
    // Get active project's sketch, board FQBN, and port from manifest
    // Create temp directory with sketch
    // Compile and flash
    // Stream progress events to frontend via Channel
    // On success, transition to runtime mode (Phase 7)
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "event", content = "data")]
pub enum FlashEvent {
    Compiling,
    Uploading,
    Succeeded { binary_size: u64 },
    Failed { error: String },
}
```

## Step 8: Flash UI in Code View

Update the "Approve & Flash" button flow:

1. User clicks "Approve & Flash"
2. The pending sketch is saved (approve_sketch from Phase 4)
3. A flash progress overlay appears on the Code View:

```
┌─────────────────────────────────────────┐
│                                         │
│         ◉ Compiling sketch...           │
│         [━━━━━━━━━░░░░░░░░░░]           │
│                                         │
│         Board: Arduino Uno              │
│         Port: /dev/cu.usbmodem14201     │
│                                         │
└─────────────────────────────────────────┘
```

- **Compiling:** Amber pulsing indicator + "Compiling sketch..."
- **Uploading:** Cyan pulsing indicator + "Uploading to board..."
- **Succeeded:** Green checkmark + "Flashed successfully" + binary size info. Auto-dismiss after 2 seconds.
- **Failed:** Red X + error message. Show the full compiler or upload error in a scrollable monospace block. Include a "Try Again" button.

### Pre-flash checks

Before starting the flash:
1. Verify a sketch exists (approved)
2. Verify the board FQBN is set in the manifest
3. Verify the serial port is set and the board is connected (run `detect_boards` and confirm the port is present)
4. If any check fails, show an error message instead of starting the flash

## Step 9: Wire Up Sidebar Health

Update the arduino-cli sidebar indicator:
- On app startup, call `detect_arduino_cli()`
- Green dot if found and responsive
- Red dot if not found
- Amber dot during installation

Also add a board connection indicator — green if a board is detected on the configured port, red otherwise. Poll every 10 seconds or on window focus.

## Step 10: Verify

1. App detects arduino-cli on startup (or offers to install it)
2. `detect_boards` returns connected boards with FQBN and port
3. The serial port dropdown in Manifest View shows detected ports
4. Board FQBN auto-fills when a known board is detected
5. "Approve & Flash" compiles the sketch — progress overlay shows "Compiling"
6. Compilation succeeds — progress shows "Uploading"
7. Upload succeeds — progress shows "Flashed successfully" with binary size
8. If compilation fails (syntax error in sketch), the error message is displayed
9. If the board is disconnected, a clear error appears before flash attempt
10. Sidebar arduino-cli dot is green when installed, red when missing

## Common Issues

**arduino-cli not in PATH:** Don't rely on PATH. Store the full binary path and invoke it directly. The auto-install puts it in the app data directory.

**Port busy (macOS):** If another process has the serial port open (like Serial Monitor in Arduino IDE), flashing will fail. The error message should mention closing other serial connections.

**Core installation takes time:** The first compile for a new board type requires downloading the core (can be 100MB+ for ESP32). Show a progress indicator and don't timeout.

**Board requires manual reset:** Some boards (Leonardo, Micro) need a manual reset to enter bootloader mode. Detect this from the board type and show instructions.

**Windows COM port permissions:** Windows may require the user to install USB drivers separately. Include troubleshooting guidance for common drivers (CH340, CP2102, FTDI).

## What NOT to Do

- Do not keep arduino-cli running as a persistent child process. Invoke it on demand for compile and flash operations.
- Do not open the serial port for communication in this phase. Serial reading/writing is Phase 6. Flashing uses arduino-cli's own serial handling.
- Do not start the runtime window after flashing in this phase. The transition to runtime mode is Phase 7. For now, flashing just shows a success message.
- Do not bundle arduino-cli in the app binary yet. That's a future roadmap item. For now, detect or download it at runtime.
- Do not attempt to auto-detect libraries the sketch needs. If the sketch uses external libraries, the user needs to install them manually for now (or the code model should avoid external libraries).