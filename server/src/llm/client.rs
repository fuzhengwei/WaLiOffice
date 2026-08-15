use anyhow::{anyhow, Result};
use reqwest::Client;
use std::time::Duration;
use tracing::debug;

use super::types::*;
use crate::models::{ChatAttachment, ChatMessage};

pub struct LlmClient {
    http: Client,
    base_url: String,
    api_key: String,
    model: String,
    timeout: Duration,
}

impl LlmClient {
    pub fn new() -> Self {
        let cfg = crate::config::config();
        let http = Client::builder()
            .timeout(Duration::from_millis(cfg.llm_chat_timeout_ms))
            .build()
            .expect("reqwest client build");
        Self {
            http,
            base_url: cfg.llm_base_url.trim_end_matches('/').to_string(),
            api_key: cfg.llm_api_key.clone(),
            model: cfg.llm_model.clone(),
            timeout: Duration::from_millis(cfg.llm_chat_timeout_ms),
        }
    }

    pub fn for_user(user_id: &str, preferred_model: Option<&str>) -> Self {
        let mut client = Self::new();
        let pool = crate::state::db_pool();

        if let Ok(Some(settings)) = crate::db::settings_repo::find_by_user(&pool, user_id) {
            if let Some(profile) = settings
                .llm_profiles
                .iter()
                .find(|item| item.id == settings.active_profile_id)
            {
                if !profile.base_url.trim().is_empty() {
                    client.base_url = profile.base_url.trim_end_matches('/').to_string();
                }
                if let Some(api_key) = profile
                    .api_key
                    .as_ref()
                    .filter(|item| !item.trim().is_empty())
                {
                    client.api_key = api_key.clone();
                }

                let requested_model = preferred_model.filter(|model| {
                    profile.models.iter().any(|item| item == *model)
                        && is_chat_compatible_model(model)
                });
                client.model = requested_model
                    .map(|item| item.to_string())
                    .or_else(|| {
                        if profile
                            .models
                            .iter()
                            .any(|item| item == &settings.active_model)
                            && is_chat_compatible_model(&settings.active_model)
                        {
                            Some(settings.active_model.clone())
                        } else {
                            None
                        }
                    })
                    .or_else(|| {
                        if is_chat_compatible_model(&profile.default_model) {
                            Some(profile.default_model.clone())
                        } else {
                            profile
                                .models
                                .iter()
                                .find(|item| is_chat_compatible_model(item))
                                .cloned()
                        }
                    })
                    .unwrap_or_else(|| profile.default_model.clone());
            }
        } else if let Some(model) =
            preferred_model.filter(|item| !item.trim().is_empty() && is_chat_compatible_model(item))
        {
            client.model = model.to_string();
        }

        client
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    /// 非流式 chat（带可选工具）
    pub async fn chat(
        &self,
        messages: &[ChatMessage],
        tools: Option<&[FunctionDef]>,
    ) -> Result<ChatCompletionResponse> {
        self.chat_with_attachments(messages, tools, None).await
    }

    pub async fn chat_with_attachments(
        &self,
        messages: &[ChatMessage],
        tools: Option<&[FunctionDef]>,
        user_attachments: Option<&[ChatAttachment]>,
    ) -> Result<ChatCompletionResponse> {
        let has_image_attachments = has_image_attachments(user_attachments);
        let req = ChatCompletionRequest {
            model: self.model.clone(),
            messages: build_request_messages(messages, user_attachments),
            tools: tools.map(|t| t.to_vec()),
            tool_choice: None,
            temperature: Some(0.7),
            stream: Some(false),
        };

        match self.send_chat_request(&req).await {
            Ok(result) => Ok(result),
            Err(err) if has_image_attachments && tools.is_some() => {
                tracing::warn!(
                    "vision request with tools failed, retrying vision chat without tools: {err}"
                );
                let vision_only_req = ChatCompletionRequest {
                    model: self.model.clone(),
                    messages: build_request_messages(messages, user_attachments),
                    tools: None,
                    tool_choice: None,
                    temperature: Some(0.7),
                    stream: Some(false),
                };

                match self.send_chat_request(&vision_only_req).await {
                    Ok(result) => Ok(result),
                    Err(vision_err) => {
                        tracing::warn!(
                            "vision-only retry failed, retrying text-only chat: {vision_err}"
                        );
                        self.retry_text_only_after_vision_failure(messages, tools)
                            .await
                    }
                }
            }
            Err(err) if has_image_attachments => {
                tracing::warn!("vision request failed, retrying text-only chat: {err}");
                self.retry_text_only_after_vision_failure(messages, tools)
                    .await
            }
            Err(err) => Err(err),
        }
    }

    async fn retry_text_only_after_vision_failure(
        &self,
        messages: &[ChatMessage],
        tools: Option<&[FunctionDef]>,
    ) -> Result<ChatCompletionResponse> {
        let mut fallback_messages = messages.to_vec();
        fallback_messages.push(ChatMessage {
            role: "system".to_string(),
            content: "当前模型或接口暂未成功处理本次图片视觉输入。不要假装已经看到了图片内容；请明确告知用户当前限制，并优先依据 OCR 文本、图片中的关键区域说明或用户补充描述继续回答。".to_string(),
            tool_calls: None,
            tool_call_id: None,
        });
        let fallback_req = ChatCompletionRequest {
            model: self.model.clone(),
            messages: build_request_messages(&fallback_messages, None),
            tools: tools.map(|t| t.to_vec()),
            tool_choice: None,
            temperature: Some(0.7),
            stream: Some(false),
        };
        self.send_chat_request(&fallback_req).await
    }

    async fn send_chat_request(
        &self,
        req: &ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse> {
        let url = format!("{}/chat/completions", self.base_url);
        debug!("LLM chat request to {url}");

        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .timeout(self.timeout)
            .json(req)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("LLM 返回错误 {status}: {body}"));
        }

        let result: ChatCompletionResponse = resp.json().await?;
        Ok(result)
    }

