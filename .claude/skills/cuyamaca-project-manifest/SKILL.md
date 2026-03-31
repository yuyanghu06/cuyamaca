---
name: cuyamaca-project-manifest
description: Build the project system and manifest editor for Cuyamaca — project CRUD, the manifest data model, the parts editor UI with component picker, and pin assignment editing. Use this skill whenever the user wants to create the project system, build the manifest editor, implement the parts panel, add the component picker, set up the hardware definition workflow, or references "phase 3", "project system", "manifest", "parts editor", "component picker", "hardware definition", "pin assignment", or "add component". Also trigger when the user asks about the manifest JSON schema, component types, project file structure, or how to define board configurations. This skill assumes Phase 2 is complete (LLM abstraction layer with provider trait and model slots).
---

# Phase 3 — Project System + Manifest Editor

This skill builds the project system and hardware definition workflow. Users create projects, define their board and components in a manifest, and see everything reflected in the parts panel. The manifest is the ground truth that all later phases (code generation, tool synthesis, runtime control) build from.

## What This Skill Produces

- Project data model and file structure (`manifest.json`, `sketch.ino`, `tools.json`, `history/`)
- Project CRUD operations: create, open, list, delete
- Manifest data model matching the CLAUDE.md schema (board, serial port, baud rate, components)
- Manifest View in the main area: board configuration + component list with inline editing
- Parts Panel (right sidebar): live component list grouped by type
- Component picker modal: browse available component types, select one, configure pins
- Pin assignment editor: inline editing of pin numbers, connection types, labels
- Tauri commands for all project and manifest operations
- File system persistence in a `cuyamaca-projects/` directory

## Prerequisites

- Phase 2 complete: LLM abstraction layer in place
- The three-panel layout from Phase 1 with the Manifest View placeholder ready to be replaced

## Step 1: Define the Data Model

### Manifest types (Rust)

```rust
// src-tauri/src/models/manifest.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub project: String,
    pub board: String,             // e.g., "arduino:avr:uno"
    pub serial_port: String,       // e.g., "/dev/cu.usbmodem14201" or "COM3"
    pub baud_rate: u32,            // default 115200
    pub components: Vec<Component>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    pub id: String,                // snake_case identifier, e.g., "motor_left"
    pub component_type: String,    // e.g., "dc_motor", "ultrasonic", "servo"
    pub pins: HashMap<String, u8>, // pin name → pin number
    pub label: String,             // human-readable name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtype: Option<String>,   // e.g., "esp32-cam"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection: Option<String>, // e.g., "wifi", "i2c", "serial"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}
```

### Project types

```rust
// src-tauri/src/models/project.rs

pub struct Project {
    pub name: String,
    pub path: PathBuf,         // full path to the project directory
    pub manifest: Manifest,
    pub sketch: Option<String>, // current sketch content, if any
    pub has_tools: bool,        // whether tools.json exists
}
```

### Mirror these types in TypeScript

```typescript
// src/types/manifest.ts

export interface Manifest {
  project: string;
  board: string;
  serial_port: string;
  baud_rate: number;
  components: Component[];
}

export interface Component {
  id: string;
  component_type: string;
  pins: Record<string, number>;
  label: string;
  subtype?: string;
  connection?: string;
  resolution?: string;
  format?: string;
}
```

## Step 2: Define the Component Library

The component library is a static registry of all supported component types. Each entry defines: the type name, category (actuator / sensor / vision), required pin names, optional fields, and the expected serial output format.

Store this as a static data structure in Rust:

```rust
// src-tauri/src/models/component_library.rs

pub struct ComponentTemplate {
    pub component_type: &'static str,
    pub category: &'static str,       // "actuator", "distance", "motion", "touch", "environmental", "vision"
    pub label: &'static str,           // display name, e.g., "DC Motor"
    pub pins: &'static [PinTemplate],
    pub optional_fields: &'static [&'static str], // "subtype", "connection", "resolution", "format"
    pub serial_output: Option<&'static str>,       // expected output format description
}

pub struct PinTemplate {
    pub name: &'static str,
    pub description: &'static str,
}
```

Populate with all types from the CLAUDE.md Component Library section:

**Actuators:** dc_motor (pwm, dir_a, dir_b), servo (signal), stepper_motor (step, direction), relay (pin), led (pin)

**Distance:** ultrasonic (trig, echo), ir_distance (analog), lidar_serial (rx, tx)

**Motion:** imu (sda, scl — I2C), magnetometer (sda, scl — I2C), encoder (pin_a, pin_b)

**Touch:** bump_switch (pin), line_sensor_array (pins — variable count), force_sensor (analog)

**Environmental:** temp_humidity (data), barometer (sda, scl — I2C), light (sda, scl — I2C), gas (analog)

**Vision:** camera (no physical pins — wifi connection, needs subtype, resolution, format fields)

