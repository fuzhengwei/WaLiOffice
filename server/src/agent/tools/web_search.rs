use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures::StreamExt;
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

#[async_trait]
impl OfficeTool for WebSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "联网检索网页信息：根据关键词搜索互联网公开网页，返回标题、链接和摘要。适合查询最新资料、官网说明、新闻动态、政策信息和竞品信息。"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "搜索关键词，尽量具体，如：'OpenAI Responses API 2026 官方文档'" },
                "max_results": { "type": "integer", "description": "最多返回结果数，默认 5，建议 3-8", "minimum": 1, "maximum": 10 }
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
        let query = input
            .get("query")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        let max_results = input
            .get("max_results")
            .and_then(|v| v.as_u64())
            .unwrap_or(5)
            .clamp(1, 10) as usize;

        if query.is_empty() {
            return ToolResult::err("query 不能为空");
        }

        ctx.send(
            "state_update",
            json!({
                "phase": "running",
                "step": "联网检索",
                "detail": format!("正在搜索：{query}"),
                "at": chrono::Utc::now().to_rfc3339(),
            }),
        );

        match search_web(query, max_results).await {
            Ok(outcome) => {
                let provider_label = outcome.provider.label();
                let providers_tried = outcome
                    .providers_tried
                    .iter()
                    .map(SearchProvider::label)
                    .collect::<Vec<_>>();
                let tried_summary = providers_tried.join(" -> ");

                ctx.send(
                    "state_update",
                    json!({
                        "phase": "running",
                        "step": "检索来源",
                        "detail": format!("本次使用 {provider_label}，检索链路：{tried_summary}"),
                        "provider": outcome.provider.as_str(),
                        "provider_label": provider_label,
                        "providers_tried": providers_tried,
                        "at": chrono::Utc::now().to_rfc3339(),
                    }),
                );

                let observation = if outcome.items.is_empty() {
                    format!("已完成联网检索，本次来源为 {provider_label}，但没有找到与“{query}”相关的公开网页结果。已尝试：{tried_summary}。")
                } else {
                    let lines = outcome
                        .items
                        .iter()
                        .enumerate()
                        .map(|(idx, item)| {
                            format!(
                                "{}. [{}] {} | {} | {}",
                                idx + 1,
                                item.source,
                                item.title,
                                item.url,
                                item.snippet
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
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
    match provider.as_str() {
        "baidu_mcp" => search_with_baidu_mcp(
            &client,
            query,
            max_results,
            &cfg.baidu_mcp_sse_endpoint,
            &cfg.baidu_mcp_api_key,
        )
        .await
        .map(|items| SearchOutcome {
            provider: SearchProvider::BaiduMcp,
            items,
            providers_tried: vec![SearchProvider::BaiduMcp],
        }),
        "baidu" => search_with_baidu(&client, query, max_results)
            .await
            .map(|items| SearchOutcome {
                provider: SearchProvider::Baidu,
                items,
                providers_tried: vec![SearchProvider::Baidu],
            }),
        "searxng" => search_with_searxng(&client, query, max_results, &cfg.web_search_endpoint)
            .await
            .map(|items| SearchOutcome {
                provider: SearchProvider::Searxng,
                items,
                providers_tried: vec![SearchProvider::Searxng],
            }),
        "duckduckgo" => search_with_duckduckgo(&client, query, max_results)
            .await
            .map(|items| SearchOutcome {
                provider: SearchProvider::DuckDuckGo,
                items,
                providers_tried: vec![SearchProvider::DuckDuckGo],
            }),
        _ => {
            let mut attempts = Vec::new();
            if contains_cjk(query) {
                if !cfg.baidu_mcp_api_key.trim().is_empty() {
                    attempts.push(SearchProvider::BaiduMcp);
                }
                attempts.push(SearchProvider::Baidu);
                attempts.push(SearchProvider::Searxng);
                attempts.push(SearchProvider::DuckDuckGo);
            } else {
                attempts.push(SearchProvider::Searxng);
                attempts.push(SearchProvider::DuckDuckGo);
                if !cfg.baidu_mcp_api_key.trim().is_empty() {
                    attempts.push(SearchProvider::BaiduMcp);
                }
                attempts.push(SearchProvider::Baidu);
            }

            let mut tried = Vec::new();
            for attempt in attempts {
                tried.push(attempt);
                let result = match attempt {
                    SearchProvider::BaiduMcp => {
                        search_with_baidu_mcp(
                            &client,
                            query,
                            max_results,
                            &cfg.baidu_mcp_sse_endpoint,
                            &cfg.baidu_mcp_api_key,
                        )
                        .await
                    }
                    SearchProvider::Baidu => search_with_baidu(&client, query, max_results).await,
                    SearchProvider::Searxng => {
                        search_with_searxng(&client, query, max_results, &cfg.web_search_endpoint)
                            .await
                    }
                    SearchProvider::DuckDuckGo => {
                        search_with_duckduckgo(&client, query, max_results).await
                    }
                };
                if let Ok(results) = result {
                    if !results.is_empty() {
                        return Ok(SearchOutcome {
                            provider: attempt,
                            items: results,
                            providers_tried: tried,
                        });
                    }
                }
            }
            Ok(SearchOutcome {
                provider: tried.first().copied().unwrap_or(SearchProvider::DuckDuckGo),
                items: Vec::new(),
                providers_tried: tried,
            })
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum SearchProvider {
    BaiduMcp,
    Baidu,
    Searxng,
    DuckDuckGo,
}

impl SearchProvider {
    fn as_str(&self) -> &'static str {
        match self {
            Self::BaiduMcp => "baidu_mcp",
            Self::Baidu => "baidu",
            Self::Searxng => "searxng",
            Self::DuckDuckGo => "duckduckgo",
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Self::BaiduMcp => "百度 AI Search MCP",
            Self::Baidu => "百度搜索",
            Self::Searxng => "SearXNG",
            Self::DuckDuckGo => "DuckDuckGo",
        }
    }
}

fn contains_cjk(query: &str) -> bool {
    query.chars().any(|c| matches!(c, '\u{4E00}'..='\u{9FFF}' | '\u{3400}'..='\u{4DBF}' | '\u{F900}'..='\u{FAFF}'))
}

async fn search_with_searxng(
    client: &Client,
    query: &str,
    max_results: usize,
    endpoint: &str,
) -> anyhow::Result<Vec<SearchResultItem>> {
    let base = endpoint.trim().trim_end_matches('/');
    if base.is_empty() {
        return Err(anyhow::anyhow!("SearXNG endpoint 为空"));
    }

    let url = format!("{base}/search");
    let resp = client
        .get(url)
        .query(&[
            ("q", query),
            ("format", "json"),
            ("language", "zh-CN"),
            ("safesearch", "0"),
        ])
        .send()
        .await?
        .error_for_status()?
        .json::<SearxngResponse>()
        .await?;

    Ok(resp
        .results
        .into_iter()
        .filter(|item| !item.title.trim().is_empty() && !item.url.trim().is_empty())
        .take(max_results)
        .map(|item| SearchResultItem {
            title: item.title.trim().to_string(),
            url: item.url.trim().to_string(),
            snippet: item
                .content
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .chars()
                .take(120)
                .collect(),
            source: SearchProvider::Searxng.label().to_string(),
        })
        .collect())
}

async fn search_with_duckduckgo(
    client: &Client,
    query: &str,
    max_results: usize,
) -> anyhow::Result<Vec<SearchResultItem>> {
    let url = format!(
        "https://html.duckduckgo.com/html/?q={}",
        urlencoding::encode(query)
    );
    let html = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    let mut results = parse_duckduckgo_results(&html, max_results);

    if results.is_empty() {
        let lite_url = format!(
            "https://lite.duckduckgo.com/lite/?q={}",
            urlencoding::encode(query)
        );
        let lite_html = client
            .get(lite_url)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        results = parse_duckduckgo_results(&lite_html, max_results);
    }

    Ok(results)
}

async fn search_with_baidu(
    client: &Client,
    query: &str,
    max_results: usize,
) -> anyhow::Result<Vec<SearchResultItem>> {
    let url = format!("https://m.baidu.com/s?word={}", urlencoding::encode(query));
    let html = client
        .get(url)
        .header("Accept-Language", "zh-CN,zh;q=0.9")
        .header("Upgrade-Insecure-Requests", "1")
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    Ok(parse_baidu_results(&html, max_results))
}

async fn search_with_baidu_mcp(
    client: &Client,
    query: &str,
    max_results: usize,
    sse_endpoint: &str,
    api_key: &str,
) -> anyhow::Result<Vec<SearchResultItem>> {
    let api_key = api_key.trim();
    if api_key.is_empty() {
        return Err(anyhow::anyhow!("百度 MCP API Key 未配置"));
    }

    let sse_url = if sse_endpoint.contains("api_key=") {
        sse_endpoint.trim().to_string()
    } else {
        format!(
            "{}?api_key={}",
            sse_endpoint.trim_end_matches('/'),
            urlencoding::encode(api_key),
        )
    };
    let sse_resp = client
        .get(&sse_url)
        .header("Accept", "text/event-stream")
        .send()
        .await?
        .error_for_status()?;

    let mut stream = sse_resp.bytes_stream().eventsource();
    let message_endpoint = loop {
        let Some(event) = stream.next().await else {
            return Err(anyhow::anyhow!("百度 MCP 未返回 endpoint 事件"));
        };
        let event = event?;
        if event.event == "endpoint" && !event.data.trim().is_empty() {
            break if event.data.starts_with("http://") || event.data.starts_with("https://") {
                event.data
            } else {
                format!("http://appbuilder.baidu.com{}", event.data)
            };
        }
    };

    client
        .post(&message_endpoint)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "WaLiOffice", "version": "0.2.0" }
            }
        }))
        .send()
        .await?
        .error_for_status()?;

    let _ = client
        .post(&message_endpoint)
        .header("Content-Type", "application/json")
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        }))
        .send()
        .await;

    client
        .post(&message_endpoint)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "AIsearch",
                "arguments": {
                    "query": query,
                    "resource_type_filter": [{ "type": "web", "top_k": max_results.clamp(1, 10) }]
                }
            }
        }))
        .send()
        .await?
        .error_for_status()?;

    loop {
        let Some(event) = stream.next().await else {
            return Err(anyhow::anyhow!("百度 MCP 未返回 tools/call 结果"));
        };
        let event = event?;
        if event.event != "message" {
            continue;
        }
        let payload: serde_json::Value = match serde_json::from_str(&event.data) {
            Ok(value) => value,
            Err(_) => continue,
        };
        if payload.get("id").and_then(|value| value.as_i64()) != Some(2) {
            continue;
        }

        let text = payload
            .get("result")
            .and_then(|value| value.get("content"))
            .and_then(|value| value.as_array())
            .and_then(|items| {
                items
                    .iter()
                    .find(|item| item.get("type").and_then(|v| v.as_str()) == Some("text"))
            })
            .and_then(|item| item.get("text"))
            .and_then(|value| value.as_str())
            .unwrap_or("");

        return Ok(parse_baidu_mcp_results(text, max_results));
    }
}

