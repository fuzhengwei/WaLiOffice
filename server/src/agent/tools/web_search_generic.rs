use async_trait::async_trait;
use regex::Regex;
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;

use crate::agent::tool::{OfficeTool, ToolArtifact, ToolContext, ToolResult};

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

#[derive(Debug, Clone, Copy)]
enum SearchProvider {
    Searxng,
    DuckDuckGo,
}

impl SearchProvider {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Searxng => "searxng",
            Self::DuckDuckGo => "duckduckgo",
        }
    }

    fn label(&self) -> &'static str {
        match self {
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
        "联网检索网页信息：根据关键词搜索互联网公开网页，返回标题、链接和摘要。"
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

        match search_web(query, max_results).await {
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
                    continue_loop: None,
                }
            }
            Err(err) => ToolResult::err(format!("联网检索失败: {err}")),
        }
    }
}

async fn search_web(query: &str, max_results: usize) -> anyhow::Result<SearchOutcome> {
    let cfg = crate::config::config();
    let client = Client::builder()
        .timeout(Duration::from_millis(cfg.web_search_timeout_ms))
        .user_agent("Mozilla/5.0 (compatible; WaLiOffice/0.2; +https://localhost)")
        .build()?;
    let provider = cfg.web_search_provider.trim().to_lowercase();
    let attempts = match provider.as_str() {
        "searxng" => vec![SearchProvider::Searxng],
        "duckduckgo" => vec![SearchProvider::DuckDuckGo],
        _ => vec![SearchProvider::Searxng, SearchProvider::DuckDuckGo],
    };

    let mut tried = Vec::new();
    for attempt in attempts {
        tried.push(attempt);
        let result = match attempt {
            SearchProvider::Searxng => search_with_searxng(&client, query, max_results, &cfg.web_search_endpoint).await,
            SearchProvider::DuckDuckGo => search_with_duckduckgo(&client, query, max_results).await,
        };
        if let Ok(items) = result {
            if !items.is_empty() {
                return Ok(SearchOutcome { provider: attempt, items, providers_tried: tried });
            }
        }
    }
    Ok(SearchOutcome {
        provider: tried.first().copied().unwrap_or(SearchProvider::DuckDuckGo),
        items: Vec::new(),
        providers_tried: tried,
    })
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
