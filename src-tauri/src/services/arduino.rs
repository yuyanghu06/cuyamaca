use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::process::Command;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedBoard {
    pub port: String,
    pub fqbn: Option<String>,
    pub board_name: Option<String>,
    pub protocol: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CompileResult {
    pub binary_size: u64,
    pub max_size: u64,
}

pub struct ArduinoService {
    cli_path: RwLock<Option<PathBuf>>,
}

impl ArduinoService {
    pub fn new() -> Self {
        Self {
            cli_path: RwLock::new(None),
        }
    }

    pub async fn detect(&self) -> Result<bool, String> {
        // First check if we already have a stored path
        if let Some(ref path) = *self.cli_path.read().await {
            if path.exists() {
                return Ok(true);
            }
        }

        // Try PATH
        match Command::new("arduino-cli")
            .arg("version")
            .output()
            .await
        {
            Ok(out) if out.status.success() => {
                *self.cli_path.write().await = Some(PathBuf::from("arduino-cli"));
                Ok(true)
            }
            _ => {
                // Try common install locations
                let candidates = get_candidate_paths();
                for candidate in candidates {
                    if candidate.exists() {
                        match Command::new(&candidate)
                            .arg("version")
                            .output()
                            .await
                        {
                            Ok(out) if out.status.success() => {
                                *self.cli_path.write().await = Some(candidate);
                                return Ok(true);
                            }
                            _ => continue,
                        }
                    }
                }
                Ok(false)
            }
        }
    }

    #[allow(unreachable_code)]
    pub async fn install(&self) -> Result<(), String> {
        let install_dir = get_install_dir()?;
        std::fs::create_dir_all(&install_dir)
            .map_err(|e| format!("Failed to create install directory: {}", e))?;

        let binary_name = if cfg!(target_os = "windows") {
            "arduino-cli.exe"
        } else {
            "arduino-cli"
        };
        let target_path = install_dir.join(binary_name);

        #[cfg(target_os = "macos")]
        {
            // Try brew first
            let brew_result = Command::new("brew")
                .args(["install", "arduino-cli"])
                .output()
                .await;

            match brew_result {
                Ok(out) if out.status.success() => {
                    *self.cli_path.write().await = Some(PathBuf::from("arduino-cli"));
                    return Ok(());
                }
                _ => {
                    // Fall back to direct download
                    let output = Command::new("curl")
                        .args([
                            "-fsSL",
                            "https://raw.githubusercontent.com/arduino/arduino-cli/master/install.sh",
                            "-o",
                            "/tmp/arduino-cli-install.sh",
                        ])
                        .output()
                        .await
                        .map_err(|e| format!("Failed to download installer: {}", e))?;

                    if !output.status.success() {
                        return Err("Failed to download arduino-cli installer".to_string());
                    }

                    let output = Command::new("sh")
                        .args([
                            "/tmp/arduino-cli-install.sh",
                            "-d",
                            install_dir.to_str().unwrap_or("/tmp"),
                        ])
                        .output()
                        .await
                        .map_err(|e| format!("Failed to run installer: {}", e))?;

                    if !output.status.success() {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        return Err(format!("arduino-cli installation failed: {}", stderr));
                    }
                }
            }

            // Verify the curl/sh installation
            if target_path.exists() {
                *self.cli_path.write().await = Some(target_path);
                return Ok(());
            }
            if self.detect().await.unwrap_or(false) {
                return Ok(());
            }
            return Err("Installation completed but arduino-cli binary not found".to_string());
        }

        #[cfg(target_os = "windows")]
        {
            let output = Command::new("winget")
                .args(["install", "Arduino.ArduinoCLI", "--silent"])
                .output()
                .await;

            match output {
                Ok(out) if out.status.success() => {
                    *self.cli_path.write().await = Some(PathBuf::from("arduino-cli"));
                    return Ok(());
                }
                _ => {
                    return Err(
                        "Failed to install arduino-cli. Please install it manually from https://arduino.github.io/arduino-cli/installation/"
                            .to_string(),
                    );
                }
            }
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        return Err("Unsupported platform for auto-install. Please install arduino-cli manually.".to_string());

        #[allow(unreachable_code)]
        Ok(())
    }

    pub async fn list_boards(&self) -> Result<Vec<DetectedBoard>, String> {
        let output = self.run_cli(&["board", "list", "--format", "json"]).await?;
        parse_board_list(&output)
    }

    pub async fn ensure_core_installed(&self, fqbn: &str) -> Result<(), String> {
        let core = extract_core_from_fqbn(fqbn)?;

        // Check if already installed
        let installed = self.run_cli(&["core", "list", "--format", "json"]).await?;
        if core_is_installed(&installed, &core) {
            return Ok(());
        }

        // Handle ESP32 board manager URL
        if core.starts_with("esp32:") {
            let _ = self
                .run_cli(&[
                    "config",
                    "add",
                    "board_manager.additional_urls",
                    "https://raw.githubusercontent.com/espressif/arduino-esp32/gh-pages/package_esp32_index.json",
                ])
                .await;
        }

        // Update index and install
        self.run_cli(&["core", "update-index"]).await?;
        self.run_cli(&["core", "install", &core]).await?;

        Ok(())
    }

    pub async fn compile_and_flash(
        &self,
        sketch_content: &str,
        fqbn: &str,
        port: &str,
    ) -> Result<CompileResult, String> {
        // Ensure core is installed
        self.ensure_core_installed(fqbn).await?;

        // Create temp directory with sketch
        let temp_dir = std::env::temp_dir().join("cuyamaca-flash");
        let sketch_dir = temp_dir.join("sketch");
        std::fs::create_dir_all(&sketch_dir)
            .map_err(|e| format!("Failed to create temp directory: {}", e))?;

        let sketch_path = sketch_dir.join("sketch.ino");
        std::fs::write(&sketch_path, sketch_content)
            .map_err(|e| format!("Failed to write sketch: {}", e))?;

        // Compile
        let compile_output = self
            .run_cli(&[
                "compile",
                "--fqbn",
                fqbn,
                sketch_dir.to_str().unwrap(),
            ])
            .await
            .map_err(|e| {
                // Clean up on failure
                let _ = std::fs::remove_dir_all(&temp_dir);
                format!("Compilation failed:\n{}", e)
            })?;

        let result = parse_compile_output(&compile_output);

        // Upload
        self.run_cli(&[
            "upload",
            "--fqbn",
            fqbn,
            "--port",
            port,
            sketch_dir.to_str().unwrap(),
        ])
        .await
        .map_err(|e| {
            let _ = std::fs::remove_dir_all(&temp_dir);
            format!("Upload failed:\n{}", e)
        })?;

        // Clean up
        let _ = std::fs::remove_dir_all(&temp_dir);

        Ok(result)
    }

    async fn run_cli(&self, args: &[&str]) -> Result<String, String> {
        let cli_path = self
            .cli_path
            .read()
            .await
            .clone()
            .ok_or_else(|| "arduino-cli not configured".to_string())?;

        let output = Command::new(&cli_path)
            .args(args)
            .output()
            .await
            .map_err(|e| format!("Failed to run arduino-cli: {}", e))?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            Ok(stdout)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let msg = if stderr.is_empty() { stdout } else { stderr };
            Err(msg)
        }
    }
}

fn get_install_dir() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
    Ok(home.join(".cuyamaca").join("bin"))
}

