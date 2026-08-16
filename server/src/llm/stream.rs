use anyhow::{anyhow, Result};
use eventsource_stream::Eventsource;
use futures::StreamExt;
use reqwest::Client;
use std::sync::Mutex;
use std::time::Duration;
use tracing::debug;

use super::types::*;
use crate::models::ChatMessage;

/// 流式事件：逐 token 内容 + 工具调用增量 + 结束
#[derive(Debug, Clone)]
pub enum StreamEvent {
    Delta(String),
    ToolCallDelta {
        index: usize,
        id: Option<String>,
        name: Option<String>,
        arguments: Option<String>,
    },
    Done,
}

static STREAM_KEY_CURSOR: Mutex<usize> = Mutex::new(0);

fn should_retry_stream(status: reqwest::StatusCode) -> bool {
    status == reqwest::StatusCode::UNAUTHORIZED
        || status == reqwest::StatusCode::FORBIDDEN
        || status == reqwest::StatusCode::TOO_MANY_REQUESTS
        || status.is_server_error()
}

pub struct LlmStreamClient {
    http: Client,
    base_url: String,
    api_keys: Vec<String>,
    model: String,
    timeout: Duration,
}

impl LlmStreamClient {
    pub fn new() -> Self {
        let cfg = crate::config::config();
        let http = Client::builder()
            .timeout(Duration::from_millis(cfg.llm_chat_timeout_ms))
            .build()
            .expect("reqwest client build");
        let api_keys = if cfg.llm_api_keys.is_empty() {
            let key = cfg.llm_api_key.trim();
            if key.is_empty() {
                vec![]
            } else {
                vec![key.to_string()]
            }
        } else {
            cfg.llm_api_keys.clone()
        };
        Self {
            http,
            base_url: cfg.llm_base_url.trim_end_matches('/').to_string(),
            api_keys,
            model: cfg.llm_model.clone(),
            timeout: Duration::from_millis(cfg.llm_chat_timeout_ms),
        }
    }

    /// 流式 chat，返回事件流
    pub async fn stream_chat(
        &self,
        messages: Vec<ChatMessage>,
        tools: Option<Vec<FunctionDef>>,
    ) -> Result<tokio::sync::mpsc::Receiver<StreamEvent>> {
        let req = ChatCompletionRequest {
            model: self.model.clone(),
            messages: messages
                .iter()
                .map(RequestMessage::from_chat_message)
                .collect(),
            tools,
            tool_choice: None,
            temperature: Some(0.7),
            stream: Some(true),
        };

        let url = format!("{}/chat/completions", self.base_url);
        debug!("LLM stream request to {url}");

        let keys = if self.api_keys.is_empty() {
            vec![String::new()]
        } else {
            self.api_keys.clone()
        };

        // Round-robin: rotate starting position
        let start_index = if keys.len() > 1 {
            STREAM_KEY_CURSOR
                .lock()
                .map(|mut cursor| {
                    let start = *cursor % keys.len();
                    *cursor = (*cursor + 1) % keys.len();
                    start
                })
                .unwrap_or(0)
        } else {
            0
        };

        let mut last_error: Option<anyhow::Error> = None;
        let mut resp = None;

        for offset in 0..keys.len() {
            let index = (start_index + offset) % keys.len();
            let api_key = &keys[index];

            let attempt = self
                .http
                .post(&url)
                .header("Authorization", format!("Bearer {api_key}"))
                .header("Content-Type", "application/json")
                .timeout(self.timeout)
                .json(&req)
                .send()
                .await;

            match attempt {
                Ok(response) => {
                    if !response.status().is_success() {
                        let status = response.status();
                        let body = response.text().await.unwrap_or_default();
                        let err = anyhow!("LLM 流式返回错误 {status}: {body}");
                        if should_retry_stream(status) && offset + 1 < keys.len() {
                            tracing::warn!(
                                "LLM stream key {} failed with {status}, retrying next key",
                                index
                            );
                            last_error = Some(err);
                            continue;
                        }
                        return Err(err);
                    }
                    resp = Some(response);
                    break;
                }
                Err(err) => {
                    last_error = Some(err.into());
                    if offset + 1 < keys.len() {
                        continue;
                    }
                    break;
                }
            }
        }

        let resp = resp.ok_or_else(|| {
            last_error
                .map(|e| e)
                .unwrap_or_else(|| anyhow!("当前模型服务未配置可用 API Key"))
        })?;

        let (tx, rx) = tokio::sync::mpsc::channel(128);
        let byte_stream = resp.bytes_stream().eventsource();

        tokio::spawn(async move {
            let mut stream = byte_stream;
            while let Some(event_result) = stream.next().await {
                match event_result {
                    Ok(event) => {
                        if event.data == "[DONE]" {
                            let _ = tx.send(StreamEvent::Done).await;
                            break;
                        }
                        if let Ok(chunk) = serde_json::from_str::<StreamChunk>(&event.data) {
                            for choice in chunk.choices {
                                let delta = choice.delta;
                                if let Some(content) = delta.content {
                                    if !content.is_empty() {
                                        if tx.send(StreamEvent::Delta(content)).await.is_err() {
                                            return;
                                        }
                                    }
                                }
                                if let Some(tool_calls) = delta.tool_calls {
                                    for tc in tool_calls {
                                        let _ = tx
                                            .send(StreamEvent::ToolCallDelta {
                                                index: tc.index,
                                                id: tc.id,
                                                name: tc
                                                    .function
                                                    .as_ref()
                                                    .and_then(|f| f.name.clone()),
                                                arguments: tc
                                                    .function
                                                    .as_ref()
                                                    .and_then(|f| f.arguments.clone()),
                                            })
                                            .await;
                                    }
                                }
                                if choice.finish_reason.is_some() {
                                    let _ = tx.send(StreamEvent::Done).await;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("SSE parse error: {e}");
                    }
                }
            }
            // 确保发送 Done
            let _ = tx.send(StreamEvent::Done).await;
        });

        Ok(rx)
    }
}
