use crate::core::traits::ExecutionEngine;
use async_trait::async_trait;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::time::{timeout, Duration};

pub struct BashExecutor {
    child: Option<Child>,
}

impl BashExecutor {
    pub fn new() -> Self {
        Self { child: None }
    }
}

#[async_trait]
impl ExecutionEngine for BashExecutor {
    fn name(&self) -> &'static str {
        "BashExecutor"
    }

    async fn start_session(&mut self) -> Result<(), String> {
        let child = Command::new("bash")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn bash: {}", e))?;

        self.child = Some(child);
        Ok(())
    }

    async fn execute(&mut self, code: &str) -> Result<String, String> {
        let child = self.child.as_mut().ok_or("Session not started")?;
        let stdin = child.stdin.as_mut().ok_or("Failed to open stdin")?;
        
        let id = std::process::id();
        let delimiter = format!("__END_OF_COMMAND_{}__", id);
        // コマンド実行後に終了ステータスも取得するように変更
        let full_command = format!("{{ {}; }} 2>&1; echo \"{}:$?\"\n", code.trim(), delimiter);

        stdin.write_all(full_command.as_bytes()).await
            .map_err(|e| format!("Failed to write to stdin: {}", e))?;
        stdin.flush().await
            .map_err(|e| format!("Failed to flush stdin: {}", e))?;

        let stdout = child.stdout.as_mut().ok_or("Failed to open stdout")?;
        let mut reader = BufReader::new(stdout).lines();
        let mut result = String::new();

        let timeout_duration = Duration::from_secs(60);
        
        loop {
            match timeout(timeout_duration, reader.next_line()).await {
                Ok(Ok(Some(line))) => {
                    if line.contains(&delimiter) {
                        let parts: Vec<&str> = line.split(':').collect();
                        let status = parts.get(1).unwrap_or(&"0");
                        if *status != "0" {
                            result.push_str(&format!("\n[Exit Code: {}]", status));
                        }
                        break;
                    }
                    result.push_str(&line);
                    result.push('\n');
                }
                Ok(Ok(None)) => break,
                Ok(Err(e)) => return Err(format!("Read error: {}", e)),
                Err(_) => return Err("Command timed out".to_string()),
            }
        }

        Ok(result.trim().to_string())
    }

    async fn terminate(&mut self) -> Result<(), String> {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill().await;
        }
        Ok(())
    }
}