fn get_candidate_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Ok(install_dir) = get_install_dir() {
        let binary = if cfg!(target_os = "windows") {
            "arduino-cli.exe"
        } else {
            "arduino-cli"
        };
        paths.push(install_dir.join(binary));
    }

    #[cfg(target_os = "macos")]
    {
        paths.push(PathBuf::from("/opt/homebrew/bin/arduino-cli"));
        paths.push(PathBuf::from("/usr/local/bin/arduino-cli"));
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(local_app) = std::env::var("LOCALAPPDATA") {
            paths.push(PathBuf::from(local_app).join("Arduino CLI").join("arduino-cli.exe"));
        }
        paths.push(PathBuf::from("C:\\Program Files\\Arduino CLI\\arduino-cli.exe"));
    }

    paths
}

fn extract_core_from_fqbn(fqbn: &str) -> Result<String, String> {
    let parts: Vec<&str> = fqbn.splitn(3, ':').collect();
    if parts.len() < 2 {
        return Err(format!("Invalid FQBN: {}", fqbn));
    }
    Ok(format!("{}:{}", parts[0], parts[1]))
}

fn core_is_installed(json_output: &str, core_id: &str) -> bool {
    // Parse the JSON output from `arduino-cli core list --format json`
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(json_output) {
        // Handle both array format and object-with-platforms format
        let platforms = if let Some(arr) = val.as_array() {
            arr.clone()
        } else if let Some(arr) = val.get("platforms").and_then(|p| p.as_array()) {
            arr.clone()
        } else {
            return false;
        };

        for platform in platforms {
            if let Some(id) = platform.get("id").and_then(|i| i.as_str()) {
                if id == core_id {
                    return true;
                }
            }
        }
    }
    false
}