Also expose this library to the frontend via a Tauri command so the component picker can display it.

## Step 3: Project File System

Projects live in a `cuyamaca-projects/` directory inside the user's app data directory (use Tauri's `app_data_dir()`).

```
{app_data_dir}/cuyamaca-projects/
  my-robot/
    manifest.json
    sketch.ino          (created in Phase 4)
    tools.json          (created in Phase 4)
    history/            (created in Phase 4)
      sketch_v1.ino
      ...
```

Tauri commands for project CRUD:

```rust
#[tauri::command]
pub fn create_project(name: String, board: String) -> Result<Manifest, String> {
    // Validate name (alphanumeric + hyphens, no spaces)
    // Create directory
    // Create manifest.json with defaults (empty components, 115200 baud)
    // Return the manifest
}

#[tauri::command]
pub fn list_projects() -> Result<Vec<ProjectSummary>, String> {
    // Scan cuyamaca-projects/ directory
    // Read each manifest.json, return name + board + component count
}

#[tauri::command]
pub fn open_project(name: String) -> Result<Project, String> {
    // Read manifest.json, optionally sketch.ino, check for tools.json
    // Set as the active project in app state
}

#[tauri::command]
pub fn delete_project(name: String) -> Result<(), String> {
    // Remove the project directory
}

#[tauri::command]
pub fn save_manifest(state: tauri::State<'_, AppState>) -> Result<(), String> {
    // Write the active project's manifest to disk
}
```

### Active project state

Add to `AppState`:

```rust
pub struct AppState {
    pub active_project: Mutex<Option<Project>>,
    pub model_manager: Mutex<ModelManager>,
    // ... other state
}
```

All manifest mutations (add component, change board, etc.) modify the in-memory project and then persist to disk via `save_manifest`.

## Step 4: Manifest Editing Commands

```rust
#[tauri::command]
pub fn set_board(
    state: tauri::State<'_, AppState>,
    board: String,
) -> Result<(), String>

#[tauri::command]
pub fn set_serial_port(
    state: tauri::State<'_, AppState>,
    port: String,
) -> Result<(), String>

#[tauri::command]
pub fn set_baud_rate(
    state: tauri::State<'_, AppState>,
    baud: u32,
) -> Result<(), String>

#[tauri::command]
pub fn add_component(
    state: tauri::State<'_, AppState>,
    component: Component,
) -> Result<(), String>

#[tauri::command]
pub fn update_component(
    state: tauri::State<'_, AppState>,
    id: String,
    component: Component,
) -> Result<(), String>

#[tauri::command]
pub fn remove_component(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<(), String>

#[tauri::command]
pub fn list_serial_ports() -> Result<Vec<String>, String> {
    // Use the serialport crate to enumerate available ports
    // Returns port names: /dev/cu.* on macOS, COM* on Windows
}

#[tauri::command]
pub fn get_component_library() -> Vec<ComponentTemplate> {
    // Return the static component library for the frontend picker
}
```

For serial port enumeration, add to Cargo.toml:
```toml
serialport = "4"
```

## Step 5: Build the Manifest View

Replace the ManifestView placeholder from Phase 1 with the real editor.

### Layout

```
┌─────────────────────────────────────┐
│  Board Configuration                │
│  ┌────────────────────────────────┐ │
│  │ Board: [arduino:avr:uno    ▼]  │ │
│  │ Port:  [/dev/cu.usbmodem ▼]   │ │
│  │ Baud:  [115200            ▼]   │ │
│  └────────────────────────────────┘ │
│                                     │
│  Components                  [+ Add]│
│  ┌────────────────────────────────┐ │
│  │ ≡ Left Drive Motor    dc_motor │ │
│  │   pwm: 5  dir_a: 2  dir_b: 3  │ │
│  ├────────────────────────────────┤ │
│  │ ≡ Front Distance    ultrasonic │ │
│  │   trig: 9  echo: 10           │ │
│  ├────────────────────────────────┤ │
│  │ ≡ Head Servo          servo    │ │
│  │   signal: 3                    │ │
│  └────────────────────────────────┘ │
└─────────────────────────────────────┘
```

### Board configuration section

Three dropdowns/inputs at the top:

- **Board:** Dropdown with common board identifiers. Allow custom input for boards not in the list. Common values: `arduino:avr:uno`, `arduino:avr:mega`, `arduino:avr:nano`, `arduino:sam:arduino_due_x`, `esp32:esp32:esp32`.
- **Serial Port:** Dropdown populated by `list_serial_ports()`. Include a refresh button to re-enumerate. Show platform-appropriate format (`/dev/cu.*` on macOS, `COM*` on Windows).
- **Baud Rate:** Dropdown with common values: 9600, 19200, 38400, 57600, 115200. Default 115200.

### Component list

