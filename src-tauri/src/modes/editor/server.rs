use std::sync::Mutex;
use tauri::Manager;
use tokio::process::Command;

pub struct VscodeServer {
    process: Mutex<Option<tokio::process::Child>>,
    port: Mutex<u16>,
}

impl VscodeServer {
    pub fn new() -> Self {
        Self { process: Mutex::new(None), port: Mutex::new(8420) }
    }

    pub async fn start(&self, project_path: &str) -> Result<u16, String> {
        let mut port = 8420u16;
        let mut child = None;
        for attempt in 0..5 {
            let test_port = port + attempt;
            let check = std::process::Command::new("sh")
                .args(["-c", &format!("lsof -i :{} -P 2>/dev/null | grep -q LISTEN && echo in-use || echo free", test_port)])
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "in-use")
                .unwrap_or(false);
            if check { continue; }

            // Check if `code` command exists
            let which = std::process::Command::new("which").arg("code").output();
            if which.map(|o| !o.status.success()).unwrap_or(true) {
                return Err("VS Code not found. Install VS Code and add `code` to PATH.".to_string());
            }

            let cmd = Command::new("code")
                .args([
                    "--serve-web",
                    "--port", &test_port.to_string(),
                    "--without-connection-token",
                    "--accept-server-license-terms",
                    project_path,
                ])
                .kill_on_drop(true)
                .spawn()
                .map_err(|e| format!("Failed to start VS Code server: {}", e))?;

            child = Some(cmd);
            port = test_port;
            break;
        }
        let child = child.ok_or_else(|| "All ports 8420-8424 are in use".to_string())?;
        *self.process.lock().unwrap() = Some(child);
        *self.port.lock().unwrap() = port;
        Ok(port)
    }

    pub fn stop(&self) {
        if let Some(mut child) = self.process.lock().unwrap().take() {
            let _ = child.start_kill();
        }
    }

    pub fn port(&self) -> u16 {
        *self.port.lock().unwrap()
    }
}

impl Drop for VscodeServer {
    fn drop(&mut self) {
        self.stop();
    }
}

#[tauri::command]
pub fn editor_get_port(state: tauri::State<'_, VscodeServer>) -> Result<u16, String> {
    Ok(state.port())
}

#[tauri::command]
pub async fn editor_open_project(
    state: tauri::State<'_, VscodeServer>,
    project_path: String,
) -> Result<u16, String> {
    state.stop();
    // Brief delay so the port releases
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    state.start(&project_path).await
}
