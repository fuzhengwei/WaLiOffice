use async_trait::async_trait;
use serde_json::json;

use crate::agent::tool::{OfficeTool, ToolArtifact, ToolContext, ToolResult};

/// 获取 tenant_access_token（应用身份，用于飞书文档/表格/日历 API）
async fn get_tenant_access_token() -> anyhow::Result<String> {
    let cfg = crate::config::config();
    if cfg.feishu_app_id.is_empty() || cfg.feishu_app_secret.is_empty() {
        return Err(anyhow::anyhow!("未配置飞书应用（FEISHU_APP_ID/FEISHU_APP_SECRET）"));
    }
    let client = reqwest::Client::new();
    let resp: serde_json::Value = client
        .post("https://open.feishu.cn/open-apis/auth/v3/tenant_access_token/internal")
        .json(&json!({
            "app_id": cfg.feishu_app_id,
            "app_secret": cfg.feishu_app_secret,
        }))
        .send().await?.error_for_status()?.json().await?;
    let code = resp.get("code").and_then(|v| v.as_i64()).unwrap_or(-1);
    if code != 0 {
        return Err(anyhow::anyhow!(
            "获取飞书 token 失败: {}",
            resp.get("msg").and_then(|v| v.as_str()).unwrap_or("unknown")
        ));
    }
    resp.get("tenant_access_token")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("tenant_access_token 缺失"))
}

// ============ 飞书文档读取工具 ============

pub struct FeishuDocReadTool;

#[async_trait]
impl OfficeTool for FeishuDocReadTool {
    fn name(&self) -> &str {
        "feishu_doc_read"
    }

    fn description(&self) -> &str {
        "读取飞书云文档（docx）的纯文本内容。输入文档的 document_id（从文档 URL 中 /docx/ 后的一段）。"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "document_id": { "type": "string", "description": "飞书文档 ID，形如 MDE0xxx（从 https://xxx.feishu.cn/docx/XXX 中提取 XXX）" }
            },
            "required": ["document_id"]
        })
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn produces_artifact(&self) -> bool {
        true
    }

    async fn call(&self, input: serde_json::Value, _ctx: &ToolContext) -> ToolResult {
        let doc_id = input.get("document_id").and_then(|v| v.as_str()).unwrap_or("").trim();
        if doc_id.is_empty() {
            return ToolResult::err("document_id 不能为空");
        }
        match do_read_doc(doc_id).await {
            Ok(text) => {
                let artifact = ToolArtifact {
                    kind: "markdown".into(),
                    title: format!("飞书文档 · {doc_id}"),
                    content: json!({ "type": "markdown", "markdown": text, "source": "feishu_doc" }),
                };
                ToolResult {
                    success: true,
                    data: Some(json!({ "document_id": doc_id, "content": text })),
                    error: None,
                    artifacts: Some(vec![artifact]),
                    observation: format!("飞书文档内容如下：\n{text}"),
                    continue_loop: None,
                }
            }
            Err(e) => ToolResult::err(format!("读取飞书文档失败: {e}")),
        }
    }
}

async fn do_read_doc(document_id: &str) -> anyhow::Result<String> {
    let token = get_tenant_access_token().await?;
    let client = reqwest::Client::new();
    let resp: serde_json::Value = client
        .get(format!(
            "https://open.feishu.cn/open-apis/docx/v1/documents/{document_id}/raw_content"
        ))
        .header("Authorization", format!("Bearer {token}"))
        .send().await?.error_for_status()?.json().await?;
    let code = resp.get("code").and_then(|v| v.as_i64()).unwrap_or(-1);
    if code != 0 {
        return Err(anyhow::anyhow!(
            "飞书文档读取失败: {}",
            resp.get("msg").and_then(|v| v.as_str()).unwrap_or("unknown")
        ));
    }
    resp.get("data")
        .and_then(|d| d.get("content"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("文档内容为空"))
}

// ============ 飞书多维表格查询工具 ============

pub struct FeishuBitableQueryTool;

#[async_trait]
impl OfficeTool for FeishuBitableQueryTool {
    fn name(&self) -> &str {
        "feishu_bitable_query"
    }

    fn description(&self) -> &str {
        "查询飞书多维表格（Bitable）的记录。输入 app_token 和 table_id（从多维表格 URL 中提取：/base/APP_TOKEN?table=TABLE_ID）。"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "app_token": { "type": "string", "description": "多维表格 app_token（URL /base/ 后的一段）" },
                "table_id": { "type": "string", "description": "数据表 table_id（URL ?table= 后的值）" },
                "page_size": { "type": "integer", "description": "返回记录数，默认 20", "minimum": 1, "maximum": 100 }
            },
            "required": ["app_token", "table_id"]
        })
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn produces_artifact(&self) -> bool {
        true
    }

    async fn call(&self, input: serde_json::Value, _ctx: &ToolContext) -> ToolResult {
        let app_token = input.get("app_token").and_then(|v| v.as_str()).unwrap_or("").trim();
        let table_id = input.get("table_id").and_then(|v| v.as_str()).unwrap_or("").trim();
        let page_size = input.get("page_size").and_then(|v| v.as_u64()).unwrap_or(20).clamp(1, 100) as usize;
        if app_token.is_empty() || table_id.is_empty() {
            return ToolResult::err("app_token 和 table_id 不能为空");
        }
        match do_query_bitable(app_token, table_id, page_size).await {
            Ok(records) => {
                let json_str = serde_json::to_string_pretty(&records).unwrap_or_default();
                let artifact = ToolArtifact {
                    kind: "markdown".into(),
                    title: format!("飞书表格记录 · {table_id}"),
                    content: json!({ "type": "markdown", "markdown": json_str, "source": "feishu_bitable" }),
                };
                ToolResult {
                    success: true,
                    data: Some(json!({ "records": records })),
                    error: None,
                    artifacts: Some(vec![artifact]),
                    observation: format!("飞书多维表格记录如下：\n{json_str}"),
                    continue_loop: None,
                }
            }
            Err(e) => ToolResult::err(format!("查询飞书表格失败: {e}")),
        }
    }
}