    /// 提取 JSON（容错：去 markdown fence、截取首尾花括号）
    pub fn extract_json(text: &str) -> Result<serde_json::Value> {
        let cleaned = text
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        if let Ok(v) = serde_json::from_str::<serde_json::Value>(cleaned) {
            return Ok(v);
        }

        // 尝试截取第一个 { 到最后一个 }
        if let (Some(start), Some(end)) = (cleaned.find('{'), cleaned.rfind('}')) {
            if end > start {
                if let Ok(v) = serde_json::from_str(&cleaned[start..=end]) {
                    return Ok(v);
                }
            }
        }
        // 尝试数组
        if let (Some(start), Some(end)) = (cleaned.find('['), cleaned.rfind(']')) {
            if end > start {
                if let Ok(v) = serde_json::from_str(&cleaned[start..=end]) {
                    return Ok(v);
                }
            }
        }
        Err(anyhow!("模型未返回可解析 JSON"))
    }
}

fn build_request_messages(
    messages: &[ChatMessage],
    user_attachments: Option<&[ChatAttachment]>,
) -> Vec<RequestMessage> {
    let latest_user_index = user_attachments
        .filter(|items| has_image_attachments(Some(items)))
        .and_then(|_| messages.iter().rposition(|msg| msg.role == "user"));

    messages
        .iter()
        .enumerate()
        .map(
            |(index, msg)| match (Some(index) == latest_user_index, user_attachments) {
                (true, Some(attachments)) => {
                    RequestMessage::from_multimodal_user_message(msg, attachments)
                }
                _ => RequestMessage::from_chat_message(msg),
            },
        )
        .collect()
}

fn has_image_attachments(user_attachments: Option<&[ChatAttachment]>) -> bool {
    user_attachments
        .map(|items| {
            items.iter().any(|item| {
                item.kind == "image"
                    && item
                        .data_url
                        .as_deref()
                        .map(|value| !value.trim().is_empty())
                        .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

fn is_chat_compatible_model(model: &str) -> bool {
    let normalized = model.trim().to_lowercase();
    !(normalized.starts_with("agnes-image-") || normalized.starts_with("agnes-video-"))
}
