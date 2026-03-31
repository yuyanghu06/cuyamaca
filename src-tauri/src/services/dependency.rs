use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::ipc::Channel;
use tokio::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyStatus {
    pub ollama: DepState,
    pub arduino_cli: DepState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "state", content = "data")]
pub enum DepState {
    Missing,
    Unhealthy { reason: String },
    Installing { progress: f32, message: String },
    Ready { version: String, path: String },
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "event", content = "data")]
pub enum InstallEvent {
    Progress { percent: f32, message: String },
    Extracting { message: String },
    Validating,
    Complete { version: String, path: String },
    Failed { error: String },
}

// ── Detection ──

pub async fn detect_ollama() -> DepState {
    // Try PATH first
    if let Some(state) = try_ollama_at("ollama").await {
        return state;
    }

    // Try common install locations
    for path in ollama_candidate_paths() {
        if path.exists() {
            if let Some(state) = try_ollama_at(path.to_str().unwrap_or("ollama")).await {
                return state;
            }
        }
    }

    DepState::Missing
}

pub async fn detect_arduino_cli() -> DepState {
    // Try PATH first
    if let Some(state) = try_arduino_cli_at("arduino-cli").await {
        return state;
    }

    // Try common install locations
    for path in arduino_cli_candidate_paths() {
        if path.exists() {
            if let Some(state) =
                try_arduino_cli_at(path.to_str().unwrap_or("arduino-cli")).await
            {
                return state;
            }
        }
    }

    DepState::Missing
}

async fn try_ollama_at(bin: &str) -> Option<DepState> {
    match Command::new(bin).arg("--version").output().await {
        Ok(out) if out.status.success() => {
            let version = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let version = version
                .strip_prefix("ollama version ")
                .unwrap_or(&version)
                .to_string();
            let path = resolve_bin_path(bin).await;
            Some(DepState::Ready { version, path })
        }
        Ok(_) => Some(DepState::Unhealthy {
            reason: "ollama found but version check failed".to_string(),
        }),
        Err(_) => None,
    }
}

async fn try_arduino_cli_at(bin: &str) -> Option<DepState> {
    match Command::new(bin).arg("version").output().await {
        Ok(out) if out.status.success() => {
            let raw = String::from_utf8_lossy(&out.stdout).trim().to_string();
            // Typical: "arduino-cli  Version: 0.35.3 ..."
            let version = raw
                .split("Version:")
                .nth(1)
                .map(|v| v.split_whitespace().next().unwrap_or("unknown"))
                .unwrap_or(&raw)
                .to_string();
            let path = resolve_bin_path(bin).await;
            Some(DepState::Ready { version, path })
        }
        Ok(_) => Some(DepState::Unhealthy {
            reason: "arduino-cli found but version check failed".to_string(),
        }),
        Err(_) => None,
    }
}

async fn resolve_bin_path(bin: &str) -> String {
    let which_cmd = if cfg!(target_os = "windows") {
        "where"
    } else {
        "which"
    };
    match Command::new(which_cmd).arg(bin).output().await {
        Ok(out) if out.status.success() => {
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        }
        _ => bin.to_string(),
    }
}

fn ollama_candidate_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    #[cfg(target_os = "macos")]
    {
        paths.push(PathBuf::from("/usr/local/bin/ollama"));
        paths.push(PathBuf::from("/opt/homebrew/bin/ollama"));
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(local_app) = std::env::var("LOCALAPPDATA") {
            paths.push(
                PathBuf::from(&local_app)
                    .join("Programs")
                    .join("Ollama")
                    .join("ollama.exe"),
            );
        }
        paths.push(PathBuf::from(
            "C:\\Program Files\\Ollama\\ollama.exe",
        ));
    }

    paths
}

fn arduino_cli_candidate_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // App data directory (where we install it)
    if let Some(home) = dirs::home_dir() {
        let binary = if cfg!(target_os = "windows") {
            "arduino-cli.exe"
        } else {
            "arduino-cli"
        };
        paths.push(home.join(".cuyamaca").join("bin").join(binary));
    }

    #[cfg(target_os = "macos")]
    {
        paths.push(PathBuf::from("/opt/homebrew/bin/arduino-cli"));
        paths.push(PathBuf::from("/usr/local/bin/arduino-cli"));
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(local_app) = std::env::var("LOCALAPPDATA") {
            paths.push(
                PathBuf::from(&local_app)
                    .join("Arduino CLI")
                    .join("arduino-cli.exe"),
            );
        }
        paths.push(PathBuf::from(
            "C:\\Program Files\\Arduino CLI\\arduino-cli.exe",
        ));
    }

    paths
}

