use crate::core::traits::{LlmBackend, Message, LlmResponseStream, LlmResponseChunk, ToolDefinition, ToolCall, Result, AppError};
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
    tool_calls: Option<Vec<ToolCallDelta>>,
}

#[derive(Deserialize, Debug, Default)]
struct ToolCallDelta {
    index: Option<usize>,
    id: Option<String>,
    #[serde(rename = "type")]
    call_type: Option<String>,
    function: Option<FunctionDelta>,
}

#[derive(Deserialize, Debug, Default)]
struct FunctionDelta {
    name: Option<String>,
    arguments: Option<String>,
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
    async fn stream_chat_completion(
        &self, 
        history: Vec<Message>,
        tools: Option<Vec<ToolDefinition>>
    ) -> Result<LlmResponseStream> {
        let start_time = Instant::now();
        let client = reqwest::Client::builder()
            .use_rustls_tls()
            .http2_prior_knowledge() 
            .user_agent("lasada/0.1.0 (Rust)")
            .build()
            .map_err(|e| AppError::LlmError(format!("Failed to build client: {}", e)))?;
        
        let messages_val: Vec<serde_json::Value> = history.iter().map(|m| {
            let mut val = json!({
                "role": m.role,
                "content": m.content
            });

            if let Some(img) = &m.image_base64 {
                val["content"] = json!([
                    { "type": "text", "text": m.content },
                    { "type": "image_url", "image_url": { "url": format!("data:image/jpeg;base64,{}", img) } }
                ]);
            }

            if let Some(calls) = &m.tool_calls {
                val["tool_calls"] = json!(calls.iter().map(|c| {
                    json!({
                        "id": c.id,
                        "type": "function",
                        "function": {
                            "name": c.name,
                            "arguments": c.arguments
                        }
                    })
                }).collect::<Vec<_>>());
            }

            if let Some(id) = &m.tool_call_id {
                val["tool_call_id"] = json!(id);
            }

            val
        }).collect();

        let mut base_body = json!({
            "model": self.model,
            "messages": messages_val,
            "stream": true
        });

        if let Some(t) = tools {
            base_body["tools"] = json!(t.iter().map(|tool| {
                json!({
                    "type": "function",
                    "function": {
                        "name": tool.name,
                        "description": tool.description,
                        "parameters": tool.parameters
                    }
                })
            }).collect::<Vec<_>>());
        }

        let patterns = vec![
            base_body.clone(),
            {
                let mut b = base_body.clone();
                b["stream_options"] = json!({ "include_usage": true });
                b
            },
        ];

        let auth_patterns = vec![
            vec![("Authorization", format!("Bearer {}", self.api_key))],
            vec![("api-key", self.api_key.clone())],
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
                log::trace!("[LLM_REQ] Body: {}", serde_json::to_string(body).unwrap_or_default());

                let res = req.json(body).send().await
                    .map_err(|e| AppError::LlmError(e.to_string()))?;

                let status = res.status();
                if status.is_success() {
                    if format_idx_lock.is_none() {
                        *format_idx_lock = Some((p_idx, h_idx));
                    }

                    // ツールコールのデルタを集約するためのステート
                    let mut current_tool_calls: Vec<ToolCall> = Vec::new();

                    let stream = res.bytes_stream().flat_map(move |item| {
                        let mut chunks = Vec::new();
                        match item {
                            Ok(bytes) => {
                                let s = String::from_utf8_lossy(&bytes);
                                let re_data = Regex::new(r"data:\s*(.*)").unwrap();
                                for line in s.lines() {
                                    let line = line.trim();
                                    if line.is_empty() || line == "data: [DONE]" { continue; }
                                    let raw = if let Some(cap) = re_data.captures(line) { cap.get(1).map(|m| m.as_str()).unwrap_or(line) } else { line };
                                    
                                    if let Ok(chunk) = serde_json::from_str::<ChatCompletionChunk>(raw) {
                                        if let Some(choice) = chunk.choices.get(0) {
                                            if let Some(content) = &choice.delta.content {
                                                chunks.push(Ok(LlmResponseChunk::Text(content.clone())));
                                            }
                                            
                                            if let Some(tool_deltas) = &choice.delta.tool_calls {
                                                for delta in tool_deltas {
                                                    let idx = delta.index.unwrap_or(0);
                                                    while current_tool_calls.len() <= idx {
                                                        current_tool_calls.push(ToolCall { id: String::new(), name: String::new(), arguments: String::new() });
                                                    }
                                                    
                                                    if let Some(id) = &delta.id {
                                                        current_tool_calls[idx].id = id.clone();
                                                    }
                                                    if let Some(func) = &delta.function {
                                                        if let Some(name) = &func.name {
                                                            current_tool_calls[idx].name.push_str(name);
                                                        }
                                                        if let Some(args) = &func.arguments {
                                                            current_tool_calls[idx].arguments.push_str(args);
                                                        }
                                                    }
                                                    
                                                    // 全ての引数が揃った（または区切り）の判断は難しいが、
                                                    // ストリーミング終了時にまとめて返すか、
                                                    // あるいはデルタごとに中途半端なToolCallを投げる。
                                                    // ここではストリーミング終了時に判定するため、ここでの追加は行わず、
                                                    // ループ外（ストリームの最後）で処理したい。
                                                    // しかし、flat_map内なので、最後の判別が難しい。
                                                    // 代替案として、各デルタごとに「現在の状態」を通知する。
                                                    chunks.push(Ok(LlmResponseChunk::ToolCall(current_tool_calls[idx].clone())));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e) => chunks.push(Err(AppError::LlmError(e.to_string()))),
                        }
                        futures_util::stream::iter(chunks)
                    });
                    
                    log::debug!("[LLM_RES] TTFT: {}ms", start_time.elapsed().as_millis());
                    return Ok(Box::pin(stream));
                }
            }
        }
        Err(AppError::LlmError("All request patterns failed.".into()))
    }
}
