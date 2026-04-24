use crate::core::traits::{LlmBackend, Message, LlmStream, Result, AppError};
use async_trait::async_trait;
use serde_json::json;
use futures_util::{StreamExt};
use serde::Deserialize;
use tokio::sync::Mutex;

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
    // Caches (BodyPatternIndex, HeaderIndex)
    request_format_index: Mutex<Option<(usize, usize)>>, 
}

impl OpenAICompatibleLlm {
    pub fn new(api_key: String, base_url: String, model: String) -> Self {
        Self { 
            api_key, 
            base_url, 
            model,
            request_format_index: Mutex::new(None),
        }
    }
}

#[async_trait]
impl LlmBackend for OpenAICompatibleLlm {
    async fn stream_chat_completion(&self, history: Vec<Message>) -> Result<LlmStream> {
        let client = reqwest::Client::new();
        
        let messages_val: Vec<serde_json::Value> = history.iter().map(|m| {
            if let Some(img) = &m.image_base64 {
                json!({
                    "role": m.role,
                    "content": [
                        { "type": "text", "text": m.content },
                        { "type": "image_url", "image_url": { "url": format!("data:image/jpeg;base64,{}", img) } }
                    ]
                })
            } else {
                json!({
                    "role": m.role,
                    "content": m.content
                })
            }
        }).collect();

        // 1. Define Body Patterns (Mirroring LiteLLM 1.82+ logic)
        let patterns = vec![
            // P0: Standard OpenAI
            json!({ "model": self.model, "messages": messages_val, "stream": true }),
            // P1: LiteLLM /responses default (Simple input array)
            json!({ "model": self.model, "input": messages_val, "stream": true }),
            // P2: Strict Responses API structure
            json!({ 
                "model": self.model, 
                "input": history.iter().map(|m| {
                    json!({
                        "type": "message",
                        "role": m.role,
                        "content": [{ "type": "input_text", "text": m.content }]
                    })
                }).collect::<Vec<_>>(),
                "stream": true 
            }),
        ];

        // 2. Define Header Patterns
        let auth_headers = vec![
            ("Authorization", format!("Bearer {}", self.api_key)),
            ("api-key", self.api_key.clone()),
            ("Authorization", self.api_key.clone()),
            ("X-API-Key", self.api_key.clone()),
        ];

        let mut format_idx_lock = self.request_format_index.lock().await;
        
        let (p_start, h_start, p_limit, h_limit) = if let Some((p, h)) = *format_idx_lock {
            (p, h, p + 1, h + 1)
        } else {
            (0, 0, patterns.len(), auth_headers.len())
        };

        for p_idx in p_start..p_limit {
            for h_idx in h_start..h_limit {
                let body = &patterns[p_idx];
                let (h_name, h_val) = &auth_headers[h_idx];

                log::debug!("LiteLLM Emulation - Attempting: Body P{}, Header H{} ({}) to {}", p_idx, h_idx, h_name, self.base_url);

                let res = client.post(&self.base_url)
                    .header(*h_name, h_val)
                    .json(body)
                    .send()
                    .await
                    .map_err(|e| AppError::LlmError(e.to_string()))?;

                let status = res.status();
                log::debug!("Response Status: {}", status);

                if status.is_success() {
                    if format_idx_lock.is_none() {
                        *format_idx_lock = Some((p_idx, h_idx));
                        log::info!("Adaptation Success! Fixed format: Body P{}, Header {} (Index {})", p_idx, h_name, h_idx);
                    }

                    let stream = res.bytes_stream().map(|item| {
                        match item {
                            Ok(bytes) => {
                                let s = String::from_utf8_lossy(&bytes);
                                log::debug!("Raw Chunk: {}", s);
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
                                    } else {
                                        // Some proxies send raw text instead of data: SSE
                                        content_acc.push_str(line);
                                    }
                                }
                                Ok(content_acc)
                            }
                            Err(e) => Err(AppError::LlmError(e.to_string())),
                        }
                    });

                    return Ok(Box::pin(stream));
                } else {
                    log::warn!("Combination (P{}, H{}) failed with {}. Continuing probe...", p_idx, h_idx, status);
                }
            }
        }

        Err(AppError::LlmError("All LiteLLM emulation patterns failed. Please check your URL and API Key.".into()))
    }
}
