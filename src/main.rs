mod core;
mod plugins;

use crate::core::interpreter::Interpreter;
use crate::plugins::mock_llm::MockLlm;
use crate::plugins::openai_compatible_llm::OpenAICompatibleLlm;
use crate::plugins::execution_dispatcher::ExecutionDispatcher;
use crate::core::traits::{LlmBackend, AppError};
use std::io::{self, Write};
use dotenv::dotenv;
use config::Config;
use colored::*;
use log::{info, error, debug};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Enable debug mode
    #[arg(short, long)]
    debug: bool,

    /// LLM model name to use
    #[arg(short, long)]
    model: Option<String>,

    /// LLM API base URL
    #[arg(short, long)]
    base_url: Option<String>,

    /// System prompt
    #[arg(short, long)]
    prompt: Option<String>,

    /// Automatically run code without confirmation
    #[arg(short, long)]
    auto_run: bool,

    /// Session name for persistence
    #[arg(short, long)]
    session: Option<String>,
}

fn setup_logger(debug: bool) -> Result<(), fern::InitError> {
    let now = chrono::Local::now();
    let log_filename = format!("logs/{}.log", now.format("%Y%m%d%H%M"));
    let level = if debug { log::LevelFilter::Debug } else { log::LevelFilter::Info };

    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}] {}",
                chrono::Local::now().format("[%Y-%m-%d %H:%M:%S]"),
                record.level(),
                message
            ))
        })
        .level(level)
        .chain(std::io::stdout())
        .chain(fern::log_file(log_filename)?)
        .apply()?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let args = Args::parse();
    
    if let Err(e) = setup_logger(args.debug) {
        eprintln!("Failed to initialize logger: {}", e);
    }

    let home = std::env::var("HOME").unwrap_or_default();
    let settings = Config::builder()
        .add_source(config::File::with_name("config").required(false))
        .add_source(config::File::with_name(&format!("{}/.config/lasada/config", home)).required(false))
        .build()
        .map_err(|e| format!("Could not load config: {}", e))?;

    let llm_config = settings.get_table("llm").unwrap_or_default();
    let system_config = settings.get_table("system").unwrap_or_default();
    
    // Override settings with CLI args
    let system_prompt = args.prompt
        .or_else(|| system_config.get("prompt").map(|v| v.to_string()))
        .unwrap_or_else(|| "You are a helpful assistant.".to_string());

    info!("{}", "=== Lasada (Rust Full-Scratch Edition) ===".bright_magenta().bold());
    if args.debug {
        debug!("Debug mode enabled");
    }

    let llm_type = llm_config.get("type").map(|v| v.to_string()).unwrap_or_default();
    
    let llm: Box<dyn LlmBackend> = if llm_type == "openai_compatible" {
        let api_key = std::env::var("LLM_API_KEY").unwrap_or_default();
        let base_url = args.base_url
            .or_else(|| llm_config.get("base_url").map(|v| v.to_string()))
            .unwrap_or_default();
        let model = args.model
            .or_else(|| llm_config.get("model").map(|v| v.to_string()))
            .unwrap_or_default();
        
        info!("{} {}", "Using OpenAI Compatible LLM:".cyan(), model.yellow());
        debug!("Base URL: {}", base_url);
        Box::new(OpenAICompatibleLlm::new(api_key, base_url, model))
    } else {
        info!("{}", "Using Mock LLM".yellow());
        Box::new(MockLlm)
    };

    let executor = ExecutionDispatcher::new();
    let mut interpreter = Interpreter::new(llm, executor, system_prompt);
    
    // Set auto-run and session
    interpreter.set_auto_run(args.auto_run);
    if let Some(session_name) = args.session {
        if let Err(e) = interpreter.load_session(&session_name).await {
            error!("Failed to load session: {}", e);
        }
    }

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