fn parse_board_list(json_output: &str) -> Result<Vec<DetectedBoard>, String> {
    let val: serde_json::Value =
        serde_json::from_str(json_output).map_err(|e| format!("Failed to parse board list: {}", e))?;

    let mut boards = Vec::new();

    // arduino-cli board list --format json returns an array of detected ports
    let ports = if let Some(arr) = val.as_array() {
        arr.clone()
    } else if let Some(arr) = val.get("detected_ports").and_then(|d| d.as_array()) {
        arr.clone()
    } else {
        return Ok(boards);
    };

    for port_entry in ports {
        let port_obj = port_entry
            .get("port")
            .or_else(|| Some(&port_entry));

        let address = port_obj
            .and_then(|p| p.get("address"))
            .and_then(|a| a.as_str())
            .unwrap_or("")
            .to_string();

        let protocol = port_obj
            .and_then(|p| p.get("protocol"))
            .and_then(|p| p.as_str())
            .unwrap_or("serial")
            .to_string();

        if address.is_empty() || protocol != "serial" {
            continue;
        }

        // Extract matching board info
        let matching = port_entry
            .get("matching_boards")
            .and_then(|m| m.as_array())
            .and_then(|arr| arr.first());

        let fqbn = matching
            .and_then(|b| b.get("fqbn"))
            .and_then(|f| f.as_str())
            .map(|s| s.to_string());

        let board_name = matching
            .and_then(|b| b.get("name"))
            .and_then(|n| n.as_str())
            .map(|s| s.to_string());

        boards.push(DetectedBoard {
            port: address,
            fqbn,
            board_name,
            protocol,
        });
    }

    Ok(boards)
}

