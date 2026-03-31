import { invoke } from "@tauri-apps/api/core";

export async function ping(message: string): Promise<string> {
  return invoke<string>("ping", { message });
}