// ── Installation ──

const ARDUINO_CLI_VERSION: &str = "1.1.1";

pub async fn install_ollama(
    _data_dir: &std::path::Path,
    on_event: &Channel<InstallEvent>,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        install_ollama_macos(on_event).await
    }

    #[cfg(target_os = "windows")]
    {
        install_ollama_windows(_data_dir, on_event).await
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = on_event;
        Err("Unsupported platform. Please install Ollama manually from https://ollama.com".into())
    }
}

pub async fn install_arduino_cli(
    data_dir: &std::path::Path,
    on_event: &Channel<InstallEvent>,
) -> Result<(), String> {
    let bin_dir = data_dir.join("bin");
    std::fs::create_dir_all(&bin_dir)
        .map_err(|e| format!("Failed to create bin directory: {}", e))?;

    let (url, archive_name) = get_arduino_cli_url();

    let _ = on_event.send(InstallEvent::Progress {
        percent: 0.0,
        message: format!("Downloading arduino-cli {}...", ARDUINO_CLI_VERSION),
    });

    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Download failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Download failed with status {}", resp.status()));
    }

    let total = resp.content_length().unwrap_or(0);
    let archive_path = data_dir.join(&archive_name);

    // Stream download with progress
    use futures_util::StreamExt;
    let mut stream = resp.bytes_stream();
    let mut downloaded: u64 = 0;
    let mut file = std::fs::File::create(&archive_path)
        .map_err(|e| format!("Failed to create archive file: {}", e))?;

    use std::io::Write;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Download error: {}", e))?;
        file.write_all(&chunk)
            .map_err(|e| format!("Write error: {}", e))?;
        downloaded += chunk.len() as u64;
        if total > 0 {
            let percent = (downloaded as f32 / total as f32) * 80.0; // 80% for download
            let _ = on_event.send(InstallEvent::Progress {
                percent,
                message: format!(
                    "Downloading... {:.1} MB / {:.1} MB",
                    downloaded as f64 / 1_048_576.0,
                    total as f64 / 1_048_576.0
                ),
            });
        }
    }
    drop(file);

    let _ = on_event.send(InstallEvent::Extracting {
        message: "Extracting arduino-cli...".to_string(),
    });

    // Extract
    extract_arduino_cli(&archive_path, &bin_dir)?;

    // Clean up archive
    let _ = std::fs::remove_file(&archive_path);

    let _ = on_event.send(InstallEvent::Validating);

    // Validate
    let binary_name = if cfg!(target_os = "windows") {
        "arduino-cli.exe"
    } else {
        "arduino-cli"
    };
    let binary_path = bin_dir.join(binary_name);

    if !binary_path.exists() {
        let _ = on_event.send(InstallEvent::Failed {
            error: "Binary not found after extraction".to_string(),
        });
        return Err("Binary not found after extraction".into());
    }

    // Make executable on unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&binary_path, std::fs::Permissions::from_mode(0o755));
    }

    match Command::new(&binary_path).arg("version").output().await {
        Ok(out) if out.status.success() => {
            let raw = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let version = raw
                .split("Version:")
                .nth(1)
                .map(|v| v.split_whitespace().next().unwrap_or("unknown"))
                .unwrap_or(&raw)
                .to_string();
            let _ = on_event.send(InstallEvent::Complete {
                version,
                path: binary_path.to_string_lossy().to_string(),
            });
            Ok(())
        }
        _ => {
            let _ = on_event.send(InstallEvent::Failed {
                error: "Installed binary failed version check".to_string(),
            });
            Err("Installed binary failed version check".into())
        }
    }
}

