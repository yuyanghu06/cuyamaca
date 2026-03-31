mod commands;
mod services;

use services::model_manager::ModelManager;

pub struct AppState {
    pub model_manager: ModelManager,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let state = AppState {
        model_manager: ModelManager::new(),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::debug::ping,
            commands::models::list_providers,
            commands::models::configure_model_slot,
            commands::models::get_slot_config,
            commands::models::check_model_health,
            commands::models::check_ollama_health,
            commands::models::list_ollama_models,
            commands::models::store_api_key,
            commands::models::has_api_key,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
