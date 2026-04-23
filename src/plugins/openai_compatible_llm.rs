use crate::core::traits::{LlmBackend, Message, LlmStream, Result, AppError};
use async_trait::async_trait;
use serde_json::json;
use futures_util::{StreamExt};
use serde::Deserialize;

#[derive(Deserialize)]
struct ChatCompletionChunk {
    choices: Vec<ChoiceChunk>,
}

#[derive(Deserialize)]
struct ChoiceChunk {
    delta: DeltaChunk,
}

#[derive(Deserialize)]
struct DeltaChunk {
    content: Option<String>,
}

pub struct OpenAICompatibleLlm {
    api_key: String,
    base_url: String,
    model: String,
}

impl OpenAICompatibleLlm {
    pub fn new(api_key: String, base_url: String, model: String) -> Self {
        Self { api_key, base_url, model }
    }
}

#[async_trait]
impl LlmBackend for OpenAICompatibleLlm {
    fn name(&self) -> &'static str {
        "OpenAICompatibleLlm"
    }

    async fn stream_chat_completion(&self, history: Vec<Message>) -> Result<LlmStream> {
        let client = reqwest::Client::new();
        
        let body = json!({
            "model": self.model,
            "messages": history,
            "stream": true
        });

        let res = client.post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::LlmError(e.to_string()))?;

        let stream = res.bytes_stream().map(|item| {
            match item {
                Ok(bytes) => {
                    let s = String::from_utf8_lossy(&bytes);
                    let mut content_acc = String::new();
                    for line in s.lines() {
                        let line = line.trim();
                        if line.is_empty() { continue; }
                        if line == "data: [DONE]" { break; }
                        if line.starts_with("data: ") {
                            let data = &line[6..];
                            if let Ok(chunk) = serde_json::from_str::<ChatCompletionChunk>(data) {
                                if let Some(content) = &chunk.choices[0].delta.content {
                                    content_acc.push_str(content);
                                }
                            }
                        }
                    }
                    Ok(content_acc)
                }
                Err(e) => Err(AppError::LlmError(e.to_string())),
            }
        });

        Ok(Box::pin(stream))
    }
}
