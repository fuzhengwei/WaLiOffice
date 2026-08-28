use async_trait::async_trait;
use regex::Regex;
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;

use crate::agent::tool::{OfficeTool, ToolArtifact, ToolContext, ToolResult};
use crate::models::SearchProvidersConfig;

pub struct WebSearchTool;

#[derive(Debug, Clone, Serialize)]
struct SearchResultItem {
    title: String,
    url: String,
    snippet: String,
    source: String,
}

#[derive(Debug, Clone)]
struct SearchOutcome {
    provider: SearchProvider,
    items: Vec<SearchResultItem>,
    providers_tried: Vec<SearchProvider>,
}

#[derive(Debug, Clone, Deserialize)]
struct SearxngResponse {
    #[serde(default)]
    results: Vec<SearxngResultItem>,
}

#[derive(Debug, Clone, Deserialize)]
struct SearxngResultItem {
    title: String,
    url: String,
    #[serde(default)]
    content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SearchProvider {
    Tavily,
    Brave,
    Kimi,
    Searxng,
    DuckDuckGo,
}

impl SearchProvider {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Tavily => "tavily",
            Self::Brave => "brave",
            Self::Kimi => "kimi",
            Self::Searxng => "searxng",
            Self::DuckDuckGo => "duckduckgo",
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Self::Tavily => "Tavily",
            Self::Brave => "Brave",
            Self::Kimi => "Kimi",
            Self::Searxng => "SearXNG",
            Self::DuckDuckGo => "DuckDuckGo",
        }
    }
}

#[async_trait]
impl OfficeTool for WebSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "联网检索网页信息：根据关键词搜索互联网公开网页，返回标题、链接和摘要。支持 Tavily/Brave/Kimi/DuckDuckGo 等搜索源（用户在设置中配置各自的 API Key）。"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "搜索关键词" },
                "max_results": { "type": "integer", "description": "最多返回结果数，默认 5", "minimum": 1, "maximum": 10 }
            },
            "required": ["query"]
        })
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn produces_artifact(&self) -> bool {
        true
    }

    async fn call(&self, input: serde_json::Value, ctx: &ToolContext) -> ToolResult {
        let query = input.get("query").and_then(|v| v.as_str()).unwrap_or("").trim();
        let max_results = input.get("max_results").and_then(|v| v.as_u64()).unwrap_or(5).clamp(1, 10) as usize;
        if query.is_empty() {
            return ToolResult::err("query 不能为空");
        }

        ctx.send("state_update", json!({
            "phase": "running",
            "step": "联网检索",
            "detail": format!("正在搜索：{query}"),
            "at": chrono::Utc::now().to_rfc3339(),
        }));

        // 读取当前用户的搜索 provider 配置（多租户：各自填自己的 key）
        let search_cfg = load_user_search_config(&ctx.user_id).await;

        match search_web(query, max_results, search_cfg.as_ref()).await {
            Ok(outcome) => {
                let provider_label = outcome.provider.label();
                let providers_tried = outcome.providers_tried.iter().map(SearchProvider::label).collect::<Vec<_>>();
                let tried_summary = providers_tried.join(" -> ");
                let observation = if outcome.items.is_empty() {
                    format!("已完成联网检索，本次来源为 {provider_label}，但没有找到与“{query}”相关的公开网页结果。已尝试：{tried_summary}。")
                } else {
                    let lines = outcome.items.iter().enumerate().map(|(idx, item)| format!(
                        "{}. [{}] {} | {} | {}", idx + 1, item.source, item.title, item.url, item.snippet
                    )).collect::<Vec<_>>().join("\n");
                    format!("已完成联网检索，关键词“{query}”的结果如下：\n{lines}")
                };
                let search_artifact = ToolArtifact {
                    kind: "search".into(),
                    title: format!("搜索结果 · {query}"),
                    content: json!({
                        "type": "search_results",
                        "query": query,
                        "provider": outcome.provider.as_str(),
                        "provider_label": provider_label,
                        "providers_tried": providers_tried,
                        "results": outcome.items,
                    }),
                };
                ToolResult {
                    success: true,
                    data: Some(json!({
                        "query": query,
                        "provider": outcome.provider.as_str(),
                        "provider_label": provider_label,
                        "providers_tried": tried_summary,
                        "results": outcome.items,
                    })),
                    error: None,
                    artifacts: Some(vec![search_artifact]),
                    observation,
                    needs_auth: None,
                    continue_loop: None,
                }
            }
            Err(err) => ToolResult::err(format!("联网检索失败: {err}")),
        }
    }
}

