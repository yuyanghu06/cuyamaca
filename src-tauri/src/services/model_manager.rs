use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use super::anthropic::AnthropicProvider;
use super::google::GoogleProvider;
use super::mistral::MistralProvider;
use super::ollama::OllamaProvider;
use super::openai::OpenAIProvider;
use super::provider::ModelProvider;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ProviderType {
    Ollama,
    OpenAI,
    Anthropic,
    Google,
    Mistral,
}

impl std::fmt::Display for ProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderType::Ollama => write!(f, "ollama"),
            ProviderType::OpenAI => write!(f, "openai"),
            ProviderType::Anthropic => write!(f, "anthropic"),
            ProviderType::Google => write!(f, "google"),
            ProviderType::Mistral => write!(f, "mistral"),
        }
    }
}

impl std::str::FromStr for ProviderType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "ollama" => Ok(ProviderType::Ollama),
            "openai" => Ok(ProviderType::OpenAI),
            "anthropic" => Ok(ProviderType::Anthropic),
            "google" => Ok(ProviderType::Google),
            "mistral" => Ok(ProviderType::Mistral),
            _ => Err(format!("Unknown provider: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotConfig {
    pub provider: ProviderType,
    pub model: String,
}

pub struct ModelManager {
    code_model: RwLock<Option<Arc<dyn ModelProvider>>>,
    runtime_model: RwLock<Option<Arc<dyn ModelProvider>>>,
    code_config: RwLock<Option<SlotConfig>>,
    runtime_config: RwLock<Option<SlotConfig>>,
}

impl ModelManager {
    pub fn new() -> Self {
        Self {
            code_model: RwLock::new(None),
            runtime_model: RwLock::new(None),
            code_config: RwLock::new(None),
            runtime_config: RwLock::new(None),
        }
    }

    pub async fn configure_code_model(
        &self,
        config: SlotConfig,
        api_key: Option<String>,
    ) -> Result<(), String> {
        let provider = Self::create_provider(&config, api_key)?;
        *self.code_model.write().await = Some(provider);
        *self.code_config.write().await = Some(config);
        Ok(())
    }

    pub async fn configure_runtime_model(
        &self,
        config: SlotConfig,
        api_key: Option<String>,
    ) -> Result<bool, String> {
        let provider = Self::create_provider(&config, api_key)?;
        let multimodal = provider.supports_multimodal();
        *self.runtime_model.write().await = Some(provider);
        *self.runtime_config.write().await = Some(config);
        // Return whether multimodal is supported — caller warns the user if not
        Ok(multimodal)
    }

    pub async fn code_model(&self) -> Result<Arc<dyn ModelProvider>, String> {
        self.code_model
            .read()
            .await
            .clone()
            .ok_or_else(|| "Code model not configured".to_string())
    }

    pub async fn runtime_model(&self) -> Result<Arc<dyn ModelProvider>, String> {
        self.runtime_model
            .read()
            .await
            .clone()
            .ok_or_else(|| "Runtime model not configured".to_string())
    }

    pub async fn code_config(&self) -> Option<SlotConfig> {
        self.code_config.read().await.clone()
    }

    pub async fn runtime_config(&self) -> Option<SlotConfig> {
        self.runtime_config.read().await.clone()
    }

    fn create_provider(
        config: &SlotConfig,
        api_key: Option<String>,
    ) -> Result<Arc<dyn ModelProvider>, String> {
        match config.provider {
            ProviderType::Ollama => {
                Ok(Arc::new(OllamaProvider::new(config.model.clone(), None)))
            }
            ProviderType::OpenAI => {
                let key = api_key.ok_or("OpenAI requires an API key")?;
                Ok(Arc::new(OpenAIProvider::new(
                    config.model.clone(),
                    key,
                    None,
                )))
            }
            ProviderType::Anthropic => {
                let key = api_key.ok_or("Anthropic requires an API key")?;
                Ok(Arc::new(AnthropicProvider::new(
                    config.model.clone(),
                    key,
                    None,
                )))
            }
            ProviderType::Google => {
                let key = api_key.ok_or("Google requires an API key")?;
                Ok(Arc::new(GoogleProvider::new(
                    config.model.clone(),
                    key,
                    None,
                )))
            }
            ProviderType::Mistral => {
                let key = api_key.ok_or("Mistral requires an API key")?;
                Ok(Arc::new(MistralProvider::new(
                    config.model.clone(),
                    key,
                    None,
                )))
            }
        }
    }
}
