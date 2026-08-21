use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::sync::Mutex;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct AgnesCredentials {
    pub base_url: String,
    pub api_keys: Vec<String>,
}

impl AgnesCredentials {
    pub fn endpoint(&self, path: &str) -> String {
        let base = self.base_url.trim_end_matches('/');
        if base.ends_with("/v1") && path.starts_with("/v1/") {
            format!("{}{}", base.trim_end_matches("/v1"), path)
        } else if !base.ends_with("/v1") && !path.starts_with("/v1/") {
            format!("{base}/v1{path}")
        } else {
            format!("{base}{path}")
        }
    }

    /// Round-robin 选一个 Key；只有一个就直接返回
    pub fn pick_key(&self, scope: &str) -> Option<String> {
        if self.api_keys.is_empty() {
            return None;
        }
        if self.api_keys.len() == 1 {
            return Some(self.api_keys[0].clone());
        }
        let cursor = AGNES_KEY_CURSOR
            .lock()
            .ok()
            .and_then(|mut map| {
                let entry = map.entry(scope.to_string()).or_insert(0usize);
                let start = *entry % self.api_keys.len();
                *entry = (*entry + 1) % self.api_keys.len();
                Some(start)
            })
            .unwrap_or(0);
        Some(self.api_keys[cursor].clone())
    }

    /// 按 round-robin 顺序返回所有 Key（用于失败重试）
    pub fn ordered_keys(&self, scope: &str) -> Vec<String> {
        if self.api_keys.len() <= 1 {
            return self.api_keys.clone();
        }
        let start = AGNES_KEY_CURSOR
            .lock()
            .ok()
            .and_then(|mut map| {
                let entry = map.entry(scope.to_string()).or_insert(0usize);
                let s = *entry % self.api_keys.len();
                *entry = (*entry + 1) % self.api_keys.len();
                Some(s)
            })
            .unwrap_or(0);
        self.api_keys
            .iter()
            .cycle()
            .skip(start)
            .take(self.api_keys.len())
            .cloned()
            .collect()
    }
}

static AGNES_KEY_CURSOR: Lazy<Mutex<std::collections::HashMap<String, usize>>> =
    Lazy::new(|| Mutex::new(std::collections::HashMap::new()));

use once_cell::sync::Lazy;

pub fn resolve_image_credentials(_user_id: &str) -> Result<AgnesCredentials> {
    let config = crate::config::config();
    credentials_from_config(
        &config.llm_image_base_url,
        &config.llm_image_api_keys,
        &config.llm_image_api_key,
        "图片",
    )
}

pub fn resolve_video_credentials(_user_id: &str) -> Result<AgnesCredentials> {
    let config = crate::config::config();
    credentials_from_config(
        &config.llm_video_base_url,
        &config.llm_video_api_keys,
        &config.llm_video_api_key,
        "视频",
    )
}

fn credentials_from_config(
    base_url: &str,
    api_keys: &[String],
    fallback_api_key: &str,
    kind: &str,
) -> Result<AgnesCredentials> {
    let base_url = base_url.trim();
    if base_url.is_empty() {
        return Err(anyhow!("未配置 {kind} 模型的 BASE_URL"));
    }

    let mut keys: Vec<String> = api_keys
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // 去重
    keys.dedup();

    if keys.is_empty() {
        let fallback = fallback_api_key.trim();
        if fallback.is_empty() {
            return Err(anyhow!("未配置 {kind} 模型的 API_KEY"));
        }
        keys.push(fallback.to_string());
    }

    Ok(AgnesCredentials {
        base_url: base_url.to_string(),
        api_keys: keys,
    })
}

pub fn agnes_image_model() -> String {
    crate::config::config().llm_image_model.trim().to_string()
}

pub fn agnes_video_model() -> String {
    crate::config::config().llm_video_model.trim().to_string()
}

pub fn http_client(timeout: Duration) -> Result<Client> {
    Ok(Client::builder().timeout(timeout).build()?)
}

fn should_retry_agnes(status: reqwest::StatusCode) -> bool {
    status == reqwest::StatusCode::UNAUTHORIZED
        || status == reqwest::StatusCode::FORBIDDEN
        || status == reqwest::StatusCode::TOO_MANY_REQUESTS
        || status.is_server_error()
}

/// 带 API Key 负载均衡的 POST 请求
pub async fn post_json<T: DeserializeOwned>(
    client: &Client,
    url: &str,
    credentials: &AgnesCredentials,
    body: &Value,
) -> Result<T> {
    post_json_url(client, url, credentials, body).await
}

pub async fn post_json_url<T: DeserializeOwned>(
    client: &Client,
    url: &str,
    credentials: &AgnesCredentials,
    body: &Value,
) -> Result<T> {
    let keys = credentials.ordered_keys("agnes-post");
    if keys.is_empty() {
        return Err(anyhow!("Agnes API 未配置可用 API Key"));
    }

    let mut last_error: Option<anyhow::Error> = None;

    for (index, api_key) in keys.iter().enumerate() {
        let response = client
            .post(url)
            .header("Authorization", format!("Bearer {api_key}"))
            .header("Content-Type", "application/json")
            .json(body)
            .send()
            .await;

        match response {
            Ok(resp) => {
                if !resp.status().is_success() {
                    let status = resp.status();
                    let detail = resp.text().await.unwrap_or_default();
                    let err = anyhow!("Agnes API 返回错误 {status}: {detail}");
                    if should_retry_agnes(status) && index + 1 < keys.len() {
                        tracing::warn!(
                            "Agnes API key {} failed with {status}, retrying next key",
                            index
                        );
                        last_error = Some(err);
                        continue;
                    }
                    return Err(err);
                }
                return Ok(resp.json::<T>().await?);
            }
            Err(err) => {
                last_error = Some(err.into());
                if index + 1 < keys.len() {
                    continue;
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow!("Agnes API 请求失败")))
}

/// 带 API Key 负载均衡的 GET 请求
pub async fn get_json<T: DeserializeOwned>(
    client: &Client,
    url: &str,
    credentials: &AgnesCredentials,
) -> Result<T> {
    let keys = credentials.ordered_keys("agnes-get");
    if keys.is_empty() {
        return Err(anyhow!("Agnes API 未配置可用 API Key"));
    }

    let mut last_error: Option<anyhow::Error> = None;

    for (index, api_key) in keys.iter().enumerate() {
        let response = client
            .get(url)
            .header("Authorization", format!("Bearer {api_key}"))
            .send()
            .await;

        match response {
            Ok(resp) => {
                if !resp.status().is_success() {
                    let status = resp.status();
                    let detail = resp.text().await.unwrap_or_default();
                    let err = anyhow!("Agnes API 返回错误 {status}: {detail}");
                    if should_retry_agnes(status) && index + 1 < keys.len() {
                        tracing::warn!(
                            "Agnes API key {} failed with {status}, retrying next key",
                            index
                        );
                        last_error = Some(err);
                        continue;
                    }
                    return Err(err);
                }
                return Ok(resp.json::<T>().await?);
            }
            Err(err) => {
                last_error = Some(err.into());
                if index + 1 < keys.len() {
                    continue;
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow!("Agnes API 请求失败")))
}
