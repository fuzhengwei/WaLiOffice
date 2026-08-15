use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::time::Duration;

pub const AGNES_API_BASE: &str = "https://apihub.agnes-ai.com";
pub const AGNES_IMAGE_MODEL: &str = "agnes-image-2.1-flash";
pub const AGNES_VIDEO_MODEL: &str = "agnes-video-v2.0";

#[derive(Debug, Clone)]
pub struct AgnesCredentials {
    pub base_url: String,
    pub api_key: String,
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
}

pub fn resolve_credentials(user_id: &str) -> Result<AgnesCredentials> {
    let pool = crate::state::db_pool();
    if let Ok(Some(settings)) = crate::db::settings_repo::find_by_user(&pool, user_id) {
        if let Some(profile) = settings
            .llm_profiles
            .iter()
            .find(|item| item.id == settings.active_profile_id)
        {
            if let Some(api_key) = profile
                .api_key
                .as_ref()
                .map(|item| item.trim())
                .filter(|item| !item.is_empty())
            {
                let base_url = if profile.base_url.trim().is_empty() {
                    crate::config::config().llm_base_url.trim().to_string()
                } else {
                    profile.base_url.trim().to_string()
                };
                return Ok(AgnesCredentials {
                    base_url,
                    api_key: api_key.to_string(),
                });
            }
        }
    }

    let config = crate::config::config();
    let api_key = config.llm_api_key.trim().to_string();
    if api_key.is_empty() {
        Err(anyhow!("当前模型服务未配置 Agnes 可用的 API Key"))
    } else {
        Ok(AgnesCredentials {
            base_url: config.llm_base_url.trim().to_string(),
            api_key,
        })
    }
}

pub fn resolve_api_key(user_id: &str) -> Result<String> {
    Ok(resolve_credentials(user_id)?.api_key)
}

pub fn http_client(timeout: Duration) -> Result<Client> {
    Ok(Client::builder().timeout(timeout).build()?)
}

pub async fn post_json<T: DeserializeOwned>(
    client: &Client,
    path: &str,
    api_key: &str,
    body: &Value,
) -> Result<T> {
    let url = format!("{AGNES_API_BASE}{path}");
    post_json_url(client, &url, api_key, body).await
}

pub async fn post_json_url<T: DeserializeOwned>(
    client: &Client,
    url: &str,
    api_key: &str,
    body: &Value,
) -> Result<T> {
    let response = client
        .post(url)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .json(body)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let detail = response.text().await.unwrap_or_default();
        return Err(anyhow!("Agnes API 返回错误 {status}: {detail}"));
    }

    Ok(response.json::<T>().await?)
}

pub async fn get_json<T: DeserializeOwned>(client: &Client, url: &str, api_key: &str) -> Result<T> {
    let response = client
        .get(url)
        .header("Authorization", format!("Bearer {api_key}"))
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let detail = response.text().await.unwrap_or_default();
        return Err(anyhow!("Agnes API 返回错误 {status}: {detail}"));
    }

    Ok(response.json::<T>().await?)
}
