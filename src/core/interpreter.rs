use crate::core::traits::{ExecutionEngine, LlmBackend, Message, Result};
use std::sync::Arc;
use tokio::sync::Mutex;
use futures_util::StreamExt;
use std::io::{self, Write};
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use log::{info, debug, warn};
use tiktoken_rs::cl100k_base;
use dialoguer::{Confirm, theme::ColorfulTheme};
use std::fs;
use std::path::PathBuf;

pub struct Interpreter {
    llm: Box<dyn LlmBackend>,
    executor: Arc<Mutex<dyn ExecutionEngine>>,
    history: Vec<Message>,
    system_prompt: String,
    max_tokens: usize,
    auto_run: bool,
    session_name: Option<String>,
}

impl Interpreter {
    pub fn new<E>(llm: Box<dyn LlmBackend>, executor: E, system_prompt: String) -> Self
    where
        E: ExecutionEngine + 'static,
    {
        Self {
            llm,
            executor: Arc::new(Mutex::new(executor)),
            history: Vec::new(),
            system_prompt,
            max_tokens: 8000,
            auto_run: false,
            session_name: None,
        }
    }

    pub fn set_auto_run(&mut self, auto_run: bool) {
        self.auto_run = auto_run;
    }

    pub async fn load_session(&mut self, name: &str) -> Result<()> {
        self.session_name = Some(name.to_string());
        let path = self.get_session_path(name);
        if path.exists() {
            let content = fs::read_to_string(path)
                .map_err(|e| crate::core::traits::AppError::ConfigError(format!("Failed to read session: {}", e)))?;
            self.history = serde_json::from_str(&content)
                .map_err(|e| crate::core::traits::AppError::ConfigError(format!("Failed to parse session: {}", e)))?;
            info!("Session '{}' loaded ({} messages)", name, self.history.len());
        }
        Ok(())
    }

