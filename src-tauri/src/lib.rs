mod commands;
mod models;
mod services;

use models::project::Project;
use services::arduino::ArduinoService;
use services::model_manager::ModelManager;
use services::serial::SerialConnection;
use std::sync::Mutex;

pub struct AppState {
    pub model_manager: ModelManager,
    pub active_project: Mutex<Option<Project>>,
    pub arduino: ArduinoService,
    pub serial: Mutex<Option<SerialConnection>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let state = AppState {
        model_manager: ModelManager::new(),
        active_project: Mutex::new(None),
        arduino: ArduinoService::new(),
        serial: Mutex::new(None),
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
            commands::codegen::generate_sketch,
            commands::codegen::modify_sketch,
            commands::codegen::approve_sketch,
            commands::codegen::reject_sketch,
            commands::codegen::upload_sketch,
            commands::codegen::get_sketch,
            commands::codegen::get_tools,
            commands::codegen::send_chat_message,
            commands::codegen::clear_chat_history,
            commands::flash::detect_arduino_cli,
            commands::flash::install_arduino_cli,
            commands::flash::detect_boards,
            commands::flash::flash_sketch,
            commands::serial::open_serial,
            commands::serial::close_serial,
            commands::serial::send_serial_command,
            commands::serial::get_sensor_state,
            commands::serial::get_sensor_viz,
            commands::serial::subscribe_serial,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
