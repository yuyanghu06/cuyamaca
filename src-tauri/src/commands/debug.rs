#[tauri::command]
pub fn ping(message: String) -> Result<String, String> {
    Ok(format!("pong: {}", message))
}
