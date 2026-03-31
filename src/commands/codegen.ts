import { invoke } from "@tauri-apps/api/core";
import type {
  GeneratedSketchResponse,
  SerialToolDefinition,
  ChatResponse,
} from "../types/manifest";

export async function generateSketch(): Promise<GeneratedSketchResponse> {
  return invoke<GeneratedSketchResponse>("generate_sketch");
}

export async function modifySketch(
  instruction: string,
): Promise<GeneratedSketchResponse> {
  return invoke<GeneratedSketchResponse>("modify_sketch", { instruction });
}

export async function approveSketch(sketchCode: string): Promise<void> {
  return invoke("approve_sketch", { sketchCode });
}

export async function rejectSketch(): Promise<void> {
  return invoke("reject_sketch");
}

export async function uploadSketch(
  sketchContent: string,
): Promise<GeneratedSketchResponse> {
  return invoke<GeneratedSketchResponse>("upload_sketch", { sketchContent });
}

export async function getSketch(): Promise<string | null> {
  return invoke<string | null>("get_sketch");
}

export async function getTools(): Promise<SerialToolDefinition[] | null> {
  return invoke<SerialToolDefinition[] | null>("get_tools");
}

export async function sendChatMessage(
  message: string,
): Promise<ChatResponse> {
  return invoke<ChatResponse>("send_chat_message", { message });
}

export async function clearChatHistory(): Promise<void> {
  return invoke("clear_chat_history");
}