async fn do_query_bitable(app_token: &str, table_id: &str, page_size: usize) -> anyhow::Result<Vec<serde_json::Value>> {
    let token = get_tenant_access_token().await?;
    let client = reqwest::Client::new();
    let resp: serde_json::Value = client
        .get(format!(
            "https://open.feishu.cn/open-apis/bitable/v1/apps/{app_token}/tables/{table_id}/records"
        ))
        .query(&[("page_size", &page_size.to_string())])
        .header("Authorization", format!("Bearer {token}"))
        .send().await?.error_for_status()?.json().await?;
    let code = resp.get("code").and_then(|v| v.as_i64()).unwrap_or(-1);
    if code != 0 {
        return Err(anyhow::anyhow!(
            "飞书表格查询失败: {}",
            resp.get("msg").and_then(|v| v.as_str()).unwrap_or("unknown")
        ));
    }
    let records = resp
        .get("data").and_then(|d| d.get("items"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    // 简化：只取每条记录的 fields
    let simplified: Vec<serde_json::Value> = records
        .into_iter()
        .map(|r| r.get("fields").cloned().unwrap_or(serde_json::Value::Null))
        .collect();
    Ok(simplified)
}

// ============ 飞书日历事件列表工具 ============

pub struct FeishuCalendarListTool;

#[async_trait]
impl OfficeTool for FeishuCalendarListTool {
    fn name(&self) -> &str {
        "feishu_calendar_list"
    }

    fn description(&self) -> &str {
        "查询飞书日历的日程（事件）列表。可选传 start_time/end_time（Unix 秒时间戳）限定时间范围。"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "calendar_id": { "type": "string", "description": "日历 ID，可选，不传则查主日历" },
                "start_time": { "type": "string", "description": "开始时间 Unix 时间戳（秒），可选" },
                "end_time": { "type": "string", "description": "结束时间 Unix 时间戳（秒），可选" }
            },
            "required": []
        })
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn produces_artifact(&self) -> bool {
        true
    }

    async fn call(&self, input: serde_json::Value, _ctx: &ToolContext) -> ToolResult {
        let calendar_id = input.get("calendar_id").and_then(|v| v.as_str()).unwrap_or("primary");
        let start_time = input.get("start_time").and_then(|v| v.as_str()).unwrap_or("");
        let end_time = input.get("end_time").and_then(|v| v.as_str()).unwrap_or("");
        match do_list_events(calendar_id, start_time, end_time).await {
            Ok(events) => {
                let json_str = serde_json::to_string_pretty(&events).unwrap_or_default();
                let artifact = ToolArtifact {
                    kind: "markdown".into(),
                    title: "飞书日历日程".into(),
                    content: json!({ "type": "markdown", "markdown": json_str, "source": "feishu_calendar" }),
                };
                ToolResult {
                    success: true,
                    data: Some(json!({ "events": events })),
                    error: None,
                    artifacts: Some(vec![artifact]),
                    observation: format!("飞书日历日程如下：\n{json_str}"),
                    continue_loop: None,
                }
            }
            Err(e) => ToolResult::err(format!("查询飞书日历失败: {e}")),
        }
    }
}

async fn do_list_events(calendar_id: &str, start_time: &str, end_time: &str) -> anyhow::Result<Vec<serde_json::Value>> {
    let token = get_tenant_access_token().await?;
    let client = reqwest::Client::new();
    let mut url = format!(
        "https://open.feishu.cn/open-apis/calendar/v4/calendars/{}/events",
        urlencoding::encode(calendar_id)
    );
    let mut first = true;
    if !start_time.is_empty() {
        url.push_str(&format!("{}start_time={}", if first { "?" } else { "&" }, start_time));
        first = false;
    }
    if !end_time.is_empty() {
        url.push_str(&format!("{}end_time={}", if first { "?" } else { "&" }, end_time));
    }
    let resp: serde_json::Value = client
        .get(&url)
        .header("Authorization", format!("Bearer {token}"))
        .send().await?.error_for_status()?.json().await?;
    let code = resp.get("code").and_then(|v| v.as_i64()).unwrap_or(-1);
    if code != 0 {
        return Err(anyhow::anyhow!(
            "飞书日历查询失败: {}",
            resp.get("msg").and_then(|v| v.as_str()).unwrap_or("unknown")
        ));
    }
    let events = resp
        .get("data").and_then(|d| d.get("items"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    // 简化：只取 summary + start_time + end_time + organizer
    let simplified: Vec<serde_json::Value> = events
        .into_iter()
        .map(|e| {
            json!({
                "summary": e.get("summary").cloned().unwrap_or(serde_json::Value::Null),
                "start_time": e.get("start_time").cloned().unwrap_or(serde_json::Value::Null),
                "end_time": e.get("end_time").cloned().unwrap_or(serde_json::Value::Null),
            })
        })
        .collect();
    Ok(simplified)
}
