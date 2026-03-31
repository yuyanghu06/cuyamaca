import { invoke, Channel } from "@tauri-apps/api/core";
import type { SerialEvent, SensorStateSnapshot } from "../types/manifest";

export async function openSerial(): Promise<void> {
  return invoke("open_serial");
}

export async function closeSerial(): Promise<void> {
  return invoke("close_serial");
}

export async function sendSerialCommand(command: string): Promise<void> {
  return invoke("send_serial_command", { command });
}

export async function getSensorState(): Promise<SensorStateSnapshot> {
  return invoke("get_sensor_state");
}

export async function getSensorViz(): Promise<number[] | null> {
  return invoke("get_sensor_viz");
}

export async function subscribeSerial(
  onEvent: (event: SerialEvent) => void,
): Promise<void> {
  const channel = new Channel<SerialEvent>();
  channel.onmessage = onEvent;
  return invoke("subscribe_serial", { onEvent: channel });
}
