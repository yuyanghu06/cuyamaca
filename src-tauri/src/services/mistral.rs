use super::provider::*;
use futures_util::StreamExt;
use serde_json::json;
use tokio::sync::mpsc;

pub struct MistralProvider {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    model: String,
}

impl MistralProvider {
    pub fn new(model: String, api_key: String, base_url: Option<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url
                .unwrap_or_else(|| "https://api.mistral.ai/v1".to_string()),
            api_key,
            model,
        }
    }

    fn build_messages(&self, request: &CompletionRequest) -> Vec<serde_json::Value> {
        let mut msgs = Vec::new();

        if let Some(ref system) = request.system_prompt {
            msgs.push(json!({ "role": "system", "content": system }));
        }

        for msg in &request.messages {
            match &msg.content {
                MessageContent::Text(text) => {
                    msgs.push(json!({ "role": msg.role, "content": text }));
                }
                MessageContent::Multimodal(parts) => {
                    let text: String = parts
                        .iter()
                        .filter_map(|p| match p {
                            ContentPart::Text { text } => Some(text.as_str()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    msgs.push(json!({ "role": msg.role, "content": text }));
                }
            }
        }

        msgs
    }
}

#[async_trait::async_trait]
impl ModelProvider for MistralProvider {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, String> {
        let messages = self.build_messages(&request);
        let mut body = json!({
            "model": self.model,
            "messages": messages,
        });

        if let Some(temp) = request.temperature {
            body["temperature"] = json!(temp);
        }
        if let Some(max) = request.max_tokens {
            body["max_tokens"] = json!(max);
        }

        let resp = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .timeout(std::time::Duration::from_secs(60))
            .send()
            .await
            .map_err(|e| format!("Mistral request failed: {}", e))?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(match status.as_u16() {
                401 => "Mistral: Invalid API key".to_string(),
                429 => "Mistral: Rate limit exceeded".to_string(),
                _ => format!("Mistral error {}: {}", status, text),
            });
        }

        let data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        let content = data["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(CompletionResponse {
            content,
            tool_calls: None,
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
            "stream": true,
        });

        if let Some(temp) = request.temperature {
            body["temperature"] = json!(temp);
        }
        if let Some(max) = request.max_tokens {
            body["max_tokens"] = json!(max);
        }

        let resp = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .timeout(std::time::Duration::from_secs(60))
            .send()
            .await
            .map_err(|e| format!("Mistral stream failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("Mistral error {}: {}", status, text));
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

                if line.is_empty() || !line.starts_with("data: ") {
                    continue;
                }

                let data_str = &line[6..];
                if data_str == "[DONE]" {
                    let _ = tx
                        .send(StreamChunk {
                            content: String::new(),
                            done: true,
                        })
                        .await;
                    break;
                }

                if let Ok(data) = serde_json::from_str::<serde_json::Value>(data_str) {
                    let token = data["choices"][0]["delta"]["content"]
                        .as_str()
                        .unwrap_or("");
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
            }
        }

        Ok(CompletionResponse {
            content: full_content,
            tool_calls: None,
        })
    }

    async fn is_healthy(&self) -> bool {
        self.client
            .get(format!("{}/models", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, String> {
        Ok(vec![ModelInfo {
            id: "codestral-latest".to_string(),
            name: "Codestral".to_string(),
            multimodal: false,
        }])
    }

    fn supports_multimodal(&self) -> bool {
        false
    }
}
