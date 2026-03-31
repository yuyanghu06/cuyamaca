use crate::services::arduino::DetectedBoard;
use crate::AppState;
use serde::Serialize;
use tauri::ipc::Channel;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "event", content = "data")]
pub enum FlashEvent {
    Compiling,
    Uploading,
    Succeeded { binary_size: u64, max_size: u64 },
    Failed { error: String },
}

#[tauri::command]
pub async fn detect_arduino_cli(
    state: tauri::State<'_, AppState>,
) -> Result<bool, String> {
    state.arduino.detect().await
}

#[tauri::command]
pub async fn install_arduino_cli(
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    state.arduino.install().await
}

#[tauri::command]
pub async fn detect_boards(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<DetectedBoard>, String> {
    let available = state.arduino.detect().await.unwrap_or(false);
    if !available {
        return Err("arduino-cli is not installed".to_string());
    }
    state.arduino.list_boards().await
}

#[tauri::command]
pub async fn flash_sketch(
    state: tauri::State<'_, AppState>,
    on_event: Channel<FlashEvent>,
) -> Result<(), String> {
    // Get project info (release lock before async work)
    let (sketch, fqbn, port) = {
        let active = state
            .active_project
            .lock()
            .map_err(|e| e.to_string())?;
        let project = active.as_ref().ok_or("No active project")?;

        let sketch = project
            .sketch
            .as_ref()
            .ok_or("No sketch to flash. Approve a sketch first.")?
            .clone();

        let fqbn = if project.manifest.board.is_empty() {
            return Err("Board FQBN not set in manifest. Configure your board first.".to_string());
        } else {
            project.manifest.board.clone()
        };

        let port = if project.manifest.serial_port.is_empty() {
            return Err(
                "Serial port not set in manifest. Select a port first.".to_string(),
            );
        } else {
            project.manifest.serial_port.clone()
        };

        (sketch, fqbn, port)
    };

    // Verify arduino-cli is available
    let available = state.arduino.detect().await.unwrap_or(false);
    if !available {
        let _ = on_event.send(FlashEvent::Failed {
            error: "arduino-cli is not installed".to_string(),
        });
        return Err("arduino-cli is not installed".to_string());
    }

    // Verify board is connected
    match state.arduino.list_boards().await {
        Ok(boards) => {
            let port_found = boards.iter().any(|b| b.port == port);
            if !port_found {
                let _ = on_event.send(FlashEvent::Failed {
                    error: format!(
                        "No board detected on port {}. Check your USB connection.",
                        port
                    ),
                });
                return Err(format!("No board detected on port {}", port));
            }
        }
        Err(e) => {
            let _ = on_event.send(FlashEvent::Failed {
                error: format!("Board detection failed: {}", e),
            });
            return Err(e);
        }
    }

    // Compile
    let _ = on_event.send(FlashEvent::Compiling);

    // Upload (compile + upload via arduino-cli)
    let _ = on_event.send(FlashEvent::Uploading);

    match state
        .arduino
        .compile_and_flash(&sketch, &fqbn, &port)
        .await
    {
        Ok(result) => {
            let _ = on_event.send(FlashEvent::Succeeded {
                binary_size: result.binary_size,
                max_size: result.max_size,
            });
            Ok(())
        }
        Err(e) => {
            let _ = on_event.send(FlashEvent::Failed {
                error: e.clone(),
            });
            Err(e)
        }
    }
}
