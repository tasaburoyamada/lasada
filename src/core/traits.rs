use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use futures_util::Stream;
use std::pin::Pin;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

pub type LlmStream = Pin<Box<dyn Stream<Item = Result<String, String>> + Send>>;

#[async_trait]
pub trait LlmBackend: Send + Sync {
    fn name(&self) -> &'static str;
    // ストリーミング形式で結果を返すように変更
    async fn stream_chat_completion(&self, history: Vec<Message>) -> Result<LlmStream, String>;
}

#[async_trait]
pub trait ExecutionEngine: Send + Sync {
    fn name(&self) -> &'static str;
    async fn start_session(&mut self) -> Result<(), String>;
    async fn execute(&mut self, code: &str) -> Result<String, String>;
    async fn terminate(&mut self) -> Result<(), String>;
}
