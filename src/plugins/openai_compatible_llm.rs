use crate::core::traits::{LlmBackend, Message, LlmStream, Result, AppError};
use async_trait::async_trait;
use serde_json::json;
use futures_util::{StreamExt};
use serde::Deserialize;
use tokio::sync::Mutex;
use regex::Regex;
use std::time::Instant;

#[derive(Deserialize, Debug)]
struct ChatCompletionChunk {
    #[serde(default)]
    choices: Vec<ChoiceChunk>,
    #[serde(default)]
    output: Vec<OutputChunk>,
    #[serde(rename = "type")]
    #[serde(default)]
    event_type: String,
    #[serde(default)]
    delta: Option<serde_json::Value>,
}

#[derive(Deserialize, Debug, Default)]
struct ChoiceChunk {
    delta: DeltaChunk,
}

#[derive(Deserialize, Debug, Default)]
struct DeltaChunk {
    content: Option<String>,
}

#[derive(Deserialize, Debug, Default)]
struct OutputChunk {
    content: Vec<ContentPart>,
}

#[derive(Deserialize, Debug, Default)]
struct ContentPart {
    text: Option<String>,
}

pub struct OpenAICompatibleLlm {
    api_key: String,
    base_url: String,
    model: String,
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

    fn mask_key(&self, key: &str) -> String {
        if key.len() <= 4 { return "****".to_string(); }
        let prefix: String = key.chars().take(2).collect();
        let suffix: String = key.chars().skip(key.len() - 2).collect();
        format!("{}...{} ({} chars)", prefix, suffix, key.len())
    }
}

#[async_trait]
impl LlmBackend for OpenAICompatibleLlm {
    async fn stream_chat_completion(&self, history: Vec<Message>) -> Result<LlmStream> {
        let start_time = Instant::now();
        let client = reqwest::Client::builder()
            .use_rustls_tls()
            .http2_prior_knowledge() 
            .user_agent("lasada/0.1.0 (Rust)")
            .build()
            .map_err(|e| AppError::LlmError(format!("Failed to build client: {}", e)))?;
        
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
                json!({ "role": m.role, "content": m.content })
            }
        }).collect();

        let patterns = vec![
            json!({ "model": self.model, "messages": messages_val, "stream": true, "stream_options": { "include_usage": true } }),
            json!({ "model": self.model, "messages": messages_val, "stream": true }),
            json!({ "model": self.model, "input": messages_val, "stream": true }),
            json!({ "model": format!("openai/{}", self.model), "input": messages_val, "stream": true }),
        ];

        let auth_patterns = vec![
            vec![("api-key", self.api_key.clone())],
            vec![("Authorization", format!("Bearer {}", self.api_key))],
            vec![("api-key", self.api_key.clone()), ("OpenAI-Beta", "realtime=v1".to_string())],
            vec![("Authorization", self.api_key.clone())],
        ];

        let mut format_idx_lock = self.request_format_index.lock().await;
        let (p_start, h_start, p_limit, h_limit) = if let Some((p, h)) = *format_idx_lock {
            (p, h, p + 1, h + 1)
        } else {
            (0, 0, patterns.len(), auth_patterns.len())
        };

        for p_idx in p_start..p_limit {
            for h_idx in h_start..h_limit {
                let body = &patterns[p_idx];
                let headers = &auth_patterns[h_idx];

                let mut req = client.post(&self.base_url)
                    .header("Content-Type", "application/json; charset=utf-8")
                    .header("Accept", "application/json, text/event-stream");

                for (name, val) in headers {
                    req = req.header(*name, val);
                }

                log::debug!("[LLM_REQ] URL: {}", self.base_url);
                log::debug!("[LLM_REQ] Pattern: P{}, Header: H{}", p_idx, h_idx);
                log::trace!("[LLM_REQ] Body: {}", serde_json::to_string(body).unwrap_or_default());
                for (name, val) in headers {
                    log::trace!("[LLM_REQ] Header: {}: {}", name, if name.contains("key") || name.contains("Auth") { self.mask_key(val) } else { val.clone() });
                }

                let res = req.json(body).send().await
                    .map_err(|e| AppError::LlmError(e.to_string()))?;

                let status = res.status();
                log::debug!("[LLM_RES] Status: {}", status);

                if status.is_success() {
                    if format_idx_lock.is_none() {
                        *format_idx_lock = Some((p_idx, h_idx));
                        log::info!("[LLM_ADAPT] Fixed format: P{}, H{}", p_idx, h_idx);
                    }

                    let stream = res.bytes_stream().map(move |item| {
                        match item {
                            Ok(bytes) => {
                                let s = String::from_utf8_lossy(&bytes);
                                log::trace!("[LLM_STREAM] Raw: {}", s);
                                let mut content_acc = String::new();
                                let re_data = Regex::new(r"data:\s*(.*)").unwrap();
                                for line in s.lines() {
                                    let line = line.trim();
                                    if line.is_empty() || line == "data: [DONE]" { continue; }
                                    let raw = if let Some(cap) = re_data.captures(line) { cap.get(1).map(|m| m.as_str()).unwrap_or(line) } else { line };
                                    if let Ok(chunk) = serde_json::from_str::<ChatCompletionChunk>(raw) {
                                        if let Some(c) = chunk.choices.get(0).and_then(|c| c.delta.content.as_ref()) {
                                            content_acc.push_str(c);
                                        } else if let Some(o) = chunk.output.get(0).and_then(|o| o.content.get(0)).and_then(|cp| cp.text.as_ref()) {
                                            content_acc.push_str(o);
                                        } else if chunk.event_type == "response.text.delta" {
                                            if let Some(delta_val) = chunk.delta {
                                                if let Some(text) = delta_val.as_str() { content_acc.push_str(text); }
                                                else if let Some(text) = delta_val.get("text").and_then(|t| t.as_str()) { content_acc.push_str(text); }
                                            }
                                        }
                                    }
                                }
                                Ok(content_acc)
                            }
                            Err(e) => Err(AppError::LlmError(e.to_string())),
                        }
                    });
                    
                    log::debug!("[LLM_RES] TTFT: {}ms", start_time.elapsed().as_millis());
                    return Ok(Box::pin(stream));
                } else {
                    log::warn!("[LLM_RES] Pattern P{}, H{} failed: {}", p_idx, h_idx, status);
                }
            }
        }
        Err(AppError::LlmError("All request patterns failed.".into()))
    }
}
