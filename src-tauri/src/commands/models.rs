use crate::services::keystore;
use crate::services::model_manager::{ProviderType, SlotConfig};
use crate::services::ollama::OllamaProvider;
use crate::services::provider::ModelProvider;
use crate::AppState;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub requires_key: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SlotConfigResponse {
    pub provider: String,
    pub model: String,
    pub multimodal_warning: bool,
}

#[tauri::command]
pub fn list_providers() -> Vec<ProviderInfo> {
    vec![
        ProviderInfo {
            id: "ollama".to_string(),
            name: "Ollama (Local)".to_string(),
            requires_key: false,
        },
        ProviderInfo {
            id: "openai".to_string(),
            name: "OpenAI".to_string(),
            requires_key: true,
        },
        ProviderInfo {
            id: "anthropic".to_string(),
            name: "Anthropic".to_string(),
            requires_key: true,
        },
        ProviderInfo {
            id: "google".to_string(),
            name: "Google".to_string(),
            requires_key: true,
        },
        ProviderInfo {
            id: "mistral".to_string(),
            name: "Mistral".to_string(),
            requires_key: true,
        },
    ]
}

#[tauri::command]
pub async fn configure_model_slot(
    state: tauri::State<'_, AppState>,
    slot: String,
    provider: String,
    model: String,
    api_key: Option<String>,
) -> Result<SlotConfigResponse, String> {
    let provider_type: ProviderType = provider.parse()?;

    // Store API key in keychain if provided
    if let Some(ref key) = api_key {
        if provider_type != ProviderType::Ollama {
            keystore::store_api_key(&provider, key)?;
        }
    }

    // Resolve API key: use provided key or fetch from keychain
    let resolved_key = match api_key {
        Some(key) => Some(key),
        None if provider_type != ProviderType::Ollama => keystore::get_api_key(&provider)?,
        _ => None,
    };

    let config = SlotConfig {
        provider: provider_type,
        model: model.clone(),
    };

    let multimodal_warning = match slot.as_str() {
        "code" => {
            state.model_manager.configure_code_model(config, resolved_key).await?;
            false
        }
        "runtime" => {
            let supports_multimodal = state
                .model_manager
                .configure_runtime_model(config, resolved_key)
                .await?;
            !supports_multimodal
        }
        _ => return Err(format!("Invalid slot: {}. Use 'code' or 'runtime'", slot)),
    };

    Ok(SlotConfigResponse {
        provider,
        model,
        multimodal_warning,
    })
}

#[tauri::command]
pub async fn get_slot_config(
    state: tauri::State<'_, AppState>,
    slot: String,
) -> Result<Option<SlotConfigResponse>, String> {
    let config = match slot.as_str() {
        "code" => state.model_manager.code_config().await,
        "runtime" => state.model_manager.runtime_config().await,
        _ => return Err(format!("Invalid slot: {}", slot)),
    };

    Ok(config.map(|c| SlotConfigResponse {
        provider: c.provider.to_string(),
        model: c.model,
        multimodal_warning: false,
    }))
}

#[tauri::command]
pub async fn check_model_health(
    state: tauri::State<'_, AppState>,
    slot: String,
) -> Result<bool, String> {
    let provider = match slot.as_str() {
        "code" => state.model_manager.code_model().await,
        "runtime" => state.model_manager.runtime_model().await,
        _ => return Err(format!("Invalid slot: {}", slot)),
    };

    match provider {
        Ok(p) => Ok(p.is_healthy().await),
        Err(_) => Ok(false),
    }
}

#[tauri::command]
pub async fn check_ollama_health() -> Result<bool, String> {
    let provider = OllamaProvider::new("".to_string(), None);
    Ok(provider.is_healthy().await)
}

#[tauri::command]
pub async fn list_ollama_models() -> Result<Vec<crate::services::provider::ModelInfo>, String> {
    let provider = OllamaProvider::new("".to_string(), None);
    provider.list_models().await
}

#[tauri::command]
pub async fn store_api_key(provider: String, key: String) -> Result<(), String> {
    keystore::store_api_key(&provider, &key)
}

#[tauri::command]
pub async fn has_api_key(provider: String) -> Result<bool, String> {
    Ok(keystore::get_api_key(&provider)?.is_some())
}
