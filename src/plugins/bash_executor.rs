use crate::core::traits::{ExecutionEngine, AppError, Result};
use async_trait::async_trait;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::time::Duration;
use std::io::{self, Write};

pub struct BashExecutor {
    child: Option<Child>,
}

impl BashExecutor {
    pub fn new() -> Self {
        Self { child: None }
    }
}

use log::{debug, trace, warn};

#[async_trait]
impl ExecutionEngine for BashExecutor {
    async fn start_session(&mut self) -> Result<()> {
        let mut child = Command::new("bash")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| AppError::ExecutionError(format!("Failed to spawn bash: {}", e)))?;

        let pid = child.id().unwrap_or(0);
        debug!("[BASH_INIT] Spawned bash process with PID: {}", pid);

        // Redirect stderr to stdout for this session to simplify output capturing
        if let Some(stdin) = child.stdin.as_mut() {
            stdin.write_all(b"exec 2>&1\n").await
                .map_err(|e| AppError::ExecutionError(format!("Failed to setup stderr redirection: {}", e)))?;
            stdin.flush().await
                .map_err(|e| AppError::ExecutionError(format!("Failed to flush stdin: {}", e)))?;
        }

        self.child = Some(child);
        Ok(())
    }

    async fn execute(&mut self, code: &str, _language: &str) -> Result<String> {
        let child = self.child.as_mut().ok_or(AppError::ExecutionError("Session not started".into()))?;
        let pid = child.id().unwrap_or(0);
        let stdin = child.stdin.as_mut().ok_or(AppError::ExecutionError("Failed to open stdin".into()))?;
        
        let id = std::process::id();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros();
        let delimiter = format!("__LASADA_END_{}_{}__", id, timestamp);
        
        trace!("[BASH_EXEC] PID: {} | Payload: {}", pid, code.trim());
        
        let full_command = format!("{}\necho \"{}:$?\"\n", code.trim(), delimiter);

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
                            if l.starts_with(&delimiter) {
                                let status = l.strip_prefix(&delimiter)
                                    .and_then(|s| s.strip_prefix(':'))
                                    .unwrap_or("0");
                                debug!("[BASH_DONE] PID: {} | Exit Code: {}", pid, status);
                                if status != "0" {
                                    result.push_str(&format!("\n[Exit Code: {}]", status));
                                }
                                break;
                            }
                            trace!("[BASH_STDOUT] {}", l);
                            // Visual feedback
                            println!("{}", l);
                            io::stdout().flush().ok();

                            result.push_str(&l);
                            result.push('\n');
                        }
                        _ => break,
                    }
                }
                line = stderr_reader.next_line() => {
                    match line {
                        Ok(Some(l)) => {
                            trace!("[BASH_STDERR] {}", l);
                            result.push_str(&l);
                            result.push('\n');
                        }
                        _ => {},
                    }
                }
                _ = tokio::time::sleep(Duration::from_secs(30)) => {
                    warn!("[BASH_TIMEOUT] PID: {} | Command execution exceeded 30s.", pid);
                    return Err(AppError::Timeout);
                }
            }
        }

        Ok(result.trim().to_string())
    }
}
