import { invoke, Channel } from "@tauri-apps/api/core";
import type { DetectedBoard, FlashEvent } from "../types/manifest";

export async function detectArduinoCli(): Promise<boolean> {
  return invoke<boolean>("detect_arduino_cli");
}

export async function installArduinoCli(): Promise<void> {
  return invoke("install_arduino_cli");
}

export async function detectBoards(): Promise<DetectedBoard[]> {
  return invoke<DetectedBoard[]>("detect_boards");
}

export async function flashSketch(
  onEvent: (event: FlashEvent) => void,
): Promise<void> {
  const channel = new Channel<FlashEvent>();
  channel.onmessage = onEvent;
  return invoke("flash_sketch", { onEvent: channel });
}