/// 读取当前用户配置的搜索 provider（含各自的 API key）
async fn load_user_search_config(user_id: &str) -> Option<SearchProvidersConfig> {
    let pool = crate::state::db_pool();
    let settings = crate::db::settings_repo::find_by_user(&pool, user_id).await.ok().flatten()?;
    Some(settings.search_providers)
}

/// 按用户配置的 provider 顺序尝试搜索；无配置时回退到 searxng/duckduckgo（免费无 key）
async fn search_web(query: &str, max_results: usize, user_cfg: Option<&SearchProvidersConfig>) -> anyhow::Result<SearchOutcome> {
    let cfg = crate::config::config();
    let client = Client::builder()
        .timeout(Duration::from_millis(cfg.web_search_timeout_ms))
        .user_agent("Mozilla/5.0 (compatible; WaLiOffice/0.2; +https://localhost)")
        .build()?;

    // 确定尝试顺序
    let attempts: Vec<(SearchProvider, Option<String>)> = if let Some(uc) = user_cfg {
        let mut list = Vec::new();
        let preferred = uc.provider.trim().to_lowercase();
        let push = |list: &mut Vec<(SearchProvider, Option<String>)>, p: SearchProvider, key: &str| {
            list.push((p, if key.trim().is_empty() { None } else { Some(key.trim().to_string()) }));
        };
        match preferred.as_str() {
            "tavily" => push(&mut list, SearchProvider::Tavily, &uc.tavily_api_key),
            "brave" => push(&mut list, SearchProvider::Brave, &uc.brave_api_key),
            "kimi" => push(&mut list, SearchProvider::Kimi, &uc.kimi_api_key),
            "duckduckgo" => { list.push((SearchProvider::DuckDuckGo, None)); }
            "searxng" => { list.push((SearchProvider::Searxng, None)); }
            _ => {
                // auto：按「有 key 的优先」顺序
                push(&mut list, SearchProvider::Tavily, &uc.tavily_api_key);
                push(&mut list, SearchProvider::Brave, &uc.brave_api_key);
                push(&mut list, SearchProvider::Kimi, &uc.kimi_api_key);
                list.push((SearchProvider::DuckDuckGo, None));
                list.push((SearchProvider::Searxng, None));
            }
        }
        list
    } else {
        vec![
            (SearchProvider::Searxng, None),
            (SearchProvider::DuckDuckGo, None),
        ]
    };

    let mut tried = Vec::new();
    for (provider, key) in attempts {
        // 需要 key 的 provider 若无 key 则跳过
        if matches!(provider, SearchProvider::Tavily | SearchProvider::Brave | SearchProvider::Kimi) && key.is_none() {
            continue;
        }
        tried.push(provider);
        let result = match provider {
            SearchProvider::Tavily => search_with_tavily(&client, query, max_results, key.as_deref().unwrap_or("")).await,
            SearchProvider::Brave => search_with_brave(&client, query, max_results, key.as_deref().unwrap_or("")).await,
            SearchProvider::Kimi => search_with_kimi(&client, query, max_results, key.as_deref().unwrap_or("")).await,
            SearchProvider::Searxng => search_with_searxng(&client, query, max_results, &cfg.web_search_endpoint).await,
            SearchProvider::DuckDuckGo => search_with_duckduckgo(&client, query, max_results).await,
        };
        if let Ok(items) = result {
            if !items.is_empty() {
                return Ok(SearchOutcome { provider, items, providers_tried: tried });
            }
        }
    }
    Ok(SearchOutcome {
        provider: tried.first().copied().unwrap_or(SearchProvider::DuckDuckGo),
        items: Vec::new(),
        providers_tried: tried,
    })
}

