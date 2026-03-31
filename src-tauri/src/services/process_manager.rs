use std::sync::Arc;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

#[derive(Debug, Clone, PartialEq)]
pub enum ProcessState {
    Stopped,
    Starting,
    Running,
    Failed(String),
}

pub struct ProcessManager {
    child: Arc<Mutex<Option<Child>>>,
    state: Arc<Mutex<ProcessState>>,
}

impl ProcessManager {
    pub fn new() -> Self {
        Self {
            child: Arc::new(Mutex::new(None)),
            state: Arc::new(Mutex::new(ProcessState::Stopped)),
        }
    }

    pub async fn start_ollama(&self) -> Result<(), String> {
        {
            let state = self.state.lock().await;
            if *state == ProcessState::Running {
                return Ok(());
            }
        }

        *self.state.lock().await = ProcessState::Starting;

        // Check if Ollama is already running (started externally)
        if self.check_ollama_health().await {
            *self.state.lock().await = ProcessState::Running;
            return Ok(());
        }

        // Find the ollama binary
        let ollama_bin = find_ollama_binary().await.ok_or(
            "Ollama binary not found. Install it from Settings.".to_string(),
        )?;

        // Start Ollama serve as a child process
        let child = Command::new(&ollama_bin)
            .arg("serve")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| format!("Failed to start Ollama: {}", e))?;

        *self.child.lock().await = Some(child);

        // Wait for health check (up to 15 seconds)
        for i in 0..30 {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            if self.check_ollama_health().await {
                *self.state.lock().await = ProcessState::Running;
                return Ok(());
            }

            // Check if process exited early
            let mut guard = self.child.lock().await;
            if let Some(ref mut child) = *guard {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        *guard = None;
                        drop(guard);
                        let msg = format!("Ollama exited with status: {}", status);
                        *self.state.lock().await = ProcessState::Failed(msg.clone());
                        return Err(msg);
                    }
                    _ => {}
                }
            }

            if i == 29 {
                *self.state.lock().await = ProcessState::Failed(
                    "Ollama did not become healthy within 15 seconds".to_string(),
                );
                return Err("Ollama startup timed out".to_string());
            }
        }

        Ok(())
    }

    pub async fn stop_ollama(&self) {
        let mut guard = self.child.lock().await;
        if let Some(mut child) = guard.take() {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
        drop(guard);
        *self.state.lock().await = ProcessState::Stopped;
    }

    pub async fn restart_ollama(&self) -> Result<(), String> {
        self.stop_ollama().await;
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        self.start_ollama().await
    }

    pub async fn get_state(&self) -> ProcessState {
        // If we think we're running, verify the child process is still alive
        let state = self.state.lock().await.clone();
        if state == ProcessState::Running {
            let mut guard = self.child.lock().await;
            if let Some(ref mut child) = *guard {
                match child.try_wait() {
                    Ok(Some(_)) => {
                        // Process exited unexpectedly
                        *guard = None;
                        drop(guard);
                        // Check if Ollama is still reachable (maybe restarted externally)
                        if self.check_ollama_health().await {
                            return ProcessState::Running;
                        }
                        *self.state.lock().await =
                            ProcessState::Failed("Ollama process exited unexpectedly".to_string());
                        return ProcessState::Failed(
                            "Ollama process exited unexpectedly".to_string(),
                        );
                    }
                    _ => return ProcessState::Running,
                }
            } else {
                // No child but we thought we were running — check health
                drop(guard);
                if self.check_ollama_health().await {
                    return ProcessState::Running;
                }
                *self.state.lock().await = ProcessState::Stopped;
                return ProcessState::Stopped;
            }
        }
        state
    }

    async fn check_ollama_health(&self) -> bool {
        match reqwest::Client::new()
            .get("http://localhost:11434/")
            .timeout(std::time::Duration::from_secs(2))
            .send()
            .await
        {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }
}

async fn find_ollama_binary() -> Option<String> {
    // Try PATH
    let which_cmd = if cfg!(target_os = "windows") {
        "where"
    } else {
        "which"
    };
    if let Ok(out) = Command::new(which_cmd).arg("ollama").output().await {
        if out.status.success() {
            let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(path);
            }
        }
    }

    // Try common paths
    let candidates = vec![
        #[cfg(target_os = "macos")]
        "/usr/local/bin/ollama",
        #[cfg(target_os = "macos")]
        "/opt/homebrew/bin/ollama",
        #[cfg(target_os = "macos")]
        "/Applications/Ollama.app/Contents/Resources/ollama",
    ];

    for candidate in candidates {
        if std::path::Path::new(candidate).exists() {
            return Some(candidate.to_string());
        }
    }

    None
}
