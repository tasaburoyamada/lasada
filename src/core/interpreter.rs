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
            self.history.push(Message {
                role: "system".to_string(),
                content: self.system_prompt.clone(),
            });
        }
        
        Ok(())
    }

    pub async fn chat(&mut self, user_input: &str) -> Result<()> {
        let processed_input = self.analyze_files(user_input).await;
        
        self.history.push(Message {
            role: "user".to_string(),
            content: processed_input,
        });

        for _ in 0..10 {
            self.manage_context();
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

                    self.history.push(Message {
                        role: "user".to_string(),
                        content: format!("Execution Result ({}):\n{}", lang, result),
                    });
                } else {
                    println!("{}", "⏭️  Skipped by user.".yellow());
                    self.history.push(Message {
                        role: "user".to_string(),
                        content: format!("Command ({}) was skipped by user. Please provide an alternative or explain why it was needed.", lang),
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

    fn manage_context(&mut self) {
        let bpe = cl100k_base().unwrap();
        let mut current_tokens = 0;
        
        // Count tokens
        for msg in &self.history {
            current_tokens += bpe.encode_with_special_tokens(&msg.content).len();
        }

        debug!("Current context tokens: {}", current_tokens);

        if current_tokens > self.max_tokens {
            info!("Context window exceeded ({} tokens). Truncating history...", current_tokens);
            // Keep system prompt (index 0) and the last 5 messages as a simple heuristic
            if self.history.len() > 6 {
                let system_msg = self.history.remove(0);
                let keep_count = 5;
                let remove_count = self.history.len() - keep_count;
                self.history.drain(0..remove_count);
                self.history.insert(0, system_msg);
                debug!("Truncated history. New length: {}", self.history.len());
            }
        }
    }

    async fn analyze_files(&self, text: &str) -> String {
        let mut result = text.to_string();
        // Regex to find @path/to/file
        let re = regex::Regex::new(r"@([^\s]+)").unwrap();
        
        let mut file_contents = String::new();
        for cap in re.captures_iter(text) {
            let path = &cap[1];
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
