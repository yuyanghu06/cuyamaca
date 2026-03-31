use crate::services::sensor_state::SensorStateSnapshot;
use crate::services::sensor_viz::SensorVizRenderer;
use crate::services::serial::SerialConnection;
use crate::AppState;
use serde::Serialize;
use std::sync::atomic::Ordering;
use tauri::ipc::Channel;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "event", content = "data")]
pub enum SerialEvent {
    RawLine(String),
    SensorUpdate {
        sensor_id: String,
        values: Vec<f64>,
        formatted: String,
    },
    Disconnected {
        error: String,
    },
}

#[tauri::command]
pub async fn open_serial(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let (port_name, baud_rate, components) = {
        let project = state
            .active_project
            .lock()
            .map_err(|e| e.to_string())?;
        let project = project.as_ref().ok_or("No active project")?;
        let m = &project.manifest;
        if m.serial_port.is_empty() {
            return Err("No serial port configured in manifest".to_string());
        }
        (m.serial_port.clone(), m.baud_rate, m.components.clone())
    };

    // Close any existing connection first
    {
        let mut serial = state.serial.lock().map_err(|e| e.to_string())?;
        if let Some(conn) = serial.take() {
            conn.stop();
        }
    }

    // Brief delay after flash to let the port settle
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let connection = SerialConnection::open(&port_name, baud_rate, components)?;

    let mut serial = state.serial.lock().map_err(|e| e.to_string())?;
    *serial = Some(connection);

    Ok(())
}

#[tauri::command]
pub async fn close_serial(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut serial = state.serial.lock().map_err(|e| e.to_string())?;
    if let Some(conn) = serial.take() {
        conn.stop();
    }
    Ok(())
}

#[tauri::command]
pub async fn send_serial_command(
    state: tauri::State<'_, AppState>,
    command: String,
) -> Result<(), String> {
    let serial = state.serial.lock().map_err(|e| e.to_string())?;
    let conn = serial.as_ref().ok_or("No serial connection open")?;
    conn.send_command(&command)
}

#[tauri::command]
pub async fn get_sensor_state(
    state: tauri::State<'_, AppState>,
) -> Result<SensorStateSnapshot, String> {
    let serial = state.serial.lock().map_err(|e| e.to_string())?;
    let conn = serial.as_ref().ok_or("No serial connection open")?;
    Ok(conn.get_sensor_state_snapshot())
}

#[tauri::command]
pub async fn get_sensor_viz(
    state: tauri::State<'_, AppState>,
) -> Result<Option<Vec<u8>>, String> {
    let serial = state.serial.lock().map_err(|e| e.to_string())?;
    let conn = serial.as_ref().ok_or("No serial connection open")?;
    let sensor_state = conn.sensor_state().read().unwrap();
    Ok(SensorVizRenderer::render(&sensor_state))
}

#[tauri::command]
pub async fn subscribe_serial(
    state: tauri::State<'_, AppState>,
    on_event: Channel<SerialEvent>,
) -> Result<(), String> {
    let (mut raw_rx, mut sensor_rx, running) = {
        let serial = state.serial.lock().map_err(|e| e.to_string())?;
        let conn = serial.as_ref().ok_or("No serial connection open")?;
        (
            conn.subscribe_raw(),
            conn.subscribe_sensors(),
            conn.running_flag(),
        )
    };

    tokio::spawn(async move {
        loop {
            if !running.load(Ordering::Relaxed) {
                let _ = on_event.send(SerialEvent::Disconnected {
                    error: "Connection closed".to_string(),
                });
                break;
            }

            tokio::select! {
                result = raw_rx.recv() => {
                    match result {
                        Ok(line) => {
                            let _ = on_event.send(SerialEvent::RawLine(line));
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                            let _ = on_event.send(SerialEvent::Disconnected {
                                error: "Serial connection closed".to_string(),
                            });
                            break;
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            eprintln!("Serial subscriber lagged by {} messages", n);
                        }
                    }
                }
                result = sensor_rx.recv() => {
                    match result {
                        Ok(update) => {
                            let _ = on_event.send(SerialEvent::SensorUpdate {
                                sensor_id: update.sensor_id,
                                values: update.values,
                                formatted: update.formatted,
                            });
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                    }
                }
            }
        }
    });

    Ok(())
}
