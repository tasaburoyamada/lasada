use crate::core::traits::{ExecutionEngine, AppError, Result};
use async_trait::async_trait;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::time::Duration;

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

    async fn start_session(&mut self) -> Result<()> {
        let child = Command::new("bash")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| AppError::ExecutionError(format!("Failed to spawn bash: {}", e)))?;

        self.child = Some(child);
        Ok(())
    }

    async fn execute(&mut self, code: &str) -> Result<String> {
        let child = self.child.as_mut().ok_or(AppError::ExecutionError("Session not started".into()))?;
        let stdin = child.stdin.as_mut().ok_or(AppError::ExecutionError("Failed to open stdin".into()))?;
        
        let id = std::process::id();
        let delimiter = format!("__END_OF_COMMAND_{}__", id);
        let full_command = format!("{{ {}; }} 2>&1; echo \"{}:$?\"\n", code.trim(), delimiter);

        stdin.write_all(full_command.as_bytes()).await
            .map_err(|e| AppError::ExecutionError(format!("Failed to write to stdin: {}", e)))?;
        stdin.flush().await
            .map_err(|e| AppError::ExecutionError(format!("Failed to flush stdin: {}", e)))?;

        let stdout = child.stdout.as_mut().ok_or(AppError::ExecutionError("Failed to open stdout".into()))?;
        let stderr = child.stderr.as_mut().ok_or(AppError::ExecutionError("Failed to open stderr".into()))?;
        
        let mut stdout_reader = BufReader::new(stdout).lines();
        let mut stderr_reader = BufReader::new(stderr).lines();
        
        let mut result = String::new();
        
        loop {
            tokio::select! {
                line = stdout_reader.next_line() => {
                    match line {
                        Ok(Some(l)) => {
                            if l.contains(&delimiter) {
                                let parts: Vec<&str> = l.split(':').collect();
                                let status = parts.get(1).unwrap_or(&"0");
                                if *status != "0" {
                                    result.push_str(&format!("\n[Exit Code: {}]", status));
                                }
                                break;
                            }
                            result.push_str(&l);
                            result.push('\n');
                        }
                        _ => break,
                    }
                }
                line = stderr_reader.next_line() => {
                    match line {
                        Ok(Some(l)) => {
                            result.push_str(&l);
                            result.push('\n');
                        }
                        _ => {},
                    }
                }
                _ = tokio::time::sleep(Duration::from_secs(30)) => {
                    return Err(AppError::Timeout);
                }
            }
        }

        Ok(result.trim().to_string())
    }

    async fn terminate(&mut self) -> Result<()> {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill().await;
        }
        Ok(())
    }
}
