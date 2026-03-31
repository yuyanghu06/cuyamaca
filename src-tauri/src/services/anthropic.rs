use super::provider::*;
use futures_util::StreamExt;
use serde_json::json;
use tokio::sync::mpsc;

const MULTIMODAL_MODELS: &[&str] = &["claude-sonnet", "claude-opus", "claude-3"];

pub struct AnthropicProvider {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    model: String,
}

impl AnthropicProvider {
    pub fn new(model: String, api_key: String, base_url: Option<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url
                .unwrap_or_else(|| "https://api.anthropic.com".to_string()),
            api_key,
            model,
        }
    }

    fn build_messages(&self, request: &CompletionRequest) -> Vec<serde_json::Value> {
        // Anthropic: system goes in a separate field, not in messages
        request
            .messages
            .iter()
            .map(|msg| match &msg.content {
                MessageContent::Text(text) => {
                    json!({ "role": msg.role, "content": text })
                }
                MessageContent::Multimodal(parts) => {
                    let content: Vec<serde_json::Value> = parts
                        .iter()
                        .map(|part| match part {
                            ContentPart::Text { text } => {
                                json!({ "type": "text", "text": text })
                            }
                            ContentPart::Image { data, media_type } => {
                                json!({
                                    "type": "image",
                                    "source": {
                                        "type": "base64",
                                        "media_type": media_type,
                                        "data": data,
                                    }
                                })
                            }
                        })
                        .collect();
                    json!({ "role": msg.role, "content": content })
                }
            })
            .collect()
    }
}

#[async_trait::async_trait]
impl ModelProvider for AnthropicProvider {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, String> {
        let messages = self.build_messages(&request);
        let mut body = json!({
            "model": self.model,
            "messages": messages,
            "max_tokens": request.max_tokens.unwrap_or(4096),
        });

        if let Some(ref system) = request.system_prompt {
            body["system"] = json!(system);
        }
        if let Some(temp) = request.temperature {
            body["temperature"] = json!(temp);
        }

        let resp = self
            .client
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .timeout(std::time::Duration::from_secs(60))
            .send()
            .await
            .map_err(|e| format!("Anthropic request failed: {}", e))?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(match status.as_u16() {
                401 => "Anthropic: Invalid API key".to_string(),
                429 => "Anthropic: Rate limit exceeded".to_string(),
                _ => format!("Anthropic error {}: {}", status, text),
            });
        }

        let data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        let content = data["content"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|c| c["text"].as_str())
            .unwrap_or("")
            .to_string();

        let tool_calls = data["content"].as_array().and_then(|arr| {
            let calls: Vec<ToolCall> = arr
                .iter()
                .filter(|c| c["type"].as_str() == Some("tool_use"))
                .filter_map(|c| {
                    let name = c["name"].as_str()?.to_string();
                    let arguments = c["input"].clone();
                    Some(ToolCall { name, arguments })
                })
                .collect();
            if calls.is_empty() {
                None
            } else {
                Some(calls)
            }
        });

        Ok(CompletionResponse {
            content,
            tool_calls,
        })
    }

    async fn complete_stream(
        &self,
        request: CompletionRequest,
        tx: mpsc::Sender<StreamChunk>,
    ) -> Result<CompletionResponse, String> {
        let messages = self.build_messages(&request);
        let mut body = json!({
            "model": self.model,
            "messages": messages,
            "max_tokens": request.max_tokens.unwrap_or(4096),
            "stream": true,
        });

        if let Some(ref system) = request.system_prompt {
            body["system"] = json!(system);
        }
        if let Some(temp) = request.temperature {
            body["temperature"] = json!(temp);
        }

        let resp = self
            .client
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .timeout(std::time::Duration::from_secs(60))
            .send()
            .await
            .map_err(|e| format!("Anthropic stream failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("Anthropic error {}: {}", status, text));
        }

        let mut stream = resp.bytes_stream();
        let mut full_content = String::new();
        let mut buffer = String::new();

        while let Some(chunk) = stream.next().await {
            let bytes = chunk.map_err(|e| format!("Stream read error: {}", e))?;
            buffer.push_str(&String::from_utf8_lossy(&bytes));

            while let Some(pos) = buffer.find('\n') {
                let line: String = buffer.drain(..=pos).collect();
                let line = line.trim();

                if line.is_empty() {
                    continue;
                }

                if !line.starts_with("data: ") {
                    continue;
                }

                let data_str = &line[6..];
                if let Ok(data) = serde_json::from_str::<serde_json::Value>(data_str) {
                    let event_type = data["type"].as_str().unwrap_or("");

                    match event_type {
                        "content_block_delta" => {
                            let token =
                                data["delta"]["text"].as_str().unwrap_or("");
                            if !token.is_empty() {
                                full_content.push_str(token);
                                let _ = tx
                                    .send(StreamChunk {
                                        content: token.to_string(),
                                        done: false,
                                    })
                                    .await;
                            }
                        }
                        "message_stop" => {
                            let _ = tx
                                .send(StreamChunk {
                                    content: String::new(),
                                    done: true,
                                })
                                .await;
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(CompletionResponse {
            content: full_content,
            tool_calls: None,
        })
    }

    async fn is_healthy(&self) -> bool {
        // Anthropic has no simple health endpoint; verify by checking the key
        self.client
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&json!({
                "model": self.model,
                "max_tokens": 1,
                "messages": [{"role": "user", "content": "hi"}]
            }))
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .map(|r| r.status().is_success() || r.status().as_u16() == 400)
            .unwrap_or(false)
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, String> {
        Ok(vec![
            ModelInfo {
                id: "claude-sonnet-4-20250514".to_string(),
                name: "Claude Sonnet 4".to_string(),
                multimodal: true,
            },
            ModelInfo {
                id: "claude-opus-4-20250514".to_string(),
                name: "Claude Opus 4".to_string(),
                multimodal: true,
            },
        ])
    }

    fn supports_multimodal(&self) -> bool {
        MULTIMODAL_MODELS
            .iter()
            .any(|m| self.model.contains(m))
    }
}
