mod commands;
mod models;
mod services;

use models::project::Project;
use services::model_manager::ModelManager;
use std::sync::Mutex;

pub struct AppState {
    pub model_manager: ModelManager,
    pub active_project: Mutex<Option<Project>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let state = AppState {
        model_manager: ModelManager::new(),
        active_project: Mutex::new(None),
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
            commands::projects::create_project,
            commands::projects::list_projects,
            commands::projects::open_project,
            commands::projects::delete_project,
            commands::projects::get_active_project,
            commands::projects::set_board,
            commands::projects::set_serial_port,
            commands::projects::set_baud_rate,
            commands::projects::add_component,
            commands::projects::update_component,
            commands::projects::remove_component,
            commands::projects::list_serial_ports,
            commands::projects::get_component_library,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