fn parse_duckduckgo_results(html: &str, max_results: usize) -> Vec<SearchResultItem> {
    let result_re = Regex::new(
        r#"(?s)<a[^>]+(?:class="[^"]*(?:result__a|result-link)[^"]*"|class='[^']*(?:result__a|result-link)[^']*')[^>]+href="([^"]+)"[^>]*>(.*?)</a>"#
    ).expect("search result regex");
    let tag_re = Regex::new(r"(?s)<[^>]+>").expect("html tag regex");

    result_re
        .captures_iter(html)
        .filter_map(|caps| {
            let raw_url = caps.get(1)?.as_str();
            let raw_title = caps.get(2)?.as_str();
            let title = cleanup_html(raw_title, &tag_re);
            let url = normalize_result_url(raw_url);
            if title.is_empty() || url.is_empty() {
                return None;
            }
            Some(SearchResultItem {
                title,
                url,
                snippet: String::new(),
                source: SearchProvider::DuckDuckGo.label().to_string(),
            })
        })
        .take(max_results)
        .collect::<Vec<_>>()
        .into_iter()
        .enumerate()
        .map(|(idx, mut item)| {
            item.snippet = extract_nearby_snippet(html, &item.title, idx);
            item
        })
        .collect()
}

fn parse_baidu_results(html: &str, max_results: usize) -> Vec<SearchResultItem> {
    let title_re = Regex::new(r#"<!--s-text-->(.*?)<!--/s-text-->"#).expect("baidu title regex");
    let mu_re = Regex::new(r#""mu":"([^"]+)""#).expect("baidu mu regex");
    let title_param_re = Regex::new(r#"[?&]title=([^&"]+)"#).expect("baidu title param regex");
    let tag_re = Regex::new(r"(?s)<[^>]+>").expect("baidu tag regex");

    let blocks = split_baidu_result_blocks(html);
    let mut results = Vec::new();

    for block in blocks {
        let Some(mu_caps) = mu_re.captures(block) else {
            continue;
        };

        let url = cleanup_html(mu_caps.get(1).map(|m| m.as_str()).unwrap_or(""), &tag_re);
        if url.is_empty()
            || results
                .iter()
                .any(|item: &SearchResultItem| item.url == url)
        {
            continue;
        }

        let title = title_re
            .captures(block)
            .and_then(|caps| caps.get(1).map(|m| cleanup_html(m.as_str(), &tag_re)))
            .filter(|value| !value.is_empty())
            .or_else(|| {
                title_param_re
                    .captures(block)
                    .and_then(|caps| {
                        caps.get(1)
                            .map(|m| urlencoding::decode(m.as_str()).ok().map(|v| v.into_owned()))
                    })
                    .flatten()
                    .map(|value| cleanup_html(&value, &tag_re))
            })
            .unwrap_or_else(|| url.clone());

        if title.is_empty() {
            continue;
        }

        let snippet = extract_baidu_snippet(block, &title, &tag_re);
        results.push(SearchResultItem {
            title,
            url,
            snippet,
            source: SearchProvider::Baidu.label().to_string(),
        });
        if results.len() >= max_results {
            break;
        }
    }

    results
}

fn parse_baidu_mcp_results(text: &str, max_results: usize) -> Vec<SearchResultItem> {
    let block_re = Regex::new(r#"(?s)Title:\s*(.*?)\nContent:\s*(.*?)\nURL:\s*(https?://\S+)"#)
        .expect("baidu mcp result regex");

    block_re
        .captures_iter(text)
        .filter_map(|caps| {
            let title = caps.get(1)?.as_str().trim().to_string();
            let snippet = caps
                .get(2)?
                .as_str()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            let url = caps.get(3)?.as_str().trim().to_string();
            if title.is_empty() || url.is_empty() {
                return None;
            }
            Some(SearchResultItem {
                title,
                url,
                snippet: snippet.chars().take(160).collect(),
                source: SearchProvider::BaiduMcp.label().to_string(),
            })
        })
        .take(max_results)
        .collect()
}

fn split_baidu_result_blocks(html: &str) -> Vec<&str> {
    let marker = r#"<div  class="c-result result""#;
    let mut starts = html
        .match_indices(marker)
        .map(|(idx, _)| idx)
        .collect::<Vec<_>>();
    if starts.is_empty() {
        return Vec::new();
    }

    starts.push(html.len());
    starts
        .windows(2)
        .filter_map(|window| html.get(window[0]..window[1]))
        .collect()
}

fn extract_baidu_snippet(block: &str, title: &str, tag_re: &Regex) -> String {
    let cleaned = cleanup_html(block, tag_re)
        .replace(title, "")
        .replace("百度APP内打开", "")
        .replace("百度", "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    cleaned.trim().chars().take(140).collect::<String>()
}

fn cleanup_html(input: &str, tag_re: &Regex) -> String {
    tag_re
        .replace_all(input, "")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

fn normalize_result_url(raw_url: &str) -> String {
    if let Ok(parsed) = Url::parse(raw_url) {
        if let Some(target) = parsed
            .query_pairs()
            .find(|(key, _)| key == "uddg")
            .map(|(_, value)| value.to_string())
        {
            return target;
        }
        return parsed.to_string();
    }

    if raw_url.starts_with("//") {
        return format!("https:{raw_url}");
    }

    raw_url.to_string()
}

fn extract_nearby_snippet(html: &str, title: &str, fallback_index: usize) -> String {
    let title_pos = html
        .find(title)
        .unwrap_or(fallback_index.saturating_mul(120));
    let start = title_pos.saturating_sub(120);
    let end = (title_pos + 280).min(html.len());
    let fragment = &html[start..end];
    let cleaned = Regex::new(r"(?s)<[^>]+>")
        .expect("fragment regex")
        .replace_all(fragment, " ")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ");
    let collapsed = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    collapsed
        .replace(title, "")
        .trim()
        .chars()
        .take(120)
        .collect::<String>()
}
