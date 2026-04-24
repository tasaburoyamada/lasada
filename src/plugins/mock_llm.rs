use crate::core::traits::{LlmBackend, Message, LlmStream, Result};
use async_trait::async_trait;
use futures_util::stream;

pub struct MockLlm;

#[async_trait]
impl LlmBackend for MockLlm {
    async fn stream_chat_completion(&self, history: Vec<Message>) -> Result<LlmStream> {

        let last_message = history.last().ok_or(crate::core::traits::AppError::LlmError("No history".into()))?;
        let response = if last_message.content.contains("ls") {
            "I will check the directory for you.\n\n```bash\nls -F\n```"
        } else if last_message.content.contains("date") {
            "Let me check the current date.\n\n```bash\ndate\n```"
        } else {
            "How can I help you today?"
        };

        let s = stream::iter(response.chars().map(|c| Ok(c.to_string())));
        Ok(Box::pin(s))
    }
}
