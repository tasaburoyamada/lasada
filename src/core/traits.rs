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
}

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_base64: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

pub type LlmStream = Pin<Box<dyn Stream<Item = Result<String>> + Send>>;

// ストリーミング中にツールコールが発生した場合の情報を保持する構造体
#[derive(Debug, Clone)]
pub enum LlmResponseChunk {
    Text(String),
    ToolCall(ToolCall),
}

pub type LlmResponseStream = Pin<Box<dyn Stream<Item = Result<LlmResponseChunk>> + Send>>;

#[async_trait]
pub trait LlmBackend: Send + Sync {
    async fn stream_chat_completion(
        &self, 
        history: Vec<Message>, 
        tools: Option<Vec<ToolDefinition>>
    ) -> Result<LlmResponseStream>;
}

#[async_trait]
pub trait ExecutionEngine: Send + Sync {
    async fn start_session(&mut self) -> Result<()>;
    async fn execute(&mut self, code: &str, language: &str) -> Result<String>;
}
