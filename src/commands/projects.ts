import { invoke } from "@tauri-apps/api/core";
import type {
  Manifest,
  Component,
  Project,
  ProjectSummary,
  ComponentTemplate,
} from "../types/manifest";

export async function createProject(
  name: string,
  board: string,
): Promise<Manifest> {
  return invoke<Manifest>("create_project", { name, board });
}

export async function listProjects(): Promise<ProjectSummary[]> {
  return invoke<ProjectSummary[]>("list_projects");
}

export async function openProject(name: string): Promise<Project> {
  return invoke<Project>("open_project", { name });
}

export async function deleteProject(name: string): Promise<void> {
  return invoke("delete_project", { name });
}

export async function getProjectsPath(): Promise<string> {
  return invoke<string>("get_projects_path");
}

export async function getActiveProject(): Promise<Project | null> {
  return invoke<Project | null>("get_active_project");
}

export async function setBoard(board: string): Promise<void> {
  return invoke("set_board", { board });
}

export async function setSerialPort(port: string): Promise<void> {
  return invoke("set_serial_port", { port });
}

export async function setBaudRate(baud: number): Promise<void> {
  return invoke("set_baud_rate", { baud });
}

export async function addComponent(component: Component): Promise<void> {
  return invoke("add_component", { component });
}

export async function updateComponent(
  id: string,
  component: Component,
): Promise<void> {
  return invoke("update_component", { id, component });
}

export async function removeComponent(id: string): Promise<void> {
  return invoke("remove_component", { id });
}

export async function listSerialPorts(): Promise<string[]> {
  return invoke<string[]>("list_serial_ports");
}

export async function getComponentLibrary(): Promise<ComponentTemplate[]> {
  return invoke<ComponentTemplate[]>("get_component_library");
}
