mod core;
mod plugins;

use crate::core::interpreter::Interpreter;
use crate::plugins::mock_llm::MockLlm;
use crate::plugins::bash_executor::BashExecutor;
use crate::plugins::openai_compatible_llm::OpenAICompatibleLlm;
use crate::core::traits::LlmBackend;
use std::io::{self, Write};
use dotenv::dotenv;
use config::Config;
use colored::*;
use log::{info, error};

fn setup_logger() -> Result<(), fern::InitError> {
    // 現在時刻を取得してファイル名を生成 (YYYYMMDDHHMM.log)
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
async fn main() -> Result<(), String> {
    dotenv().ok();
    
    if let Err(e) = setup_logger() {
        eprintln!("Failed to initialize logger: {}", e);
    }

    // 設定ファイルの読み込み
    let home_config = format!("{}/.config/lasada/config", std::env::var("HOME").unwrap_or_default());
    let settings = Config::builder()
        .add_source(config::File::with_name("config").required(false))
        .add_source(config::File::with_name(&home_config).required(false))
        .build()
        .map_err(|e| format!("Could not load config: {}", e))?;

    let llm_config = settings.get_table("llm").map_err(|e| e.to_string())?;
    let system_config = settings.get_table("system").map_err(|e| e.to_string())?;
    
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
        return Err(e);
    }

    info!("{}", "Ready for input. Type 'exit' to quit.".bright_black());

    loop {
        print!("{} ", "User >".blue().bold());
        io::stdout().flush().map_err(|e| e.to_string())?;

        let mut input = String::new();
        io::stdin().read_line(&mut input).map_err(|e| e.to_string())?;
        
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
