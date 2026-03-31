---
name: cuyamaca-installer
description: Build the native installers (macOS .dmg, Windows .exe/.msi) and the in-app first-run dependency installation wizard for Cuyamaca, the Tauri v2 Arduino control app. Use this skill whenever the user asks to build installers, set up distribution, create a .dmg or .msi, implement the first-run experience, auto-install Ollama or arduino-cli, build a setup wizard, handle dependency detection, manage child processes for Ollama, or anything related to packaging, bundling, or distributing Cuyamaca. Also trigger for "first launch", "onboarding flow", "dependency check", "auto-install", "process lifecycle", or "app distribution". This skill covers both the Tauri bundler configuration for producing native installers AND the Rust/React code for the in-app dependency wizard that runs on first launch.
---

# Cuyamaca Installer & Dependency Wizard Skill

This skill covers two tightly coupled concerns:

1. **Native installers** — configuring Tauri v2's bundler to produce `.dmg` (macOS) and `.exe`/`.msi` (Windows) installers.
2. **First-run dependency wizard** — the in-app flow that detects, downloads, installs, and validates Ollama and arduino-cli on the user's machine.

Read the project's `CLAUDE.md` before implementing. The architecture, process lifecycle, and platform requirements there are authoritative.

## Bundled References

- `references/ci-signing.md` — GitHub Actions release workflow and code signing setup for macOS and Windows. Read this when setting up CI/CD or preparing for production distribution.
- `references/process-manager.md` — Full Rust implementation for managing Ollama as a child process (start, health-check, graceful shutdown, crash recovery) and invoking arduino-cli on demand. Read this when implementing the process lifecycle.

## Prerequisites

- A working Tauri v2 project that builds and runs
- Rust backend structure from earlier phases (commands, services, state)
- React frontend with routing or view switching in place

---

## Part 1: Native Installers

### How Tauri Bundling Works

Tauri v2's built-in bundler produces platform-native installers. No electron-builder or NSIS scripts needed. Run `npm run tauri build` and it outputs the correct format for the current OS in `src-tauri/target/release/bundle/`.

You can only build for the OS you're on. Use CI (GitHub Actions) for cross-platform releases — see `references/ci-signing.md`.

### Tauri Configuration

In `src-tauri/tauri.conf.json`, configure the `bundle` section:

```json
{
  "bundle": {
    "active": true,
    "targets": "all",
    "identifier": "com.cuyamaca.app",
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ],
    "macOS": {
      "minimumSystemVersion": "10.15",
      "dmg": {
        "appPosition": { "x": 180, "y": 170 },
        "applicationFolderPosition": { "x": 480, "y": 170 },
        "windowSize": { "width": 660, "height": 400 }
      },
      "signingIdentity": null
    },
    "windows": {
      "nsis": {
        "installMode": "currentUser"
      }
    }
  }
}
```

Key decisions: `identifier` is a stable reverse-domain string (changing it later breaks updates). `minimumSystemVersion` is `"10.15"` for Tauri v2. `installMode: "currentUser"` avoids requiring admin on Windows. `signingIdentity: null` for development — see `references/ci-signing.md` for production signing.

### Icon Generation

```bash
npm run tauri icon path/to/source-icon.png
```

Source image: 1024x1024+ PNG with transparency. This generates `.icns`, `.ico`, and all required PNG sizes.

### Building

**macOS:** `npm run tauri build` → outputs `.dmg` in `src-tauri/target/release/bundle/dmg/`

**Windows:** `npm run tauri build` → outputs `.exe` (NSIS) and `.msi` in `src-tauri/target/release/bundle/nsis/` and `msi/`

---

## Part 2: First-Run Dependency Wizard

### Architecture

```
App launches → Rust checks deps → all healthy? → main app
                                → any missing? → wizard view
                                                    │
                                    user clicks Install → Rust downloads + installs
                                                    │
                                    all healthy → wizard dismisses → main app
```

The frontend renders progress and status. The Rust backend does all detection, downloading, installation, and validation. The frontend never downloads files or checks paths.

