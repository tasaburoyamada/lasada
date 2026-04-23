use crate::core::traits::{ExecutionEngine, LlmBackend, Message, Result};
use std::sync::Arc;
use tokio::sync::Mutex;
use futures_util::StreamExt;
use std::io::{self, Write};
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use log::info;

pub struct Interpreter {
    llm: Box<dyn LlmBackend>,
    executor: Arc<Mutex<dyn ExecutionEngine>>,
    history: Vec<Message>,
    system_prompt: String,
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
            println!();
            info!("AI: {}", full_response);

            self.history.push(Message {
                role: "assistant".to_string(),
                content: full_response.clone(),
            });

            let codes = self.extract_bash_code(&full_response);
            if codes.is_empty() {
                break;
            }
            
            for code in codes {
                println!("{}", "──────────────────────────────────────────────────".bright_black());
                println!("{} {}", "🚀 Executing:".yellow().bold(), code.blue());
                info!("Executing: {}", code);
                
                let pb_exec = ProgressBar::new_spinner();
                pb_exec.set_style(ProgressStyle::default_spinner().template("{spinner:.yellow} Running...").unwrap());
                pb_exec.enable_steady_tick(std::time::Duration::from_millis(80));

                let result = {
                    let mut executor = self.executor.lock().await;
                    executor.execute(&code).await?
                };

                pb_exec.finish_and_clear();
                println!("{} \n{}", "✅ Result:".cyan().bold(), result);
                info!("Result: {}", result);
                println!("{}", "──────────────────────────────────────────────────".bright_black());

                self.history.push(Message {
                    role: "user".to_string(),
                    content: format!("Execution Result:\n{}", result),
                });
            }
        }

        Ok(())
    }

    fn extract_bash_code(&self, text: &str) -> Vec<String> {
        let mut codes = Vec::new();
        let re = regex::Regex::new(r"```bash\n([\s\S]*?)```").unwrap();
        for cap in re.captures_iter(text) {
            codes.push(cap[1].trim().to_string());
        }
        codes
    }
}
