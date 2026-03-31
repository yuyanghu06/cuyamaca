import { invoke, Channel } from "@tauri-apps/api/core";
import type { AgentEvent } from "../types/manifest";

export async function openRuntimeWindow(): Promise<void> {
  return invoke("open_runtime_window");
}

export async function runtimeSendMessage(
  message: string,
  onEvent: (event: AgentEvent) => void,
): Promise<void> {
  const channel = new Channel<AgentEvent>();
  channel.onmessage = onEvent;
  return invoke("runtime_send_message", { message, onEvent: channel });
}

export async function runtimeKill(): Promise<void> {
  return invoke("runtime_kill");
}

export async function closeRuntimeWindow(): Promise<void> {
  return invoke("close_runtime_window");
}