### Dependency Status Model

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyStatus {
    pub ollama: DepState,
    pub arduino_cli: DepState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DepState {
    Missing,
    Unhealthy { reason: String },
    Installing { progress: f32, message: String },
    Ready { version: String, path: String },
}
```

### Detection Logic

Detection is platform-specific. For each dependency, the Rust backend: (1) checks PATH, (2) checks common install locations, (3) validates with a version command.

**Ollama:**
- PATH check: `which ollama` (macOS) / `where ollama` (Windows)
- Fallback paths: `/usr/local/bin/ollama`, `/opt/homebrew/bin/ollama` (macOS); `%LOCALAPPDATA%\Programs\Ollama\ollama.exe`, `C:\Program Files\Ollama\ollama.exe` (Windows)
- Validation: `ollama --version` + optional health check at `http://localhost:11434/`

**arduino-cli:**
- PATH check: `which arduino-cli` / `where arduino-cli`
- Fallback paths: `/usr/local/bin/arduino-cli`, `/opt/homebrew/bin/arduino-cli` (macOS); `C:\Program Files\Arduino CLI\arduino-cli.exe` (Windows)
- Validation: `arduino-cli version`

If found and version check passes → `DepState::Ready`. If binary found but version fails → `DepState::Unhealthy`. If not found anywhere → `DepState::Missing`.

### Installation Strategy

Always use direct binary downloads. Never use Homebrew, winget, or any package manager — the user may not have them.

| Dependency | macOS | Windows |
|---|---|---|
| Ollama | Download `.dmg` from ollama.com, mount with `hdiutil`, copy `Ollama.app` to `/Applications/` | Download `.exe` installer from ollama.com, run silently (`/S` flag) |
| arduino-cli | Download tarball from arduino.cc, extract to app data directory | Download zip from arduino.cc, extract to app data directory |

arduino-cli is a single binary — no installer needed. Extract it into the Tauri app data directory (`app.path().app_data_dir()`) so it doesn't pollute the system.

### Download with Progress Streaming

Use `reqwest` streaming + Tauri Channels to report progress:

```rust
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "event", content = "data")]
pub enum InstallEvent {
    Progress { percent: f32, message: String },
    Extracting { message: String },
    Validating,
    Complete { version: String, path: String },
    Failed { error: String },
}

#[tauri::command]
pub async fn install_dependency(
    dep: String,
    on_event: Channel<InstallEvent>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    match dep.as_str() {
        "ollama" => install_ollama(&data_dir, &on_event).await,
        "arduino-cli" => install_arduino_cli(&data_dir, &on_event).await,
        _ => Err(format!("Unknown dependency: {}", dep)),
    }
}
```

The download function streams bytes, tracks progress against `content_length`, and sends `InstallEvent::Progress` through the channel. After download, send `Extracting`, then `Validating`, then `Complete` or `Failed`.

**Extraction crates:**
- `.tar.gz` (macOS): `flate2` + `tar`
- `.zip` (Windows): `zip`
- `.dmg` (macOS Ollama): shell out to `hdiutil attach` / `hdiutil detach`
- `.exe` (Windows Ollama): run installer with `/S` silent flag

### Post-Install Validation

After installation, run the version command to confirm the binary works. If it fails, return `InstallEvent::Failed` with a descriptive error. If it succeeds, return `InstallEvent::Complete` with the version string and binary path.

### Tauri Commands

Three commands for the wizard:

```rust
/// Check all dependencies, return their status
#[tauri::command]
pub async fn check_dependencies(app: tauri::AppHandle) -> Result<DependencyStatus, String>

/// Install a specific dependency with progress streaming
#[tauri::command]
pub async fn install_dependency(dep: String, on_event: Channel<InstallEvent>, app: tauri::AppHandle) -> Result<(), String>

/// User chose to skip — mark wizard complete, load main app with degraded indicators
#[tauri::command]
pub async fn skip_dependency_setup() -> Result<(), String>
```

### Frontend Wizard View

The wizard renders when `check_dependencies` reports any `Missing` state. It follows the project's design language from CLAUDE.md.

**Layout:**

```
┌─────────────────────────────────────────┐
│         [App icon / wordmark]           │
│    Welcome to Cuyamaca                  │
│                                         │
│    ┌─────────────────────────────┐      │
│    │  ✓  Ollama         Ready    │      │
│    │     v0.3.14                 │      │
│    ├─────────────────────────────┤      │
│    │  ✕  arduino-cli   Missing   │      │
│    │     [Install]               │      │
│    └─────────────────────────────┘      │
│                                         │
│    [ Skip — I'll set up manually ]      │
└─────────────────────────────────────────┘
```

**Each dependency row has five states:**

1. **Checking** — spinner, "Checking..." (while `check_dependencies` is in flight)
2. **Ready** — green checkmark, version, path
3. **Missing** — red X, [Install] button
4. **Installing** — progress bar from `InstallEvent::Progress`, status text, button disabled
5. **Failed** — red X, error message, [Retry] button

**Flow:**

1. On mount, call `check_dependencies`. Show Checking state for each row.
2. Results arrive. Ready deps show green. Missing deps show Install buttons.
3. User clicks Install. Call `install_dependency` with a Tauri Channel. Progress streams in.
4. On Complete → flip to Ready. On Failed → show error + Retry.
5. When ALL deps are Ready → show [Continue] button → dismiss wizard → load main app.
6. [Skip] link at bottom → dismiss immediately. Sidebar shows degraded status for missing deps. User configures paths manually in Settings.

**Persistence:** After wizard completes (all Ready or Skip), store a flag in a config file in the app data directory so it doesn't re-appear on next launch:

```json
{ "setup_complete": true, "ollama_path": "...", "arduino_cli_path": "..." }
```

On subsequent launches, still run `check_dependencies` silently. If a dep disappears, show degraded sidebar indicators — don't re-launch the wizard. The wizard is first-run only.

### Process Management

After installation/detection, the app manages Ollama as a child process (start on launch, stop on exit). See `references/process-manager.md` for the full implementation including the `ProcessManager` struct, Tauri lifecycle hooks, health polling, and crash recovery.

arduino-cli is not a long-running process. It's invoked on demand for compile + flash.

---

## Integration Checklist

1. `npm run tauri build` produces `.dmg` on macOS and `.exe`/`.msi` on Windows
2. Installing from the output puts the app in the correct location
3. First launch shows the wizard if Ollama or arduino-cli is missing
4. Detection correctly identifies installed deps and their versions
5. Installation downloads and installs each dep with progress feedback
6. Post-install validation confirms the dep works
7. Subsequent launches skip the wizard
8. Ollama auto-starts on launch and auto-stops on exit
9. No orphaned Ollama processes after app closes
10. Skip path works — app loads with degraded indicators
11. Settings allows manual override of dependency paths

## File Structure

```
src-tauri/src/
├── commands/
│   └── setup.rs            # check_dependencies, install_dependency, skip_dependency_setup
├── services/
│   ├── process_manager.rs  # Ollama child process lifecycle
│   └── dependency.rs       # Detection + installation logic
src/
├── views/
│   └── SetupWizard.tsx     # First-run wizard UI
├── components/
│   └── DependencyRow.tsx   # Individual dep status/install row
```

## What NOT to Do

- Do not use Homebrew, winget, or any package manager for auto-installation. Direct binary downloads only.
- Do not download deps during the Tauri build step. They're installed at runtime on the user's machine.
- Do not require admin/root for dep installation. arduino-cli goes in app data. Ollama's installer may request elevation on its own.
- Do not block the main thread during downloads. Everything is async with progress streaming.
- Do not hardcode download URLs. Store them in a config constant so they're updatable.
- Do not install without user consent. The wizard waits for the user to click Install.
- Do not manage arduino-cli as a long-running process. It's invoked on demand and exits.
- Do not show the wizard on every launch. First-run only. Sidebar handles ongoing health.
- Do not attempt cross-platform builds on a single machine. Use CI.
- Do not embed Ollama or arduino-cli into the app binary. That's a future roadmap item.