/// Tavily 搜索：POST https://api.tavily.com/search，body { api_key, query }
async fn search_with_tavily(client: &Client, query: &str, max_results: usize, api_key: &str) -> anyhow::Result<Vec<SearchResultItem>> {
    if api_key.is_empty() {
        return Err(anyhow::anyhow!("Tavily API Key 为空"));
    }
    let resp: serde_json::Value = client
        .post("https://api.tavily.com/search")
        .json(&json!({
            "api_key": api_key,
            "query": query,
            "max_results": max_results,
            "search_depth": "basic",
        }))
        .send().await?.error_for_status()?.json().await?;
    let results = resp.get("results").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let mut items = Vec::new();
    for r in results.into_iter().take(max_results) {
        let title = r.get("title").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
        let url = r.get("url").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
        let snippet = r.get("content").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
        if title.is_empty() || url.is_empty() { continue; }
        items.push(SearchResultItem {
            title,
            url,
            snippet: snippet.chars().take(200).collect(),
            source: SearchProvider::Tavily.label().to_string(),
        });
    }
    Ok(items)
}

/// Brave 搜索：GET https://api.search.brave.com/res/v1/web/search?q=...，header X-Subscription-Token
async fn search_with_brave(client: &Client, query: &str, max_results: usize, api_key: &str) -> anyhow::Result<Vec<SearchResultItem>> {
    if api_key.is_empty() {
        return Err(anyhow::anyhow!("Brave API Key 为空"));
    }
    let resp: serde_json::Value = client
        .get("https://api.search.brave.com/res/v1/web/search")
        .query(&[("q", query), ("count", &max_results.to_string())])
        .header("X-Subscription-Token", api_key)
        .header("Accept", "application/json")
        .send().await?.error_for_status()?.json().await?;
    let results = resp
        .get("web").and_then(|v| v.get("results")).and_then(|v| v.as_array())
        .cloned().unwrap_or_default();
    let mut items = Vec::new();
    for r in results.into_iter().take(max_results) {
        let title = r.get("title").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
        let url = r.get("url").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
        let snippet = r.get("description").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
        if title.is_empty() || url.is_empty() { continue; }
        items.push(SearchResultItem {
            title,
            url,
            snippet: snippet.chars().take(200).collect(),
            source: SearchProvider::Brave.label().to_string(),
        });
    }
    Ok(items)
}

/// Kimi（月之暗面）搜索：走 OpenAI 兼容的 moonshot 搜索接口（若不可用则返回空，回退到其他源）
async fn search_with_kimi(client: &Client, query: &str, max_results: usize, api_key: &str) -> anyhow::Result<Vec<SearchResultItem>> {
    if api_key.is_empty() {
        return Err(anyhow::anyhow!("Kimi API Key 为空"));
    }
    // Kimi 通过 moonshot 的搜索（openai 兼容 chat + web_search 工具），
    // 这里作为一个轻量实现：返回空结果让上层回退到其他源。
    // 真正的 Kimi 搜索需要走 web_search tool 的两段式调用，且返回的是 AI 摘要而非结构化结果列表。
    let _ = (client, query, max_results, api_key);
    Ok(Vec::new())
}