    async fn save_session(&self) -> Result<()> {
        if let Some(ref name) = self.session_name {
            let path = self.get_session_path(name);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).ok();
            }
            let content = serde_json::to_string_pretty(&self.history)
                .map_err(|e| crate::core::traits::AppError::ConfigError(format!("Failed to serialize session: {}", e)))?;
            fs::write(path, content)
                .map_err(|e| crate::core::traits::AppError::ConfigError(format!("Failed to write session: {}", e)))?;
        }
        Ok(())
    }

    fn get_session_path(&self, name: &str) -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_default();
        PathBuf::from(home).join(".config/lasada/sessions").join(format!("{}.json", name))
    }

    pub async fn init(&mut self) -> Result<()> {
        let mut executor = self.executor.lock().await;
        executor.start_session().await?;
        
        if self.history.is_empty() {
            let sys_info = self.get_system_info();
            let mut content = self.system_prompt.clone();
            // Inject symbolic system state
            content.push_str("\n\n@CTX:[DOM:SYSTEM_ROOT|GOAL:INITIALIZED]\n");
            content.push_str("@BIAS:{P:1.0, M:1.0, S:1.0, D:1.0, C:1.0}\n");
            content.push_str("CONCEPT: [[READY]] [[ENV_LOADED]]\n");
            content.push_str("--- Runtime Specs ---\n");
            content.push_str(&sys_info);
            content.push_str("\n--------------------\n");

            self.history.push(Message {
                role: "system".to_string(),
                content,
                image_base64: None,
            });
        }
        
        Ok(())
    }

    fn get_system_info(&self) -> String {
        use sysinfo::System;
        let mut sys = System::new_all();
        sys.refresh_all();

        let os = System::name().unwrap_or_else(|| "Unknown".to_string());
        let kernel = System::kernel_version().unwrap_or_else(|| "Unknown".to_string());
        let host = System::host_name().unwrap_or_else(|| "Unknown".to_string());
        let cpu = sys.cpus().len();
        let mem = sys.total_memory() / 1024 / 1024; // MB
        let pwd = std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| "Unknown".to_string());

        format!(
            "OS: {}\nKernel: {}\nHostname: {}\nCPU Cores: {}\nTotal Memory: {} MB\nWorking Directory: {}",
            os, kernel, host, cpu, mem, pwd
        )
    }

    pub async fn chat(&mut self, user_input: &str) -> Result<()> {
        let processed_input = self.analyze_files(user_input).await;
        
        self.history.push(Message {
            role: "user".to_string(),
            content: processed_input,
            image_base64: None,
        });

        for _ in 0..10 {
            self.manage_context().await?;
            self.save_session().await?;

            let pb = ProgressBar::new_spinner();
            pb.set_style(ProgressStyle::default_spinner()
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
                .template("{spinner:.green} Thinking...")
                .unwrap());
            pb.enable_steady_tick(std::time::Duration::from_millis(80));

            let mut stream = self.llm.stream_chat_completion(self.history.clone()).await?;
            pb.finish_and_clear();

            let mut full_response = String::new();
            print!("{}", "AI > ".green().bold());
            io::stdout().flush().unwrap();

            while let Some(chunk) = stream.next().await {
                let text = chunk?;
                print!("{}", text.green());
                io::stdout().flush().unwrap();
                full_response.push_str(&text);
            }
            println!("\n"); // Clear the stream line
            
            // Render rich markdown
            termimad::print_text(&full_response);
            println!();
            
            info!("AI: {}", full_response);

            self.history.push(Message {
                role: "assistant".to_string(),
                content: full_response.clone(),
                image_base64: None,
            });

            let code_blocks = self.extract_code_blocks(&full_response);
            if code_blocks.is_empty() {
                break;
            }
            
            for (lang, code) in code_blocks {
                println!("{}", "──────────────────────────────────────────────────".bright_black());
                println!("{} {} ({})", "🚀 Proposed Command:".yellow().bold(), code.blue(), lang.magenta());
                
                let should_run = if self.auto_run {
                    true
                } else {
                    Confirm::with_theme(&ColorfulTheme::default())
                        .with_prompt("Do you want to execute this command?")
                        .default(false)
                        .interact()
                        .unwrap_or(false)
                };

                if should_run {
                    info!("Executing ({}): {}", lang, code);
                    
                    let pb_exec = ProgressBar::new_spinner();
                    pb_exec.set_style(ProgressStyle::default_spinner().template("{spinner:.yellow} Running...").unwrap());
                    pb_exec.enable_steady_tick(std::time::Duration::from_millis(80));

                    let result = {
                        let mut executor = self.executor.lock().await;
                        executor.execute(&code, &lang).await?
                    };

                    pb_exec.finish_and_clear();
                    println!("{} \n{}", "✅ Result:".cyan().bold(), result);
                    info!("Result: {}", result);
                    println!("{}", "──────────────────────────────────────────────────".bright_black());

                    let mut image_base64 = None;
                    let mut display_result = result.clone();

                    if result.contains("SCREENSHOT_SAVED: ") {
                        if let Some(path) = result.lines().find(|l| l.contains("SCREENSHOT_SAVED: ")) {
                            let path = path.replace("SCREENSHOT_SAVED: ", "").trim().to_string();
                            if let Ok(bytes) = fs::read(&path) {
                                use base64::{Engine as _, engine::general_purpose};
                                image_base64 = Some(general_purpose::STANDARD.encode(bytes));
                                display_result = format!("(Screenshot captured and attached: {})", path);
                            }
                        }
                    }

                    let mut feedback = format!("Execution Result ({}):\n{}", lang, display_result);
                    if result.to_lowercase().contains("error") || result.to_lowercase().contains("failed") || result.contains("[Exit Code:") {
                        feedback.push_str("\n\n(It seems the command failed or had an error. Please analyze the output and suggest a fix if necessary.)");
                    }

                    self.history.push(Message {
                        role: "user".to_string(),
                        content: feedback,
                        image_base64,
                    });
                } else {
                    println!("{}", "⏭️  Skipped by user.".yellow());
                    self.history.push(Message {
                        role: "user".to_string(),
                        content: format!("Command ({}) was skipped by user. Please provide an alternative or explain why it was needed.", lang),
                        image_base64: None,
                    });
                    break;
                }
            }
        }
        self.save_session().await?;
        Ok(())
    }

    fn extract_code_blocks(&self, text: &str) -> Vec<(String, String)> {
        let mut blocks = Vec::new();
        // Regex to match any code block with a language tag
        let re = regex::Regex::new(r"```([a-zA-Z0-9]*)\s*[\r\n]+([\s\S]*?)```").unwrap();
        for cap in re.captures_iter(text) {
            let lang = cap[1].to_lowercase();
            let code = cap[2].trim().to_string();
            blocks.push((lang, code));
        }
        blocks
    }

    async fn manage_context(&mut self) -> Result<()> {
        let bpe = cl100k_base().unwrap();
        let mut current_tokens = 0;
        
        // Count tokens
        for msg in &self.history {
            current_tokens += bpe.encode_with_special_tokens(&msg.content).len();
        }

        debug!("Current context tokens: {}", current_tokens);

        if current_tokens > self.max_tokens {
            info!("Context window exceeded ({} tokens). Summarizing history...", current_tokens);
            // Keep system prompt (index 0) and the last 5 messages as a simple heuristic
            if self.history.len() > 6 {
                let system_msg = self.history.remove(0);
                let keep_count = 5;
                let remove_index = self.history.len() - keep_count;
                
                let to_summarize: Vec<Message> = self.history.drain(0..remove_index).collect();
                
                println!("{}", "⏳ Summarizing old context...".yellow().dimmed());
                let summary = self.generate_internal_summary(&to_summarize).await?;
                
                self.history.insert(0, Message {
                    role: "system".to_string(),
                    content: summary,
                    image_base64: None,
                });
                self.history.insert(0, system_msg);
                debug!("Summarized history. New length: {}", self.history.len());
            }
        }
        Ok(())
    }

    async fn generate_internal_summary(&self, messages: &[Message]) -> Result<String> {
        let mut summarization_history = Vec::new();
        let messages_text = messages.iter()
            .map(|m| format!("{}: {}", m.role, m.content))
            .collect::<Vec<_>>()
            .join("\n");

        summarization_history.push(Message {
            role: "system".to_string(),
            content: "@CTX:[DOM:HV-CAD|SUB:ENCODER|GOAL:STATE_SERIALIZATION] ![[STRICT_VLOG_FORMAT]] ![[NO_EXPLANATION]] ![[MINIMAL_TOKEN]] @BIAS:{C:1.0, S:1.0, P:1.0}".to_string(),
            image_base64: None,
        });
        summarization_history.push(Message {
            role: "user".to_string(),
            content: format!("@INPUT:\n\n{}", messages_text),
            image_base64: None,
        });

        let mut stream = self.llm.stream_chat_completion(summarization_history).await?;
        let mut summary = String::new();
        while let Some(chunk) = stream.next().await {
            summary.push_str(&chunk?);
        }
        Ok(summary.trim().to_string())
    }

    async fn analyze_files(&self, text: &str) -> String {
        let mut result = text.to_string();
        // Regex to find @path/to/file
        let re = regex::Regex::new(r"@([^\s]+)").unwrap();
        
        let mut file_contents = String::new();
        for cap in re.captures_iter(text) {
            let path = &cap[1];
            let path_buf = PathBuf::from(path);
            
            if let Some(ext) = path_buf.extension().and_then(|e| e.to_str()) {
                let ext_lower = ext.to_lowercase();
                if ext_lower == "pdf" {
                    // ... (pdf logic)
                    match tokio::process::Command::new("pdftotext")
                        .arg(path)
                        .arg("-")
                        .output()
                        .await 
                    {
                        Ok(output) if output.status.success() => {
                            let content = String::from_utf8_lossy(&output.stdout);
                            file_contents.push_str(&format!("\n\n--- Content of PDF {} ---\n{}\n---\n", path, content));
                            info!("Injected PDF file: {}", path);
                        }
                        _ => {
                            warn!("Failed to extract text from PDF: {}", path);
                            file_contents.push_str(&format!("\n\n(Error: Failed to extract text from PDF {}. Make sure 'poppler-utils' is installed.)\n", path));
                        }
                    }
                    continue;
                } else if ext_lower == "ipynb" {
                    // Jupyter Notebook support
                    match fs::read_to_string(path) {
                        Ok(content) => {
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                                let mut notebook_text = String::new();
                                if let Some(cells) = json.get("cells").and_then(|c| c.as_array()) {
                                    for cell in cells {
                                        let cell_type = cell.get("cell_type").and_then(|t| t.as_str()).unwrap_or("");
                                        let source = cell.get("source").and_then(|s| {
                                            if s.is_array() {
                                                Some(s.as_array().unwrap().iter().map(|line| line.as_str().unwrap_or("")).collect::<String>())
                                            } else {
                                                s.as_str().map(|s| s.to_string())
                                            }
                                        }).unwrap_or_default();

                                        if cell_type == "code" {
                                            notebook_text.push_str(&format!("\n# In [ ]:\n{}\n", source));
                                        } else if cell_type == "markdown" {
                                            notebook_text.push_str(&format!("\n'''\n{}\n'''\n", source));
                                        }
                                    }
                                    file_contents.push_str(&format!("\n\n--- Content of Jupyter Notebook {} ---\n{}\n---\n", path, notebook_text));
                                    info!("Injected Jupyter Notebook: {}", path);
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Failed to read ipynb {}: {}", path, e);
                        }
                    }
                    continue;
                }
            }

            match fs::read_to_string(path) {
                Ok(content) => {
                    file_contents.push_str(&format!("\n\n--- Content of {} ---\n{}\n---\n", path, content));
                    info!("Injected file: {}", path);
                }
                Err(e) => {
                    warn!("Failed to read file {}: {}", path, e);
                    file_contents.push_str(&format!("\n\n(Error reading file {}: {})\n", path, e));
                }
            }
        }
        
        result.push_str(&file_contents);
        result
    }
}
