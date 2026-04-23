use crate::core::traits::{LlmBackend, Message, LlmStream};
use async_trait::async_trait;
use futures_util::stream;

pub struct MockLlm;

#[async_trait]
impl LlmBackend for MockLlm {
    fn name(&self) -> &'static str {
        "MockLlm"
    }

    async fn stream_chat_completion(&self, history: Vec<Message>) -> Result<LlmStream, String> {
        let last_message = history.last().ok_or("No history")?;
        let response = if last_message.content.contains("ls") {
            "I will check the directory for you.\n\n```bash\nls -F\n```"
        } else if last_message.content.contains("date") {
            "Let me check the current date.\n\n```bash\ndate\n```"
        } else {
            "How can I help you today?"
        };

        // 1文字ずつ出すような擬似ストリーム
        let s = stream::iter(response.chars().map(|c| Ok(c.to_string())));
        Ok(Box::pin(s))
    }
}
