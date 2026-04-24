use crate::core::traits::{ExecutionEngine, LlmBackend, Message, Result};
use std::sync::Arc;
use tokio::sync::Mutex;
use futures_util::StreamExt;
use std::io::{self, Write};
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use log::{info, debug};
use tiktoken_rs::cl100k_base;

pub struct Interpreter {
    llm: Box<dyn LlmBackend>,
    executor: Arc<Mutex<dyn ExecutionEngine>>,
    history: Vec<Message>,
    system_prompt: String,
    max_tokens: usize,
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
        }
    }

    pub async fn init(&mut self) -> Result<()> {
        let mut executor = self.executor.lock().await;
        executor.start_session().await?;
        
        self.history.push(Message {
            role: "system".to_string(),
            content: self.system_prompt.clone(),
        });
        
        Ok(())
    }

    pub async fn chat(&mut self, user_input: &str) -> Result<()> {
        self.history.push(Message {
            role: "user".to_string(),
            content: user_input.to_string(),
        });

        for _ in 0..10 {
            self.manage_context();

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
                println!("{} {} ({})", "🚀 Executing:".yellow().bold(), code.blue(), lang.magenta());
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
            }
        }

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
}
