use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use futures_util::Stream;
use std::pin::Pin;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Llm error: {0}")]
    LlmError(String),
    #[error("Execution error: {0}")]
    ExecutionError(String),
    #[error("Configuration error: {0}")]
    ConfigError(String),
    #[error("Timeout error")]
    Timeout,
    #[error("Unknown error")]
    Unknown,
}

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_base64: Option<String>,
}

pub type LlmStream = Pin<Box<dyn Stream<Item = Result<String>> + Send>>;

#[async_trait]
pub trait LlmBackend: Send + Sync {
    fn name(&self) -> &'static str;
    async fn stream_chat_completion(&self, history: Vec<Message>) -> Result<LlmStream>;
}

#[async_trait]
pub trait ExecutionEngine: Send + Sync {
    fn name(&self) -> &'static str;
    async fn start_session(&mut self) -> Result<()>;
    async fn execute(&mut self, code: &str, language: &str) -> Result<String>;
    async fn terminate(&mut self) -> Result<()>;
}