Each component renders as a Glass Standard card with:
- A drag handle icon (≡) on the left for future reordering
- The component label (bold) and type (secondary text) on the header row
- Pin assignments displayed below in monospace, editable inline
- An expand/collapse toggle to show/hide the pin editor
- A delete button (subtle, appears on hover)

Clicking a component card expands it to show:
- Label text input
- ID text input (auto-generated from label, editable)
- Pin assignments as labeled number inputs
- Optional fields (subtype, connection, resolution, format) if the component type requires them
- A "Remove" button at the bottom

### Add Component button

Opens the component picker modal.

## Step 6: Build the Component Picker

A modal overlay (Glass Strong) that displays the component library grouped by category.

```
┌─────────────────────────────────────────┐
│  Add Component                     [✕]  │
│                                         │
│  ACTUATORS                              │
│  ┌──────┐  ┌──────┐  ┌──────┐          │
│  │  DC  │  │Servo │  │Stepper│          │
│  │Motor │  │      │  │Motor │          │
│  └──────┘  └──────┘  └──────┘          │
│  ┌──────┐  ┌──────┐                     │
│  │Relay │  │ LED  │                     │
│  └──────┘  └──────┘                     │
│                                         │
│  DISTANCE / PROXIMITY                   │
│  ┌──────┐  ┌──────┐  ┌──────┐          │
│  │Ultra- │  │  IR  │  │LiDAR │          │
│  │sonic  │  │Dist. │  │Serial│          │
│  └──────┘  └──────┘  └──────┘          │
│                                         │
│  ... more categories ...                │
└─────────────────────────────────────────┘
```

Each tile is a Glass Standard card showing the component type name and a small icon (use inline SVGs — no icon fonts). Clicking a tile:

1. Creates a new component with the selected type
2. Auto-generates an ID from the type (e.g., `dc_motor_1`, incrementing if duplicates exist)
3. Sets a default label (e.g., "DC Motor 1")
4. Pre-fills pin fields with empty values (user must assign pins)
5. Closes the modal and scrolls to the new component in the list, expanded for editing

## Step 7: Update the Parts Panel

The right-side Parts Panel mirrors the manifest's component list but in a compact, read-only format grouped by category.

**Grouped display:**
```
ACTUATORS
  Left Drive Motor     pwm:5
  Right Drive Motor    pwm:6
  Head Servo           sig:3

SENSORS
  Front Distance       trig:9
  
VISION
  Forward Camera       wifi
```

Each row shows: component label + one key pin or connection info. Clicking a component in the parts panel scrolls to and expands it in the Manifest View.

When no project is loaded, show "no project loaded" with a "Create Project" button.

## Step 8: Project List in Sidebar

Update the sidebar to show the project list above the navigation items:

```
┌──────────┐
│ CUYAMACA │
│          │
│ PROJECTS │
│  my-robot│  ← active (cyan accent)
│  test-bot│
│  [+ New] │
│          │
│ ─────── │
│ Manifest │  ← nav items
│ Code     │
│ Chat     │
│          │
│ ─────── │
│ ● Ollama │
│ ● CLI    │
│ ● Code   │
│ ● Runtime│
└──────────┘
```

Clicking a project name opens it. The active project is highlighted with a cyan left border. The "+ New" button opens a small inline form (project name + board dropdown) to create a new project.

## Step 9: Verify

1. Create a new project from the sidebar. A project directory appears in the app data folder with a `manifest.json`.
2. The Manifest View shows the board configuration section with the selected board.
3. Serial port dropdown lists available ports on the system.
4. Click "Add Component" — the picker modal opens with all categories.
5. Select "DC Motor" — a new component appears in the list with empty pin fields.
6. Edit the label, ID, and pin assignments. Changes persist to `manifest.json`.
7. Add several components of different types. The Parts Panel groups them by category.
8. Remove a component — it disappears from both the Manifest View and Parts Panel.
9. Close and reopen the app — the project loads from disk with all components intact.
10. Create a second project, switch between them. Each has its own manifest.

## Common Issues

**Serial port enumeration on macOS:** The `serialport` crate may not find ports if the USB driver isn't installed. For Arduino Uno, the built-in driver works. For CH340-based boards, the user needs the CH340 driver.

**Component ID collisions:** Auto-generated IDs must be unique within a project. When creating a component, check existing IDs and increment a suffix if needed.

**File system permissions:** Tauri's `app_data_dir()` should always be writable, but verify on both macOS and Windows.

## What NOT to Do

- Do not build the code editor or sketch generation. The Manifest View is only for hardware definition. Code comes in Phase 4.
- Do not add serial communication logic. Port enumeration is fine, but reading/writing serial data comes in Phase 6.
- Do not add drag-and-drop reordering for components yet. Just include the handle icon as a visual placeholder.
- Do not validate pin conflicts (two components using the same pin) in this phase — add that as a nice-to-have in Phase 8.
- Do not allow editing the manifest from the Parts Panel. It's read-only. Edits happen in the Manifest View.