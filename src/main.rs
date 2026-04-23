mod core;
mod plugins;

use crate::core::interpreter::Interpreter;
use crate::plugins::mock_llm::MockLlm;
use crate::plugins::bash_executor::BashExecutor;
use crate::plugins::openai_compatible_llm::OpenAICompatibleLlm;
use crate::core::traits::{LlmBackend, AppError};
use std::io::{self, Write};
use dotenv::dotenv;
use config::Config;
use colored::*;
use log::{info, error};

fn setup_logger() -> Result<(), fern::InitError> {
    let now = chrono::Local::now();
    let log_filename = format!("logs/{}.log", now.format("%Y%m%d%H%M"));

    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}] {}",
                chrono::Local::now().format("[%Y-%m-%d %H:%M:%S]"),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Info)
        .chain(std::io::stdout())
        .chain(fern::log_file(log_filename)?)
        .apply()?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    
    if let Err(e) = setup_logger() {
        eprintln!("Failed to initialize logger: {}", e);
    }

    let settings = Config::builder()
        .add_source(config::File::with_name("config").required(false))
        .build()
        .map_err(|e| format!("Could not load config: {}", e))?;

    let llm_config = settings.get_table("llm").unwrap_or_default();
    let system_config = settings.get_table("system").unwrap_or_default();
    
    let system_prompt = system_config.get("prompt")
        .map(|v| v.to_string())
        .unwrap_or_else(|| "You are a helpful assistant.".to_string());

    info!("{}", "=== Lasada (Rust Full-Scratch Edition) ===".bright_magenta().bold());

    let llm_type = llm_config.get("type").map(|v| v.to_string()).unwrap_or_default();
    
    let llm: Box<dyn LlmBackend> = if llm_type == "openai_compatible" {
        let api_key = std::env::var("LLM_API_KEY").unwrap_or_default();
        let base_url = llm_config.get("base_url").map(|v| v.to_string()).unwrap_or_default();
        let model = llm_config.get("model").map(|v| v.to_string()).unwrap_or_default();
        
        info!("{} {}", "Using OpenAI Compatible LLM:".cyan(), model.yellow());
        Box::new(OpenAICompatibleLlm::new(api_key, base_url, model))
    } else {
        info!("{}", "Using Mock LLM".yellow());
        Box::new(MockLlm)
    };

    let executor = BashExecutor::new();
    let mut interpreter = Interpreter::new(llm, executor, system_prompt);
    
    if let Err(e) = interpreter.init().await {
        error!("{} {}", "Initialization Error:".red().bold(), e);
        return Err(e.into());
    }

    info!("{}", "Ready for input. Type 'exit' to quit.".bright_black());

    loop {
        print!("{} ", "User >".blue().bold());
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        let input = input.trim();
        if input.is_empty() {
            continue;
        }
        if input == "exit" || input == "quit" {
            break;
        }

        info!("User: {}", input);
        if let Err(e) = interpreter.chat(input).await {
            error!("{} {}", "Error:".red().bold(), e);
        }
    }

    info!("{}", "Goodbye.".bright_magenta());
    Ok(())
}