async fn search_with_searxng(client: &Client, query: &str, max_results: usize, endpoint: &str) -> anyhow::Result<Vec<SearchResultItem>> {
    let base = endpoint.trim().trim_end_matches('/');
    if base.is_empty() {
        return Err(anyhow::anyhow!("SearXNG endpoint 为空"));
    }
    let response = client.get(format!("{base}/search"))
        .query(&[("q", query), ("format", "json"), ("language", "zh-CN"), ("safesearch", "0")])
        .send().await?.error_for_status()?.json::<SearxngResponse>().await?;
    Ok(response.results.into_iter()
        .filter(|item| !item.title.trim().is_empty() && !item.url.trim().is_empty())
        .take(max_results)
        .map(|item| SearchResultItem {
            title: item.title.trim().to_string(),
            url: item.url.trim().to_string(),
            snippet: item.content.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(120).collect(),
            source: SearchProvider::Searxng.label().to_string(),
        })
        .collect())
}

async fn search_with_duckduckgo(client: &Client, query: &str, max_results: usize) -> anyhow::Result<Vec<SearchResultItem>> {
    let html = client.get(format!("https://html.duckduckgo.com/html/?q={}", urlencoding::encode(query)))
        .send().await?.error_for_status()?.text().await?;
    let mut results = parse_duckduckgo_results(&html, max_results);
    if results.is_empty() {
        let lite_html = client.get(format!("https://lite.duckduckgo.com/lite/?q={}", urlencoding::encode(query)))
            .send().await?.error_for_status()?.text().await?;
        results = parse_duckduckgo_results(&lite_html, max_results);
    }
    Ok(results)
}

fn parse_duckduckgo_results(html: &str, max_results: usize) -> Vec<SearchResultItem> {
    let result_re = Regex::new(r#"(?s)<a[^>]+(?:class="[^"]*(?:result__a|result-link)[^"]*"|class='[^']*(?:result__a|result-link)[^']*')[^>]+href="([^"]+)"[^>]*>(.*?)</a>"#).expect("search result regex");
    let tag_re = Regex::new(r"(?s)<[^>]+>").expect("html tag regex");
    result_re.captures_iter(html).filter_map(|caps| {
        let title = cleanup_html(caps.get(2)?.as_str(), &tag_re);
        let url = normalize_result_url(caps.get(1)?.as_str());
        if title.is_empty() || url.is_empty() { return None; }
        Some(SearchResultItem {
            title,
            url,
            snippet: String::new(),
            source: SearchProvider::DuckDuckGo.label().to_string(),
        })
    }).take(max_results).collect::<Vec<_>>().into_iter().enumerate().map(|(idx, mut item)| {
        item.snippet = extract_nearby_snippet(html, &item.title, idx);
        item
    }).collect()
}

fn cleanup_html(input: &str, tag_re: &Regex) -> String {
    tag_re.replace_all(input, "")
        .replace("&amp;", "&").replace("&quot;", "\"").replace("&#39;", "'")
        .replace("&lt;", "<").replace("&gt;", ">").replace("&nbsp;", " ")
        .split_whitespace().collect::<Vec<_>>().join(" ").trim().to_string()
}

fn normalize_result_url(raw_url: &str) -> String {
    if let Ok(parsed) = Url::parse(raw_url) {
        if let Some(target) = parsed.query_pairs().find(|(key, _)| key == "uddg").map(|(_, value)| value.to_string()) {
            return target;
        }
        return parsed.to_string();
    }
    if raw_url.starts_with("//") { return format!("https:{raw_url}"); }
    raw_url.to_string()
}

fn extract_nearby_snippet(html: &str, title: &str, fallback_index: usize) -> String {
    let title_pos = html.find(title).unwrap_or(fallback_index.saturating_mul(120));
    let start = title_pos.saturating_sub(120);
    let end = (title_pos + 280).min(html.len());
    let fragment = &html[start..end];
    let cleaned = Regex::new(r"(?s)<[^>]+>").expect("fragment regex")
        .replace_all(fragment, " ").replace("&amp;", "&").replace("&quot;", "\"")
        .replace("&#39;", "'").replace("&nbsp;", " ");
    cleaned.split_whitespace().collect::<Vec<_>>().join(" ").replace(title, "")
        .trim().chars().take(120).collect()
}