fn get_arduino_cli_url() -> (String, String) {
    let base = format!(
        "https://github.com/arduino/arduino-cli/releases/download/v{}/arduino-cli_{}",
        ARDUINO_CLI_VERSION, ARDUINO_CLI_VERSION
    );

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        (format!("{}_macOS_ARM64.tar.gz", base), "arduino-cli.tar.gz".to_string())
    }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        (format!("{}_macOS_64bit.tar.gz", base), "arduino-cli.tar.gz".to_string())
    }
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        (format!("{}_Windows_64bit.zip", base), "arduino-cli.zip".to_string())
    }
    #[cfg(not(any(
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "windows", target_arch = "x86_64"),
    )))]
    {
        (format!("{}_Linux_64bit.tar.gz", base), "arduino-cli.tar.gz".to_string())
    }
}

fn extract_arduino_cli(
    archive_path: &std::path::Path,
    dest_dir: &std::path::Path,
) -> Result<(), String> {
    let ext = archive_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    if ext == "gz" || archive_path.to_string_lossy().ends_with(".tar.gz") {
        extract_tar_gz(archive_path, dest_dir)
    } else if ext == "zip" {
        extract_zip(archive_path, dest_dir)
    } else {
        Err(format!("Unknown archive format: {}", archive_path.display()))
    }
}

fn extract_tar_gz(path: &std::path::Path, dest: &std::path::Path) -> Result<(), String> {
    let file =
        std::fs::File::open(path).map_err(|e| format!("Failed to open archive: {}", e))?;
    let gz = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(gz);

    for entry in archive
        .entries()
        .map_err(|e| format!("Failed to read archive entries: {}", e))?
    {
        let mut entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let entry_path = entry
            .path()
            .map_err(|e| format!("Invalid path in archive: {}", e))?;

        // Only extract the binary
        let name = entry_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        if name == "arduino-cli" || name == "arduino-cli.exe" {
            let dest_path = dest.join(name);
            let mut out = std::fs::File::create(&dest_path)
                .map_err(|e| format!("Failed to create file: {}", e))?;
            std::io::copy(&mut entry, &mut out)
                .map_err(|e| format!("Failed to extract: {}", e))?;
            // Read remaining to drop cleanly
            let mut sink = std::io::sink();
            let _ = std::io::copy(&mut entry, &mut sink);
        }
    }
    Ok(())
}

fn extract_zip(path: &std::path::Path, dest: &std::path::Path) -> Result<(), String> {
    let file =
        std::fs::File::open(path).map_err(|e| format!("Failed to open archive: {}", e))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("Failed to read zip: {}", e))?;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("Failed to read zip entry: {}", e))?;

        let name = entry
            .enclosed_name()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
            .unwrap_or_default();

        if name == "arduino-cli" || name == "arduino-cli.exe" {
            let dest_path = dest.join(&name);
            let mut out = std::fs::File::create(&dest_path)
                .map_err(|e| format!("Failed to create file: {}", e))?;
            std::io::copy(&mut entry, &mut out)
                .map_err(|e| format!("Failed to extract: {}", e))?;
        }
    }
    Ok(())
}

// ── Ollama platform-specific installers ──

