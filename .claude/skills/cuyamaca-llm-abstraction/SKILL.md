---
name: cuyamaca-llm-abstraction
description: Build the multi-provider LLM abstraction layer for Cuyamaca with two independent model slots — a code model and a runtime model. Use this skill whenever the user wants to add LLM provider support, integrate Ollama or external APIs (OpenAI, Anthropic, Google, Mistral), set up the two-model-slot architecture, implement streaming chat with multiple providers, or references "phase 2", "LLM abstraction", "model providers", "code model", "runtime model", "multi-provider", or "Ollama integration" in the context of Cuyamaca. Also trigger when the user asks about supporting multiple LLM backends, API key storage in OS keychain, or model selection logic. This skill assumes Phase 1 is complete (Tauri v2 scaffold, IPC bridge verified, three-panel layout).
---

# Phase 2 — Multi-Provider LLM Abstraction

This skill builds the LLM service layer for Cuyamaca. Unlike Sierra (which only talks to Ollama), Cuyamaca supports multiple LLM providers and has two independent model slots: a code model for sketch generation/modification and a runtime model for agentic hardware control. Each slot can be configured to use any supported provider.

## What This Skill Produces

- A `ModelProvider` trait in Rust with `complete` (non-streaming) and `complete_stream` (streaming) methods
- Provider implementations: Ollama, OpenAI, Anthropic, Google, Mistral
- A `ModelSlot` abstraction representing a configured model (provider + model name + API key)
- Two independently configurable slots in app state: `code_model` and `runtime_model`
- Tauri commands for listing available models, checking provider health, and sending completions
- API key storage via the OS keychain (Tauri's secure storage plugin)
- Frontend command wrappers for all LLM operations

## Prerequisites

- Phase 1 complete: Tauri v2 project builds, IPC verified, layout skeleton in place
- Ollama installed and running locally (for testing the Ollama provider)
- At least one external API key (for testing external providers — optional but recommended)

## Step 1: Add Rust Dependencies

Add to `src-tauri/Cargo.toml`:

```toml
reqwest = { version = "0.12", features = ["json", "stream"] }
futures-util = "0.3"
tokio = { version = "1", features = ["full"] }
serde_json = "1"
async-trait = "0.1"
tauri-plugin-store = "2"
```

`tauri-plugin-store` is used for persisting model slot configuration (which provider, which model). API keys go through the OS keychain via Tauri's secure storage, not the store plugin.

For keychain access, add the Tauri stronghold or keyring plugin depending on your preference. The simplest approach for v2:

```toml
keyring = "3"
```

The `keyring` crate provides cross-platform OS keychain access (macOS Keychain, Windows Credential Manager).

## Step 2: Define the Provider Trait

Create the file structure:

```
src-tauri/src/
├── services/
│   ├── mod.rs
│   ├── provider.rs        # ModelProvider trait + shared types
│   ├── ollama.rs           # Ollama implementation
│   ├── openai.rs           # OpenAI implementation
│   ├── anthropic.rs        # Anthropic implementation
│   ├── google.rs           # Google Gemini implementation
│   ├── mistral.rs          # Mistral implementation
│   └── model_manager.rs   # ModelSlot, slot management, factory
```

### Shared types (`services/provider.rs`)

```rust
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,       // "user", "assistant", "system"
    pub content: MessageContent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Multimodal(Vec<ContentPart>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { data: String, media_type: String },  // base64
}

#[derive(Debug, Clone, Serialize)]
pub struct StreamChunk {
    pub content: String,
    pub done: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    pub messages: Vec<ChatMessage>,
    pub system_prompt: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub tools: Option<Vec<ToolDefinition>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value, // JSON Schema
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub content: String,
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub arguments: serde_json::Value,
}

#[async_trait::async_trait]
pub trait ModelProvider: Send + Sync {
    /// Non-streaming completion. Used by the code model for sketch generation.
    async fn complete(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse, String>;

    /// Streaming completion. Used by the runtime model and chat views.
    async fn complete_stream(
        &self,
        request: CompletionRequest,
        tx: mpsc::Sender<StreamChunk>,
    ) -> Result<CompletionResponse, String>;

    /// Check if the provider is reachable and the configured model exists.
    async fn is_healthy(&self) -> bool;

    /// List available models for this provider.
    async fn list_models(&self) -> Result<Vec<ModelInfo>, String>;

    /// Whether this provider supports multimodal input (images).
    fn supports_multimodal(&self) -> bool;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub multimodal: bool,
}
```

Key design decisions:

- `MessageContent` is an enum so the same trait handles text-only (code model) and multimodal (runtime model with camera frames and sensor visualizations).
- `CompletionResponse` returns both text content and optional tool calls. The code model uses text content for sketch generation. The runtime model uses tool calls for serial commands.
- `complete` is non-streaming — the code model doesn't need token-by-token display since the user reviews the full output as a diff.
- `complete_stream` is streaming — the runtime model streams responses in the chat view.
- `supports_multimodal()` is used by the settings UI to filter models for the runtime slot (only multimodal models are valid for runtime).

## Step 3: Implement the Ollama Provider

`services/ollama.rs` — hits Ollama's native `/api/chat` endpoint.

Key details:

**Struct:**
```rust
pub struct OllamaProvider {
    client: reqwest::Client,
    base_url: String,
    model: String,
}
```

**`complete` implementation:**
Post to `/api/chat` with `"stream": false`. Parse the single JSON response.

**`complete_stream` implementation:**
Post to `/api/chat` with `"stream": true`. Read NDJSON line by line. Each line is:
```json
{"model":"llama3.2","message":{"role":"assistant","content":"token"},"done":false}
```
Final line has `"done": true`. Buffer bytes and split on `\n` to handle partial lines.

**`list_models` implementation:**
GET `/api/tags` returns `{"models": [{"name": "llama3.2", ...}]}`. Map to `ModelInfo`. Mark models as multimodal based on known multimodal model families (llava, bakllava, moondream, llama vision).

**`is_healthy` implementation:**
GET `/` — 200 means running.

**Multimodal support:**
Ollama's `/api/chat` accepts `"images": ["base64..."]` in message objects. When the request contains `ContentPart::Image`, convert to Ollama's image format. `supports_multimodal()` returns true only if the configured model is in the known multimodal list.

## Step 4: Implement External API Providers

Each provider follows the same pattern: translate `CompletionRequest` into the provider's API format, make the HTTP call, translate the response back to `CompletionResponse`.

### OpenAI (`services/openai.rs`)

- Endpoint: `https://api.openai.com/v1/chat/completions`
- Auth: `Authorization: Bearer {api_key}`
- Streaming: SSE with `data: {"choices":[{"delta":{"content":"token"}}]}` lines
- Multimodal: GPT-4o accepts `{"type": "image_url", "image_url": {"url": "data:image/jpeg;base64,..."}}` in content arrays
- `supports_multimodal()`: true for gpt-4o models

### Anthropic (`services/anthropic.rs`)

- Endpoint: `https://api.anthropic.com/v1/messages`
- Auth: `x-api-key: {api_key}`, `anthropic-version: 2023-06-01`
- System prompt goes in the top-level `system` field, not in messages
- Streaming: SSE with `event: content_block_delta` containing `{"delta":{"text":"token"}}`
- Multimodal: accepts `{"type": "image", "source": {"type": "base64", "media_type": "image/jpeg", "data": "..."}}` in content arrays
- `supports_multimodal()`: true for claude-sonnet and claude-opus models

### Google Gemini (`services/google.rs`)

- Endpoint: `https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent`
- Auth: `?key={api_key}` query parameter
- Streaming: use `:streamGenerateContent` endpoint with SSE
- Multimodal: accepts `{"inlineData": {"mimeType": "image/jpeg", "data": "base64..."}}` in parts
- `supports_multimodal()`: true for gemini-1.5-pro and gemini-2.0-flash

### Mistral (`services/mistral.rs`)

- Endpoint: `https://api.mistral.ai/v1/chat/completions`
- Auth: `Authorization: Bearer {api_key}`
- Streaming: SSE, same format as OpenAI
- Multimodal: Codestral is text-only
- `supports_multimodal()`: false

### Shared patterns

All external providers share:
- An `api_key: String` field stored in the struct
- Error handling that distinguishes between network errors, auth errors (401), and rate limits (429)
- A reqwest client with a 60-second timeout for completions

Factor common SSE parsing into a shared utility if the implementations are too repetitive.

## Step 5: Build the Model Manager

`services/model_manager.rs` manages the two model slots and creates provider instances.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotConfig {
    pub provider: ProviderType,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProviderType {
    Ollama,
    OpenAI,
    Anthropic,
    Google,
    Mistral,
}

pub struct ModelManager {
    code_model: Option<Box<dyn ModelProvider>>,
    runtime_model: Option<Box<dyn ModelProvider>>,
    code_config: Option<SlotConfig>,
    runtime_config: Option<SlotConfig>,
}
```

The `ModelManager` provides:

- `configure_code_model(config: SlotConfig, api_key: Option<String>)` — creates the appropriate provider instance and stores it
- `configure_runtime_model(config: SlotConfig, api_key: Option<String>)` — same, but validates that the provider+model supports multimodal. Warns (does not block) if the model is text-only.
- `code_model()` — returns a reference to the code model provider, or an error if not configured
- `runtime_model()` — returns a reference to the runtime model provider, or an error if not configured
- `factory(provider: ProviderType, model: String, api_key: Option<String>, base_url: Option<String>) -> Box<dyn ModelProvider>` — constructs the right provider implementation

## Step 6: API Key Storage

API keys are stored in the OS keychain, not in plaintext config files.

Use the `keyring` crate:

```rust
use keyring::Entry;

fn store_api_key(provider: &str, key: &str) -> Result<(), String> {
    let entry = Entry::new("cuyamaca", provider).map_err(|e| e.to_string())?;
    entry.set_password(key).map_err(|e| e.to_string())?;
    Ok(())
}

fn get_api_key(provider: &str) -> Result<Option<String>, String> {
    let entry = Entry::new("cuyamaca", provider).map_err(|e| e.to_string())?;
    match entry.get_password() {
        Ok(key) => Ok(Some(key)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}
```

Service names: `"cuyamaca-openai"`, `"cuyamaca-anthropic"`, `"cuyamaca-google"`, `"cuyamaca-mistral"`.

Slot configurations (which provider, which model) are persisted via `tauri-plugin-store` to a JSON file. API keys never go in that file.

## Step 7: Create Tauri Commands

```rust
// commands/models.rs

#[tauri::command]
pub async fn list_providers() -> Vec<ProviderInfo> {
    // Return the static list of supported providers with their model lists
}

#[tauri::command]
pub async fn configure_model_slot(
    state: tauri::State<'_, AppState>,
    slot: String,          // "code" or "runtime"
    provider: String,
    model: String,
    api_key: Option<String>,
) -> Result<(), String> {
    // Store API key in keychain if provided
    // Create provider instance via factory
    // Update the appropriate slot in ModelManager
    // Persist slot config to store
}

#[tauri::command]
pub async fn get_slot_config(
    state: tauri::State<'_, AppState>,
    slot: String,
) -> Result<Option<SlotConfigResponse>, String> {
    // Return current config (provider + model, NOT the API key)
}

#[tauri::command]
pub async fn check_model_health(
    state: tauri::State<'_, AppState>,
    slot: String,
) -> Result<bool, String> {
    // Call is_healthy() on the appropriate slot's provider
}

#[tauri::command]
pub async fn list_ollama_models(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<ModelInfo>, String> {
    // Hit Ollama's /api/tags to list locally available models
}
```

Do NOT add a `send_completion` command yet. The code model completion is used in Phase 4 (code generation) and the runtime model completion is used in Phase 7 (agent loop). This phase only sets up the abstraction and configuration.

## Step 8: Frontend Command Wrappers

```typescript
// src/commands/models.ts

export interface SlotConfig {
  provider: "ollama" | "openai" | "anthropic" | "google" | "mistral";
  model: string;
}

export interface ModelInfo {
  id: string;
  name: string;
  multimodal: boolean;
}

export async function configureModelSlot(
  slot: "code" | "runtime",
  provider: string,
  model: string,
  apiKey?: string,
): Promise<void> {
  return invoke("configure_model_slot", { slot, provider, model, apiKey });
}

export async function getSlotConfig(
  slot: "code" | "runtime",
): Promise<SlotConfig | null> {
  return invoke("get_slot_config", { slot });
}

export async function checkModelHealth(
  slot: "code" | "runtime",
): Promise<boolean> {
  return invoke("check_model_health", { slot });
}

export async function listOllamaModels(): Promise<ModelInfo[]> {
  return invoke("list_ollama_models");
}
```

## Step 9: Wire Health Checks to Sidebar

Update the sidebar service health indicators from Phase 1:

- **Ollama:** Call Ollama's health check (GET `/`). Green if reachable, red if not.
- **Code Model:** Check if the code slot is configured and healthy. Green if configured + healthy, amber if configured but unhealthy, red if not configured.
- **Runtime Model:** Same logic as code model.
- **arduino-cli:** Leave as red for now — gets wired in Phase 5.

Poll health every 30 seconds or on window focus.

## Step 10: Verify

1. The app builds and runs
2. Sidebar health dots update: Ollama green (if running), code/runtime model red (not yet configured)
3. `listOllamaModels()` returns the locally available models
4. `configureModelSlot("code", "ollama", "llama3.2")` succeeds
5. After configuring, `getSlotConfig("code")` returns the config
6. `checkModelHealth("code")` returns true
7. If you have an external API key, `configureModelSlot("code", "openai", "gpt-4o", "sk-...")` succeeds and the key is stored in the OS keychain (verify via Keychain Access on macOS or Credential Manager on Windows)

## Common Issues

**Keychain permission prompt on macOS:** The first time the app accesses the keychain, macOS may prompt the user to allow access. This is expected behavior.

**Ollama multimodal model detection:** There's no reliable API to check if an Ollama model is multimodal. Maintain a hardcoded list of known multimodal model family prefixes (`llava`, `bakllava`, `moondream`, `llama-vision`). This is imperfect but sufficient.

**External API rate limits:** All external providers have rate limits. Return clear error messages when a 429 is received so the frontend can display it.

**reqwest TLS on Windows:** Ensure `reqwest` uses `native-tls` or `rustls` features appropriately for Windows compatibility.

## What NOT to Do

- Do not build the chat UI or send completions to models. This phase is infrastructure only — the providers are called in Phases 4 and 7.
- Do not store API keys in plaintext files, environment variables, or the Tauri store. Use the OS keychain exclusively.
- Do not hardcode API endpoints. Store them in the provider structs so they're configurable (useful for self-hosted OpenAI-compatible endpoints).
- Do not skip the multimodal capability check for the runtime slot. Text-only models in the runtime slot will silently drop camera frames and sensor visualizations — the user must be warned.
- Do not block on external API calls. All provider methods are async. Use tokio::spawn for long-running completions.
- Do not add tool calling execution logic. The `ToolDefinition` and `ToolCall` types are defined here for the trait, but the actual tool registry and execution loop come in later phases.