fn parse_compile_output(output: &str) -> CompileResult {
    // Try to extract binary size from compile output
    // Typical line: "Sketch uses 3464 bytes (10%) of program storage space. Maximum is 32256 bytes."
    let mut binary_size = 0u64;
    let mut max_size = 0u64;

    for line in output.lines() {
        if line.contains("Sketch uses") && line.contains("bytes") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            for (i, part) in parts.iter().enumerate() {
                if *part == "uses" {
                    if let Some(size_str) = parts.get(i + 1) {
                        binary_size = size_str.parse().unwrap_or(0);
                    }
                }
                if *part == "Maximum" {
                    if let Some(size_str) = parts.get(i + 2) {
                        max_size = size_str.parse().unwrap_or(0);
                    }
                }
            }
        }
    }

    CompileResult {
        binary_size,
        max_size,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── extract_core_from_fqbn ──

    #[test]
    fn test_extract_core_arduino_avr() {
        assert_eq!(
            extract_core_from_fqbn("arduino:avr:uno").unwrap(),
            "arduino:avr"
        );
    }

    #[test]
    fn test_extract_core_esp32() {
        assert_eq!(
            extract_core_from_fqbn("esp32:esp32:esp32-wroom-32").unwrap(),
            "esp32:esp32"
        );
    }

    #[test]
    fn test_extract_core_two_parts() {
        assert_eq!(
            extract_core_from_fqbn("arduino:samd").unwrap(),
            "arduino:samd"
        );
    }

    #[test]
    fn test_extract_core_invalid() {
        assert!(extract_core_from_fqbn("invalid").is_err());
    }

    // ── core_is_installed ──

    #[test]
    fn test_core_installed_array_format() {
        let json = r#"[{"id": "arduino:avr", "installed": "1.8.6"}]"#;
        assert!(core_is_installed(json, "arduino:avr"));
    }

    #[test]
    fn test_core_installed_object_format() {
        let json = r#"{"platforms": [{"id": "arduino:avr", "installed": "1.8.6"}]}"#;
        assert!(core_is_installed(json, "arduino:avr"));
    }

    #[test]
    fn test_core_not_installed() {
        let json = r#"[{"id": "arduino:avr"}]"#;
        assert!(!core_is_installed(json, "esp32:esp32"));
    }

    #[test]
    fn test_core_installed_empty_json() {
        assert!(!core_is_installed("[]", "arduino:avr"));
    }

    #[test]
    fn test_core_installed_invalid_json() {
        assert!(!core_is_installed("not json", "arduino:avr"));
    }

    // ── parse_board_list ──

    #[test]
    fn test_parse_board_list_detected_ports() {
        let json = r#"{
            "detected_ports": [
                {
                    "port": {"address": "/dev/cu.usbmodem14201", "protocol": "serial"},
                    "matching_boards": [{"fqbn": "arduino:avr:uno", "name": "Arduino Uno"}]
                }
            ]
        }"#;
        let boards = parse_board_list(json).unwrap();
        assert_eq!(boards.len(), 1);
        assert_eq!(boards[0].port, "/dev/cu.usbmodem14201");
        assert_eq!(boards[0].fqbn, Some("arduino:avr:uno".into()));
        assert_eq!(boards[0].board_name, Some("Arduino Uno".into()));
    }

    #[test]
    fn test_parse_board_list_array_format() {
        let json = r#"[
            {
                "port": {"address": "COM3", "protocol": "serial"},
                "matching_boards": [{"fqbn": "arduino:avr:mega", "name": "Arduino Mega"}]
            }
        ]"#;
        let boards = parse_board_list(json).unwrap();
        assert_eq!(boards.len(), 1);
        assert_eq!(boards[0].port, "COM3");
    }

    #[test]
    fn test_parse_board_list_filters_non_serial() {
        let json = r#"[
            {
                "port": {"address": "/dev/ttyUSB0", "protocol": "serial"},
                "matching_boards": []
            },
            {
                "port": {"address": "192.168.1.1", "protocol": "network"},
                "matching_boards": []
            }
        ]"#;
        let boards = parse_board_list(json).unwrap();
        assert_eq!(boards.len(), 1);
        assert_eq!(boards[0].port, "/dev/ttyUSB0");
    }

    #[test]
    fn test_parse_board_list_empty() {
        let boards = parse_board_list("[]").unwrap();
        assert!(boards.is_empty());
    }

    #[test]
    fn test_parse_board_list_no_matching_boards() {
        let json = r#"[
            {
                "port": {"address": "/dev/ttyACM0", "protocol": "serial"},
                "matching_boards": []
            }
        ]"#;
        let boards = parse_board_list(json).unwrap();
        assert_eq!(boards.len(), 1);
        assert!(boards[0].fqbn.is_none());
        assert!(boards[0].board_name.is_none());
    }

    // ── parse_compile_output ──

    #[test]
    fn test_parse_compile_standard() {
        let output = "Sketch uses 3464 bytes (10%) of program storage space. Maximum is 32256 bytes.";
        let result = parse_compile_output(output);
        assert_eq!(result.binary_size, 3464);
        assert_eq!(result.max_size, 32256);
    }

    #[test]
    fn test_parse_compile_multiline() {
        let output = "Compiling core...\nLinking everything together...\nSketch uses 9842 bytes (30%) of program storage space. Maximum is 32256 bytes.\nGlobal variables use 342 bytes.";
        let result = parse_compile_output(output);
        assert_eq!(result.binary_size, 9842);
        assert_eq!(result.max_size, 32256);
    }

    #[test]
    fn test_parse_compile_no_size_info() {
        let output = "Compiling sketch...\nDone.";
        let result = parse_compile_output(output);
        assert_eq!(result.binary_size, 0);
        assert_eq!(result.max_size, 0);
    }
}
