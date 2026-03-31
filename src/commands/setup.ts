import { invoke, Channel } from "@tauri-apps/api/core";

export interface DependencyStatus {
  ollama: DepState;
  arduinoCli: DepState;
}

export type DepState =
  | { state: "missing" }
  | { state: "unhealthy"; data: { reason: string } }
  | { state: "installing"; data: { progress: number; message: string } }
  | { state: "ready"; data: { version: string; path: string } };

export type InstallEvent =
  | { event: "progress"; data: { percent: number; message: string } }
  | { event: "extracting"; data: { message: string } }
  | { event: "validating" }
  | { event: "complete"; data: { version: string; path: string } }
  | { event: "failed"; data: { error: string } };

export async function checkDependencies(): Promise<DependencyStatus> {
  return invoke("check_dependencies");
}

export async function installDependency(
  dep: string,
  onEvent: (event: InstallEvent) => void,
): Promise<void> {
  const channel = new Channel<InstallEvent>();
  channel.onmessage = onEvent;
  return invoke("install_dependency", { dep, onEvent: channel });
}

export async function skipDependencySetup(): Promise<void> {
  return invoke("skip_dependency_setup");
}

export async function markSetupComplete(): Promise<void> {
  return invoke("mark_setup_complete");
}

export async function isSetupComplete(): Promise<boolean> {
  return invoke("is_setup_complete");
}

export async function startOllama(): Promise<void> {
  return invoke("start_ollama");
}

export async function stopOllama(): Promise<void> {
  return invoke("stop_ollama");
}

export async function restartOllama(): Promise<void> {
  return invoke("restart_ollama");
}

export async function getOllamaProcessState(): Promise<string> {
  return invoke("get_ollama_process_state");
}
