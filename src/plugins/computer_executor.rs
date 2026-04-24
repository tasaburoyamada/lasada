use crate::core::traits::{ExecutionEngine, AppError, Result};
use async_trait::async_trait;
use tokio::process::Command;

pub struct ComputerExecutor;

impl ComputerExecutor {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ExecutionEngine for ComputerExecutor {
    fn name(&self) -> &'static str {
        "ComputerExecutor"
    }

    async fn start_session(&mut self) -> Result<()> {
        // Check if xdotool is installed
        let status = Command::new("xdotool")
            .arg("version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .await;
        
        if status.is_err() || !status.unwrap().success() {
            // We won't fail init, but we'll log a warning
            log::warn!("xdotool not found. Computer operation may fail.");
        }
        Ok(())
    }

    async fn execute(&mut self, code: &str, _language: &str) -> Result<String> {
        let mut result = String::new();
        let lines = code.lines();

        for line in lines {
            let line = line.trim();
            if line.is_empty() { continue; }

            if line == "screenshot" {
                // Take screenshot
                let path = "/tmp/lasada_screenshot.jpg";
                // Try scrot first
                let output = Command::new("scrot")
                    .arg("-o")
                    .arg(path)
                    .output()
                    .await;

                if let Ok(out) = output {
                    if out.status.success() {
                        result.push_str(&format!("SCREENSHOT_SAVED: {}\n", path));
                        continue;
                    }
                }
                
                // Fallback to gnome-screenshot
                let output = Command::new("gnome-screenshot")
                    .arg("-f")
                    .arg(path)
                    .output()
                    .await;

                if let Ok(out) = output {
                    if out.status.success() {
                        result.push_str(&format!("SCREENSHOT_SAVED: {}\n", path));
                        continue;
                    }
                }

                result.push_str("Error: Failed to take screenshot. Make sure 'scrot' or 'gnome-screenshot' is installed.\n");
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            let output = Command::new("xdotool")
                .args(&parts)
                .output()
                .await
                .map_err(|e| AppError::ExecutionError(format!("xdotool error: {}", e)))?;

            if !output.status.success() {
                result.push_str(&format!("Error executing '{}': {}\n", line, String::from_utf8_lossy(&output.stderr)));
            } else {
                result.push_str(&format!("Executed: {}\n", line));
            }
        }

        Ok(result.trim().to_string())
    }

    async fn terminate(&mut self) -> Result<()> {
        Ok(())
    }
}
