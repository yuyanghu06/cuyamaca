use crate::models::manifest::{Component, Manifest};
use crate::models::project::{Project, ProjectSummary};
use crate::AppState;
use std::fs;
use std::path::PathBuf;

fn projects_dir() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
    let dir = home.join("cuyamaca-projects");
    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(|e| format!("Failed to create projects directory: {}", e))?;
    }
    Ok(dir)
}

fn validate_project_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Project name cannot be empty".to_string());
    }
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err("Project name can only contain alphanumeric characters, hyphens, and underscores".to_string());
    }
    Ok(())
}

#[tauri::command]
pub fn create_project(name: String, board: String) -> Result<Manifest, String> {
    validate_project_name(&name)?;

    let dir = projects_dir()?.join(&name);
    if dir.exists() {
        return Err(format!("Project '{}' already exists", name));
    }

    fs::create_dir_all(dir.join("history"))
        .map_err(|e| format!("Failed to create project directory: {}", e))?;

    let manifest = Manifest::new(&name, &board);
    let manifest_json = serde_json::to_string_pretty(&manifest)
        .map_err(|e| format!("Failed to serialize manifest: {}", e))?;

    fs::write(dir.join("manifest.json"), manifest_json)
        .map_err(|e| format!("Failed to write manifest: {}", e))?;

    Ok(manifest)
}

#[tauri::command]
pub fn list_projects() -> Result<Vec<ProjectSummary>, String> {
    let dir = projects_dir()?;
    let mut projects = Vec::new();

    let entries = fs::read_dir(&dir).map_err(|e| format!("Failed to read projects directory: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let manifest_path = path.join("manifest.json");
        if !manifest_path.exists() {
            continue;
        }
        let content = fs::read_to_string(&manifest_path).unwrap_or_default();
        if let Ok(manifest) = serde_json::from_str::<Manifest>(&content) {
            projects.push(ProjectSummary {
                name: manifest.project.clone(),
                board: manifest.board.clone(),
                component_count: manifest.components.len(),
            });
        }
    }

    projects.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(projects)
}

#[tauri::command]
pub fn open_project(
    state: tauri::State<'_, AppState>,
    name: String,
) -> Result<Project, String> {
    let dir = projects_dir()?.join(&name);
    if !dir.exists() {
        return Err(format!("Project '{}' not found", name));
    }

    let manifest_path = dir.join("manifest.json");
    let content = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Failed to read manifest: {}", e))?;
    let manifest: Manifest =
        serde_json::from_str(&content).map_err(|e| format!("Invalid manifest: {}", e))?;

    let sketch = {
        let sketch_path = dir.join("sketch.ino");
        if sketch_path.exists() {
            Some(fs::read_to_string(&sketch_path).unwrap_or_default())
        } else {
            None
        }
    };

    let has_tools = dir.join("tools.json").exists();

    let project = Project {
        name: name.clone(),
        path: dir,
        manifest,
        sketch,
        has_tools,
    };

    let mut active = state.active_project.lock().map_err(|e| e.to_string())?;
    *active = Some(project.clone());

    Ok(project)
}

#[tauri::command]
pub fn delete_project(
    state: tauri::State<'_, AppState>,
    name: String,
) -> Result<(), String> {
    let dir = projects_dir()?.join(&name);
    if !dir.exists() {
        return Err(format!("Project '{}' not found", name));
    }

    // Clear active project if this is the one being deleted
    if let Ok(mut active) = state.active_project.lock() {
        if active.as_ref().map(|p| &p.name) == Some(&name) {
            *active = None;
        }
    }

    fs::remove_dir_all(&dir).map_err(|e| format!("Failed to delete project: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn get_projects_path() -> Result<String, String> {
    let dir = projects_dir()?;
    Ok(dir.to_string_lossy().to_string())
}

#[tauri::command]
pub fn get_active_project(
    state: tauri::State<'_, AppState>,
) -> Result<Option<Project>, String> {
    let active = state.active_project.lock().map_err(|e| e.to_string())?;
    Ok(active.clone())
}

fn save_manifest_to_disk(project: &Project) -> Result<(), String> {
    let manifest_json = serde_json::to_string_pretty(&project.manifest)
        .map_err(|e| format!("Failed to serialize manifest: {}", e))?;
    fs::write(project.path.join("manifest.json"), manifest_json)
        .map_err(|e| format!("Failed to write manifest: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn set_board(
    state: tauri::State<'_, AppState>,
    board: String,
) -> Result<(), String> {
    let mut active = state.active_project.lock().map_err(|e| e.to_string())?;
    let project = active.as_mut().ok_or("No active project")?;
    project.manifest.board = board;
    save_manifest_to_disk(project)
}

#[tauri::command]
pub fn set_serial_port(
    state: tauri::State<'_, AppState>,
    port: String,
) -> Result<(), String> {
    let mut active = state.active_project.lock().map_err(|e| e.to_string())?;
    let project = active.as_mut().ok_or("No active project")?;
    project.manifest.serial_port = port;
    save_manifest_to_disk(project)
}

#[tauri::command]
pub fn set_baud_rate(
    state: tauri::State<'_, AppState>,
    baud: u32,
) -> Result<(), String> {
    let mut active = state.active_project.lock().map_err(|e| e.to_string())?;
    let project = active.as_mut().ok_or("No active project")?;
    project.manifest.baud_rate = baud;
    save_manifest_to_disk(project)
}

#[tauri::command]
pub fn add_component(
    state: tauri::State<'_, AppState>,
    component: Component,
) -> Result<(), String> {
    let mut active = state.active_project.lock().map_err(|e| e.to_string())?;
    let project = active.as_mut().ok_or("No active project")?;

    // Check for duplicate IDs
    if project.manifest.components.iter().any(|c| c.id == component.id) {
        return Err(format!("Component with ID '{}' already exists", component.id));
    }

    project.manifest.components.push(component);
    save_manifest_to_disk(project)
}

#[tauri::command]
pub fn update_component(
    state: tauri::State<'_, AppState>,
    id: String,
    component: Component,
) -> Result<(), String> {
    let mut active = state.active_project.lock().map_err(|e| e.to_string())?;
    let project = active.as_mut().ok_or("No active project")?;

    let idx = project
        .manifest
        .components
        .iter()
        .position(|c| c.id == id)
        .ok_or(format!("Component '{}' not found", id))?;

    project.manifest.components[idx] = component;
    save_manifest_to_disk(project)
}

#[tauri::command]
pub fn remove_component(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let mut active = state.active_project.lock().map_err(|e| e.to_string())?;
    let project = active.as_mut().ok_or("No active project")?;

    let len_before = project.manifest.components.len();
    project.manifest.components.retain(|c| c.id != id);
    if project.manifest.components.len() == len_before {
        return Err(format!("Component '{}' not found", id));
    }

    save_manifest_to_disk(project)
}

#[tauri::command]
pub fn list_serial_ports() -> Result<Vec<String>, String> {
    let ports = serialport::available_ports()
        .map_err(|e| format!("Failed to enumerate serial ports: {}", e))?;
    Ok(ports.into_iter().map(|p| p.port_name).collect())
}

#[tauri::command]
pub fn get_component_library() -> Vec<crate::models::component_library::ComponentTemplate> {
    crate::models::component_library::get_component_library()
}
