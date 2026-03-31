import { invoke, Channel } from "@tauri-apps/api/core";

export interface ProviderInfo {
  id: string;
  name: string;
  requires_key: boolean;
}

export interface SlotConfig {
  provider: string;
  model: string;
  multimodal_warning: boolean;
}

export interface ModelInfo {
  id: string;
  name: string;
  multimodal: boolean;
}

export async function listProviders(): Promise<ProviderInfo[]> {
  return invoke<ProviderInfo[]>("list_providers");
}

export async function configureModelSlot(
  slot: "code" | "runtime",
  provider: string,
  model: string,
  apiKey?: string,
): Promise<SlotConfig> {
  return invoke<SlotConfig>("configure_model_slot", {
    slot,
    provider,
    model,
    apiKey: apiKey ?? null,
  });
}

export async function getSlotConfig(
  slot: "code" | "runtime",
): Promise<SlotConfig | null> {
  return invoke<SlotConfig | null>("get_slot_config", { slot });
}

export async function checkModelHealth(
  slot: "code" | "runtime",
): Promise<boolean> {
  return invoke<boolean>("check_model_health", { slot });
}

export async function checkOllamaHealth(): Promise<boolean> {
  return invoke<boolean>("check_ollama_health");
}

export async function listOllamaModels(): Promise<ModelInfo[]> {
  return invoke<ModelInfo[]>("list_ollama_models");
}

export async function storeApiKey(
  provider: string,
  key: string,
): Promise<void> {
  return invoke("store_api_key", { provider, key });
}

export async function hasApiKey(provider: string): Promise<boolean> {
  return invoke<boolean>("has_api_key", { provider });
}

// ── Ollama model management ──

export type PullProgress =
  | { event: "started" }
  | { event: "downloading"; data: { completed: number; total: number } }
  | { event: "verifying" }
  | { event: "succeeded" }
  | { event: "failed"; data: { error: string } };

export async function pullOllamaModel(
  model: string,
  onProgress: (event: PullProgress) => void,
): Promise<void> {
  const channel = new Channel<PullProgress>();
  channel.onmessage = onProgress;
  return invoke("pull_ollama_model", { model, onProgress: channel });
}

export async function deleteOllamaModel(model: string): Promise<void> {
  return invoke("delete_ollama_model", { model });
}
