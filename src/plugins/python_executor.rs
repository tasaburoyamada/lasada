use crate::core::traits::{ExecutionEngine, AppError, Result};
use async_trait::async_trait;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::time::Duration;
use std::io::{self, Write};

pub struct PythonExecutor {
    child: Option<Child>,
}

impl PythonExecutor {
    pub fn new() -> Self {
        Self { child: None }
    }
}

#[async_trait]
impl ExecutionEngine for PythonExecutor {
    fn name(&self) -> &'static str {
        "PythonExecutor"
    }

    async fn start_session(&mut self) -> Result<()> {
        let child = Command::new("python3")
            .arg("-i") // Interactive mode
            .arg("-u") // Unbuffered
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| AppError::ExecutionError(format!("Failed to spawn python3: {}", e)))?;

        self.child = Some(child);
        Ok(())
    }

    async fn execute(&mut self, code: &str, _language: &str) -> Result<String> {
        let child = self.child.as_mut().ok_or(AppError::ExecutionError("Session not started".into()))?;
        let stdin = child.stdin.as_mut().ok_or(AppError::ExecutionError("Failed to open stdin".into()))?;
        
        let id = std::process::id();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros();
        let delimiter = format!("__LASADA_PY_END_{}_{}__", id, timestamp);
        
        // Wrap code to print delimiter and status after execution
        let full_command = format!(
            "{}\nprint(f\"{}:{}{}\")\n", 
            code.trim(), 
            delimiter, 
            0, // For now, assume success if it reaches here
            ""
        );

        stdin.write_all(full_command.as_bytes()).await
            .map_err(|e| AppError::ExecutionError(format!("Failed to write to stdin: {}", e)))?;
        stdin.flush().await
            .map_err(|e| AppError::ExecutionError(format!("Failed to flush stdin: {}", e)))?;

        let stdout = child.stdout.as_mut().ok_or(AppError::ExecutionError("Failed to open stdout".into()))?;
        
        let mut stdout_reader = BufReader::new(stdout).lines();
        let mut result = String::new();
        
        loop {
            tokio::select! {
                line = stdout_reader.next_line() => {
                    match line {
                        Ok(Some(l)) => {
                            if l.starts_with(&delimiter) {
                                break;
                            }
                            // Visual feedback
                            println!("{}", l);
                            io::stdout().flush().ok();

                            result.push_str(&l);
                            result.push('\n');
                        }
                        _ => break,
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
