use crate::core::traits::{ExecutionEngine, LlmBackend, Message, Result, ToolDefinition, ToolCall, LlmResponseChunk};
use std::sync::Arc;
use tokio::sync::Mutex;
use futures_util::StreamExt;
use std::io::{self, Write};
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use log::{info, debug, warn, trace};
use tiktoken_rs::cl100k_base;
use dialoguer::{Confirm, theme::ColorfulTheme};
use std::fs;
use std::path::PathBuf;
use serde_json::json;

use crate::core::vector_db::VectorDB;
use std::collections::HashMap;

pub struct Interpreter {
    llm: Box<dyn LlmBackend>,
    executor: Arc<Mutex<dyn ExecutionEngine>>,
    history: Vec<Message>,
    system_prompt: String,
    max_tokens: usize,
    auto_run: bool,
    session_name: Option<String>,
    vector_db: Option<VectorDB>,
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
            vector_db: None,
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
        
        match VectorDB::new() {
            Ok(db) => {
                self.vector_db = Some(db);
                debug!("[STATE_INIT] VectorDB initialized successfully.");
            },
            Err(e) => warn!("[STATE_INIT] Failed to initialize VectorDB: {}. Long-term memory disabled.", e),
        }
        
        if self.history.is_empty() {
            let sys_info = self.get_system_info();
            debug!("[STATE_INIT] System info collected: {}", sys_info.replace("\n", " | "));
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
                tool_calls: None,
                tool_call_id: None,
            });
            debug!("[STATE_INIT] Initial system prompt pushed.");
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

    fn get_tools(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "execute_bash".to_string(),
                description: "Execute a bash command on the host system.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "command": { "type": "string", "description": "The bash command to execute." }
                    },
                    "required": ["command"]
                }),
            },
            ToolDefinition {
                name: "execute_python".to_string(),
                description: "Execute Python code in an interactive session.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "code": { "type": "string", "description": "The Python code to execute." }
                    },
                    "required": ["code"]
                }),
            },
            ToolDefinition {
                name: "web_search".to_string(),
                description: "Search the web for information.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "The search query." }
                    },
                    "required": ["query"]
                }),
            },
        ]
    }

    pub async fn chat(&mut self, user_input: &str) -> Result<()> {
        debug!("[CHAT_START] User input received: {}", user_input);
        let mut processed_input = self.analyze_files(user_input).await;
        
        // Long-term memory retrieval
        if let Some(db) = &self.vector_db {
            if let Ok(related) = db.search(user_input, 3) {
                if !related.is_empty() {
                    processed_input.push_str("\n\n@RAG_CONTEXT:[");
                    for entry in related {
                        processed_input.push_str(&format!("[[{}]] ", entry.text.chars().take(200).collect::<String>().replace("\n", " ")));
                    }
                    processed_input.push_str("]\n");
                }
            }
        }
        
        self.history.push(Message {
            role: "user".to_string(),
            content: processed_input,
            image_base64: None,
            tool_calls: None,
            tool_call_id: None,
        });

        let tools = self.get_tools();

        for turn_idx in 0..10 {
            debug!("[CHAT_TURN] Starting turn #{}", turn_idx + 1);
            self.manage_context().await?;
            self.save_session().await?;

            let pb = ProgressBar::new_spinner();
            pb.set_style(ProgressStyle::default_spinner().template("{spinner:.green} Thinking...").unwrap());
            pb.enable_steady_tick(std::time::Duration::from_millis(80));

            let mut stream = self.llm.stream_chat_completion(self.history.clone(), Some(tools.clone())).await?;
            pb.finish_and_clear();

            let mut full_response = String::new();
            let mut pending_tool_calls: HashMap<usize, ToolCall> = HashMap::new();

            print!("{}", "AI > ".green().bold());
            io::stdout().flush().unwrap();

            while let Some(chunk) = stream.next().await {
                match chunk? {
                    LlmResponseChunk::Text(text) => {
                        print!("{}", text.green());
                        io::stdout().flush().unwrap();
                        full_response.push_str(&text);
                    }
                    LlmResponseChunk::ToolCall(call) => {
                        // ストリーミング中は各インデックスの最新状態を保持するだけ
                        // IDがある場合は新しいコール
                        if !call.id.is_empty() {
                            pending_tool_calls.insert(pending_tool_calls.len(), call);
                        } else {
                            // 引数の追記などは LlmBackend 側で既に行われている想定
                            if let Some(existing) = pending_tool_calls.values_mut().last() {
                                existing.arguments = call.arguments;
                                existing.name = call.name;
                            }
                        }
                    }
                }
            }
            println!("\n");

            let tool_calls_to_exec: Vec<ToolCall> = pending_tool_calls.into_values().collect();

            self.history.push(Message {
                role: "assistant".to_string(),
                content: full_response.clone(),
                image_base64: None,
                tool_calls: if tool_calls_to_exec.is_empty() { None } else { Some(tool_calls_to_exec.clone()) },
                tool_call_id: None,
            });

            if tool_calls_to_exec.is_empty() {
                break;
            }
            
            for call in tool_calls_to_exec {
                println!("{}", "──────────────────────────────────────────────────".bright_black());
                println!("{} {}({})", "🚀 Tool Call:".yellow().bold(), call.name.blue(), call.arguments.magenta());
                
                let should_run = if self.auto_run { true } else {
                    Confirm::with_theme(&ColorfulTheme::default())
                        .with_prompt("Execute this tool?")
                        .default(false)
                        .interact()
                        .unwrap_or(false)
                };

                if should_run {
                    let pb_exec = ProgressBar::new_spinner();
                    pb_exec.set_style(ProgressStyle::default_spinner().template("{spinner:.yellow} Executing...").unwrap());
                    pb_exec.enable_steady_tick(std::time::Duration::from_millis(80));

                    // Parse arguments
                    let args: serde_json::Value = serde_json::from_str(&call.arguments).unwrap_or(json!({}));
                    let (lang, code) = match call.name.as_str() {
                        "execute_bash" => ("bash", args["command"].as_str().unwrap_or("")),
                        "execute_python" => ("python", args["code"].as_str().unwrap_or("")),
                        "web_search" => ("web", args["query"].as_str().unwrap_or("")),
                        _ => ("unknown", ""),
                    };

                    let result = {
                        let mut executor = self.executor.lock().await;
                        executor.execute(code, lang).await?
                    };

                    pb_exec.finish_and_clear();
                    println!("{} \n{}", "✅ Result:".cyan().bold(), result);

                    self.history.push(Message {
                        role: "tool".to_string(),
                        content: result,
                        image_base64: None,
                        tool_calls: None,
                        tool_call_id: Some(call.id),
                    });
                } else {
                    self.history.push(Message {
                        role: "tool".to_string(),
                        content: "Skipped by user.".to_string(),
                        image_base64: None,
                        tool_calls: None,
                        tool_call_id: Some(call.id),
                    });
                }
            }
        }
        self.save_session().await?;
        Ok(())
    }

    pub async fn export_markdown(&self, path: &str) -> Result<()> {
        let mut markdown = String::new();
        markdown.push_str("# Lasada Session Report\n\n");
        for msg in &self.history {
            match msg.role.as_str() {
                "user" => markdown.push_str(&format!("## 👤 User\n\n{}\n\n", msg.content)),
                "assistant" => markdown.push_str(&format!("## 🤖 AI\n\n{}\n\n", msg.content)),
                "tool" => markdown.push_str(&format!("### 🛠 Tool Result\n\n```\n{}\n```\n\n", msg.content)),
                _ => {}
            }
        }
        fs::write(path, markdown).map_err(|e| crate::core::traits::AppError::ExecutionError(e.to_string()))?;
        Ok(())
    }

    async fn manage_context(&mut self) -> Result<()> {
        let bpe = cl100k_base().unwrap();
        let mut current_tokens = 0;
        for msg in &self.history {
            current_tokens += bpe.encode_with_special_tokens(&msg.content).len();
        }

        if current_tokens > self.max_tokens {
            if self.history.len() > 6 {
                let system_msg = self.history.remove(0);
                let keep_count = 5;
                let remove_index = self.history.len() - keep_count;
                let to_summarize: Vec<Message> = self.history.drain(0..remove_index).collect();
                
                let summary = self.generate_internal_summary(&to_summarize).await?;
                self.history.insert(0, Message {
                    role: "system".to_string(),
                    content: summary,
                    image_base64: None,
                    tool_calls: None,
                    tool_call_id: None,
                });
                self.history.insert(0, system_msg);
            }
        }
        Ok(())
    }

    async fn generate_internal_summary(&self, messages: &[Message]) -> Result<String> {
        let messages_text = messages.iter().map(|m| format!("{}: {}", m.role, m.content)).collect::<Vec<_>>().join("\n");
        let history = vec![
            Message { role: "system".to_string(), content: "Summarize concisely.".to_string(), image_base64: None, tool_calls: None, tool_call_id: None },
            Message { role: "user".to_string(), content: messages_text, image_base64: None, tool_calls: None, tool_call_id: None }
        ];
        let mut stream = self.llm.stream_chat_completion(history, None).await?;
        let mut summary = String::new();
        while let Some(chunk) = stream.next().await {
            if let LlmResponseChunk::Text(t) = chunk? { summary.push_str(&t); }
        }
        Ok(summary.trim().to_string())
    }

    async fn analyze_files(&mut self, text: &str) -> String {
        let re = regex::Regex::new(r"@([^\s]+)").unwrap();
        let mut file_contents = String::new();
        for cap in re.captures_iter(text) {
            let path = &cap[1];
            if let Ok(content) = fs::read_to_string(path) {
                file_contents.push_str(&format!("\n\n--- {} ---\n{}\n---\n", path, content));
            }
        }
        format!("{}{}", text, file_contents)
    }
}
