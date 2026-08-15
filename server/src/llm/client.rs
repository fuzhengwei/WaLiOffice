use anyhow::{anyhow, Result};
use reqwest::Client;
use std::time::Duration;
use tracing::debug;

use super::types::*;
use crate::models::ChatMessage;

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
                if let Some(api_key) = profile.api_key.as_ref().filter(|item| !item.trim().is_empty()) {
                    client.api_key = api_key.clone();
                }

                let requested_model = preferred_model.filter(|model| profile.models.iter().any(|item| item == *model));
                client.model = requested_model
                    .map(|item| item.to_string())
                    .or_else(|| {
                        if profile.models.iter().any(|item| item == &settings.active_model) {
                            Some(settings.active_model.clone())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| profile.default_model.clone());
            }
        } else if let Some(model) = preferred_model.filter(|item| !item.trim().is_empty()) {
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
        let req = ChatCompletionRequest {
            model: self.model.clone(),
            messages: messages.to_vec(),
            tools: tools.map(|t| t.to_vec()),
            tool_choice: None,
            temperature: Some(0.7),
            stream: Some(false),
        };

        let url = format!("{}/chat/completions", self.base_url);
        debug!("LLM chat request to {url}");

        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .timeout(self.timeout)
            .json(&req)
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
        let cleaned = text.trim()
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
