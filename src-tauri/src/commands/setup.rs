use crate::services::dependency::{
    self, DependencyStatus, InstallEvent,
};
use tauri::ipc::Channel;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

#[tauri::command]
pub async fn check_dependencies() -> Result<DependencyStatus, String> {
    let (ollama, arduino_cli) =
        tokio::join!(dependency::detect_ollama(), dependency::detect_arduino_cli());
    Ok(DependencyStatus {
        ollama,
        arduino_cli,
    })
}

#[tauri::command]
pub async fn install_dependency(
    dep: String,
    on_event: Channel<InstallEvent>,
) -> Result<(), String> {
    let data_dir = get_app_data_dir()?;
    std::fs::create_dir_all(&data_dir)
        .map_err(|e| format!("Cannot create app data dir: {}", e))?;

    match dep.as_str() {
        "ollama" => dependency::install_ollama(&data_dir, &on_event).await,
        "arduino-cli" => dependency::install_arduino_cli(&data_dir, &on_event).await,
        _ => Err(format!("Unknown dependency: {}", dep)),
    }
}

#[tauri::command]
pub async fn skip_dependency_setup(app: AppHandle) -> Result<(), String> {
    let store = app
        .store("setup.json")
        .map_err(|e| format!("Failed to open store: {}", e))?;
    store.set("setup_complete", serde_json::json!(true));
    store
        .save()
        .map_err(|e| format!("Failed to save store: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn mark_setup_complete(app: AppHandle) -> Result<(), String> {
    let store = app
        .store("setup.json")
        .map_err(|e| format!("Failed to open store: {}", e))?;
    store.set("setup_complete", serde_json::json!(true));
    store
        .save()
        .map_err(|e| format!("Failed to save store: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn is_setup_complete(app: AppHandle) -> Result<bool, String> {
    let store = app
        .store("setup.json")
        .map_err(|e| format!("Failed to open store: {}", e))?;
    let complete = store
        .get("setup_complete")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    Ok(complete)
}

fn get_app_data_dir() -> Result<std::path::PathBuf, String> {
    let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
    Ok(home.join(".cuyamaca"))
}
