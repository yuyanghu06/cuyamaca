mod commands;
mod models;
mod services;

use commands::runtime::RuntimeSession;
use models::project::Project;
use services::arduino::ArduinoService;
use services::model_manager::ModelManager;
use services::process_manager::ProcessManager;
use services::serial::SerialConnection;
use std::sync::Mutex;

pub struct AppState {
    pub model_manager: ModelManager,
    pub active_project: Mutex<Option<Project>>,
    pub arduino: ArduinoService,
    pub serial: Mutex<Option<SerialConnection>>,
    pub runtime: Mutex<Option<RuntimeSession>>,
    pub process_manager: ProcessManager,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let state = AppState {
        model_manager: ModelManager::new(),
        active_project: Mutex::new(None),
        arduino: ArduinoService::new(),
        serial: Mutex::new(None),
        runtime: Mutex::new(None),
        process_manager: ProcessManager::new(),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .setup(|app| {
            use services::keystore;
            use services::model_manager::{ProviderType, SlotConfig};
            use tauri::Manager;
            use tauri_plugin_store::StoreExt;

            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let Ok(store) = handle.store("settings.json") else { return };
                let state = handle.state::<AppState>();

                for slot in ["code", "runtime"] {
                    let key = format!("model_config_{}", slot);
                    let Some(value) = store.get(&key) else { continue };
                    let provider_str = match value["provider"].as_str() {
                        Some(s) => s.to_string(),
                        None => continue,
                    };
                    let model_str = match value["model"].as_str() {
                        Some(s) if !s.is_empty() => s.to_string(),
                        _ => continue,
                    };
                    let Ok(provider_type) = provider_str.parse::<ProviderType>() else { continue };
                    let api_key = if provider_type != ProviderType::Ollama {
                        keystore::get_api_key(&provider_str).ok().flatten()
                    } else {
                        None
                    };
                    let config = SlotConfig { provider: provider_type, model: model_str };
                    match slot {
                        "code" => { let _ = state.model_manager.configure_code_model(config, api_key).await; }
                        "runtime" => { let _ = state.model_manager.configure_runtime_model(config, api_key).await; }
                        _ => {}
                    }
                }
            });
            Ok(())
        })
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
            commands::models::pull_ollama_model,
            commands::models::delete_ollama_model,
            commands::projects::create_project,
            commands::projects::list_projects,
            commands::projects::open_project,
            commands::projects::delete_project,
            commands::projects::get_projects_path,
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
            commands::codegen::stream_chat_message,
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
            commands::runtime::open_runtime_window,
            commands::runtime::runtime_send_message,
            commands::runtime::runtime_kill,
            commands::runtime::close_runtime_window,
            commands::setup::check_dependencies,
            commands::setup::install_dependency,
            commands::setup::skip_dependency_setup,
            commands::setup::mark_setup_complete,
            commands::setup::is_setup_complete,
            commands::setup::start_ollama,
            commands::setup::stop_ollama,
            commands::setup::restart_ollama,
            commands::setup::get_ollama_process_state,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
