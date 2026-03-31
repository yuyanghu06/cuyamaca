use super::provider::*;
use futures_util::StreamExt;
use serde_json::json;
use tokio::sync::mpsc;

pub struct GoogleProvider {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    model: String,
}

impl GoogleProvider {
    pub fn new(model: String, api_key: String, base_url: Option<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.unwrap_or_else(|| {
                "https://generativelanguage.googleapis.com/v1beta".to_string()
            }),
            api_key,
            model,
        }
    }

    fn build_contents(&self, request: &CompletionRequest) -> Vec<serde_json::Value> {
        request
            .messages
            .iter()
            .map(|msg| {
                let role = match msg.role.as_str() {
                    "assistant" => "model",
                    other => other,
                };
                let parts: Vec<serde_json::Value> = match &msg.content {
                    MessageContent::Text(text) => {
                        vec![json!({ "text": text })]
                    }
                    MessageContent::Multimodal(content_parts) => content_parts
                        .iter()
                        .map(|part| match part {
                            ContentPart::Text { text } => json!({ "text": text }),
                            ContentPart::Image { data, media_type } => json!({
                                "inlineData": {
                                    "mimeType": media_type,
                                    "data": data,
                                }
                            }),
                        })
                        .collect(),
                };
                json!({ "role": role, "parts": parts })
            })
            .collect()
    }
}

#[async_trait::async_trait]
impl ModelProvider for GoogleProvider {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, String> {
        let contents = self.build_contents(&request);
        let mut body = json!({ "contents": contents });

        if let Some(ref system) = request.system_prompt {
            body["systemInstruction"] = json!({
                "parts": [{ "text": system }]
            });
        }

        let mut generation_config = json!({});
        if let Some(temp) = request.temperature {
            generation_config["temperature"] = json!(temp);
        }
        if let Some(max) = request.max_tokens {
            generation_config["maxOutputTokens"] = json!(max);
        }
        if generation_config != json!({}) {
            body["generationConfig"] = generation_config;
        }

        let resp = self
            .client
            .post(format!(
                "{}/models/{}:generateContent?key={}",
                self.base_url, self.model, self.api_key
            ))
            .json(&body)
            .timeout(std::time::Duration::from_secs(60))
            .send()
            .await
            .map_err(|e| format!("Google request failed: {}", e))?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(match status.as_u16() {
                401 | 403 => "Google: Invalid API key".to_string(),
                429 => "Google: Rate limit exceeded".to_string(),
                _ => format!("Google error {}: {}", status, text),
            });
        }

        let data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        let content = data["candidates"][0]["content"]["parts"][0]["text"]
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
        let contents = self.build_contents(&request);
        let mut body = json!({ "contents": contents });

        if let Some(ref system) = request.system_prompt {
            body["systemInstruction"] = json!({
                "parts": [{ "text": system }]
            });
        }

        let resp = self
            .client
            .post(format!(
                "{}/models/{}:streamGenerateContent?key={}&alt=sse",
                self.base_url, self.model, self.api_key
            ))
            .json(&body)
            .timeout(std::time::Duration::from_secs(60))
            .send()
            .await
            .map_err(|e| format!("Google stream failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("Google error {}: {}", status, text));
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
                if let Ok(data) = serde_json::from_str::<serde_json::Value>(data_str) {
                    let token = data["candidates"][0]["content"]["parts"][0]["text"]
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

        let _ = tx
            .send(StreamChunk {
                content: String::new(),
                done: true,
            })
            .await;

        Ok(CompletionResponse {
            content: full_content,
            tool_calls: None,
        })
    }

    async fn is_healthy(&self) -> bool {
        self.client
            .get(format!(
                "{}/models/{}?key={}",
                self.base_url, self.model, self.api_key
            ))
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, String> {
        Ok(vec![
            ModelInfo {
                id: "gemini-1.5-pro".to_string(),
                name: "Gemini 1.5 Pro".to_string(),
                multimodal: true,
            },
            ModelInfo {
                id: "gemini-2.0-flash".to_string(),
                name: "Gemini 2.0 Flash".to_string(),
                multimodal: true,
            },
        ])
    }

    fn supports_multimodal(&self) -> bool {
        true // All Gemini models support multimodal
    }
}
