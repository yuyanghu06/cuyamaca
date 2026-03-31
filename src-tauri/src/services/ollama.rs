use super::provider::*;
use futures_util::StreamExt;
use serde_json::json;
use tokio::sync::mpsc;

const KNOWN_MULTIMODAL_PREFIXES: &[&str] = &[
    "llava", "bakllava", "moondream", "llama3.2-vision", "llama-vision",
];

fn is_multimodal_model(model: &str) -> bool {
    let lower = model.to_lowercase();
    KNOWN_MULTIMODAL_PREFIXES.iter().any(|p| lower.contains(p))
}

pub struct OllamaProvider {
    client: reqwest::Client,
    base_url: String,
    model: String,
}

impl OllamaProvider {
    pub fn new(model: String, base_url: Option<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.unwrap_or_else(|| "http://localhost:11434".to_string()),
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
                    let mut text = String::new();
                    let mut images: Vec<String> = Vec::new();
                    for part in parts {
                        match part {
                            ContentPart::Text { text: t } => text.push_str(t),
                            ContentPart::Image { data, .. } => images.push(data.clone()),
                        }
                    }
                    let mut m = json!({ "role": msg.role, "content": text });
                    if !images.is_empty() {
                        m["images"] = json!(images);
                    }
                    msgs.push(m);
                }
            }
        }

        msgs
    }
}

#[async_trait::async_trait]
impl ModelProvider for OllamaProvider {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, String> {
        let messages = self.build_messages(&request);
        let mut body = json!({
            "model": self.model,
            "messages": messages,
            "stream": false,
        });

        if let Some(temp) = request.temperature {
            body["options"] = json!({ "temperature": temp });
        }

        let resp = self
            .client
            .post(format!("{}/api/chat", self.base_url))
            .json(&body)
            .timeout(std::time::Duration::from_secs(120))
            .send()
            .await
            .map_err(|e| format!("Ollama request failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("Ollama error {}: {}", status, text));
        }

        let data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        let content = data["message"]["content"]
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
            body["options"] = json!({ "temperature": temp });
        }

        let resp = self
            .client
            .post(format!("{}/api/chat", self.base_url))
            .json(&body)
            .timeout(std::time::Duration::from_secs(120))
            .send()
            .await
            .map_err(|e| format!("Ollama stream failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("Ollama error {}: {}", status, text));
        }

        let mut stream = resp.bytes_stream();
        let mut full_content = String::new();
        let mut buffer = Vec::new();

        while let Some(chunk) = stream.next().await {
            let bytes = chunk.map_err(|e| format!("Stream read error: {}", e))?;
            buffer.extend_from_slice(&bytes);

            // Split on newlines — each line is a JSON object
            while let Some(pos) = buffer.iter().position(|&b| b == b'\n') {
                let line: Vec<u8> = buffer.drain(..=pos).collect();
                let line_str = String::from_utf8_lossy(&line);
                let trimmed = line_str.trim();
                if trimmed.is_empty() {
                    continue;
                }

                if let Ok(data) = serde_json::from_str::<serde_json::Value>(trimmed) {
                    let token = data["message"]["content"].as_str().unwrap_or("");
                    let done = data["done"].as_bool().unwrap_or(false);

                    if !token.is_empty() {
                        full_content.push_str(token);
                        let _ = tx
                            .send(StreamChunk {
                                content: token.to_string(),
                                done: false,
                            })
                            .await;
                    }

                    if done {
                        let _ = tx
                            .send(StreamChunk {
                                content: String::new(),
                                done: true,
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
            .get(&self.base_url)
            .timeout(std::time::Duration::from_secs(3))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, String> {
        let resp = self
            .client
            .get(format!("{}/api/tags", self.base_url))
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
            .map_err(|e| format!("Failed to list Ollama models: {}", e))?;

        let data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        let models = data["models"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .map(|m| {
                let name = m["name"].as_str().unwrap_or("unknown").to_string();
                let multimodal = is_multimodal_model(&name);
                ModelInfo {
                    id: name.clone(),
                    name,
                    multimodal,
                }
            })
            .collect();

        Ok(models)
    }

    fn supports_multimodal(&self) -> bool {
        is_multimodal_model(&self.model)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multimodal_llava() {
        assert!(is_multimodal_model("llava"));
        assert!(is_multimodal_model("llava:7b"));
        assert!(is_multimodal_model("llava:13b-v1.6"));
        assert!(is_multimodal_model("llava-llama3"));
    }

    #[test]
    fn test_multimodal_bakllava() {
        assert!(is_multimodal_model("bakllava"));
        assert!(is_multimodal_model("bakllava:7b"));
    }

    #[test]
    fn test_multimodal_moondream() {
        assert!(is_multimodal_model("moondream"));
        assert!(is_multimodal_model("moondream2:latest"));
    }

    #[test]
    fn test_multimodal_llama_vision() {
        assert!(is_multimodal_model("llama3.2-vision"));
        assert!(is_multimodal_model("llama3.2-vision:11b"));
        assert!(is_multimodal_model("llama3.2-vision:90b"));
        assert!(is_multimodal_model("llama-vision:13b"));
    }

    #[test]
    fn test_multimodal_case_insensitive() {
        assert!(is_multimodal_model("LLAVA"));
        assert!(is_multimodal_model("LLaVA:7b"));
        assert!(is_multimodal_model("Moondream2"));
    }

    #[test]
    fn test_text_only_models_not_multimodal() {
        assert!(!is_multimodal_model("mistral"));
        assert!(!is_multimodal_model("codestral:latest"));
        assert!(!is_multimodal_model("llama3:8b"));
        assert!(!is_multimodal_model("phi3:14b"));
        assert!(!is_multimodal_model("gemma:7b"));
        assert!(!is_multimodal_model("qwen2:7b"));
        assert!(!is_multimodal_model("deepseek-coder:6.7b"));
    }

    #[test]
    fn test_empty_string_not_multimodal() {
        assert!(!is_multimodal_model(""));
    }
}