#[cfg(target_os = "macos")]
async fn install_ollama_macos(on_event: &Channel<InstallEvent>) -> Result<(), String> {
    let _ = on_event.send(InstallEvent::Progress {
        percent: 0.0,
        message: "Downloading Ollama for macOS...".to_string(),
    });

    let url = "https://ollama.com/download/Ollama-darwin.zip";
    let client = reqwest::Client::new();
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Download failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Download failed with status {}", resp.status()));
    }

    let total = resp.content_length().unwrap_or(0);
    let tmp_path = std::env::temp_dir().join("Ollama-darwin.zip");

    use futures_util::StreamExt;
    use std::io::Write;
    let mut stream = resp.bytes_stream();
    let mut downloaded: u64 = 0;
    let mut file = std::fs::File::create(&tmp_path)
        .map_err(|e| format!("Failed to create temp file: {}", e))?;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Download error: {}", e))?;
        file.write_all(&chunk)
            .map_err(|e| format!("Write error: {}", e))?;
        downloaded += chunk.len() as u64;
        if total > 0 {
            let percent = (downloaded as f32 / total as f32) * 70.0;
            let _ = on_event.send(InstallEvent::Progress {
                percent,
                message: format!(
                    "Downloading... {:.1} MB / {:.1} MB",
                    downloaded as f64 / 1_048_576.0,
                    total as f64 / 1_048_576.0,
                ),
            });
        }
    }
    drop(file);

    let _ = on_event.send(InstallEvent::Extracting {
        message: "Extracting Ollama.app...".to_string(),
    });

    // Unzip to /Applications
    let output = Command::new("unzip")
        .args(["-o", tmp_path.to_str().unwrap(), "-d", "/Applications/"])
        .output()
        .await
        .map_err(|e| format!("Extraction failed: {}", e))?;

    let _ = std::fs::remove_file(&tmp_path);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Extraction failed: {}", stderr));
    }

    let _ = on_event.send(InstallEvent::Validating);

    // The Ollama.app includes the CLI at a known path
    let ollama_cli = "/Applications/Ollama.app/Contents/Resources/ollama";
    if std::path::Path::new(ollama_cli).exists() {
        // Symlink into /usr/local/bin if possible
        let _ = std::fs::create_dir_all("/usr/local/bin");
        let _ = std::fs::remove_file("/usr/local/bin/ollama");
        let _ = std::os::unix::fs::symlink(ollama_cli, "/usr/local/bin/ollama");
    }

    // Verify
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    match Command::new("ollama").arg("--version").output().await {
        Ok(out) if out.status.success() => {
            let version = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let version = version
                .strip_prefix("ollama version ")
                .unwrap_or(&version)
                .to_string();
            let _ = on_event.send(InstallEvent::Complete {
                version,
                path: ollama_cli.to_string(),
            });
            Ok(())
        }
        _ => {
            let _ = on_event.send(InstallEvent::Complete {
                version: "installed".to_string(),
                path: ollama_cli.to_string(),
            });
            Ok(())
        }
    }
}

#[cfg(target_os = "windows")]
async fn install_ollama_windows(
    _data_dir: &std::path::Path,
    on_event: &Channel<InstallEvent>,
) -> Result<(), String> {
    let _ = on_event.send(InstallEvent::Progress {
        percent: 0.0,
        message: "Downloading Ollama for Windows...".to_string(),
    });

    let url = "https://ollama.com/download/OllamaSetup.exe";
    let client = reqwest::Client::new();
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Download failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Download failed with status {}", resp.status()));
    }

    let total = resp.content_length().unwrap_or(0);
    let tmp_path = std::env::temp_dir().join("OllamaSetup.exe");

    use futures_util::StreamExt;
    use std::io::Write;
    let mut stream = resp.bytes_stream();
    let mut downloaded: u64 = 0;
    let mut file = std::fs::File::create(&tmp_path)
        .map_err(|e| format!("Failed to create temp file: {}", e))?;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Download error: {}", e))?;
        file.write_all(&chunk)
            .map_err(|e| format!("Write error: {}", e))?;
        downloaded += chunk.len() as u64;
        if total > 0 {
            let percent = (downloaded as f32 / total as f32) * 80.0;
            let _ = on_event.send(InstallEvent::Progress {
                percent,
                message: format!(
                    "Downloading... {:.1} MB / {:.1} MB",
                    downloaded as f64 / 1_048_576.0,
                    total as f64 / 1_048_576.0,
                ),
            });
        }
    }
    drop(file);

    let _ = on_event.send(InstallEvent::Extracting {
        message: "Running Ollama installer...".to_string(),
    });

    // Run silent install
    let output = Command::new(&tmp_path)
        .arg("/S")
        .output()
        .await
        .map_err(|e| format!("Installer failed: {}", e))?;

    let _ = std::fs::remove_file(&tmp_path);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Installation failed: {}", stderr));
    }

    let _ = on_event.send(InstallEvent::Validating);

    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    match Command::new("ollama").arg("--version").output().await {
        Ok(out) if out.status.success() => {
            let version = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let _ = on_event.send(InstallEvent::Complete {
                version,
                path: "ollama".to_string(),
            });
            Ok(())
        }
        _ => {
            let _ = on_event.send(InstallEvent::Complete {
                version: "installed".to_string(),
                path: "ollama".to_string(),
            });
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arduino_cli_url_generation() {
        let (url, name) = get_arduino_cli_url();
        assert!(url.contains("arduino-cli"));
        assert!(url.contains(ARDUINO_CLI_VERSION));
        assert!(!name.is_empty());
    }
}
