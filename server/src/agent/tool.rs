use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::models::{Artifact, ChatAttachment};

/// 工具执行上下文
#[derive(Clone)]
pub struct ToolContext {
    pub session_id: String,
    pub user_id: String,
    pub project_id: Option<String>,
    pub preferred_model: Option<String>,
    pub attachments: Vec<ChatAttachment>,
    /// SSE 推送回调（向前端发送实时进度）
    pub emit: Arc<dyn Fn(&str, serde_json::Value) + Send + Sync>,
    /// 共享上下文（跨工具传递，如 PPT 大纲规划）
    pub scratchpad: Arc<Mutex<HashMap<String, serde_json::Value>>>,
    /// 用户工具配置（前端传入，如视频时长、宽高比等）
    pub tool_config: Option<serde_json::Value>,
}

impl ToolContext {
    pub fn new(
        session_id: String,
        user_id: String,
        project_id: Option<String>,
        preferred_model: Option<String>,
        attachments: Vec<ChatAttachment>,
        emit: impl Fn(&str, serde_json::Value) + Send + Sync + 'static,
    ) -> Self {
        Self {
            session_id,
            user_id,
            project_id,
            preferred_model,
            attachments,
            emit: Arc::new(emit),
            scratchpad: Arc::new(Mutex::new(HashMap::new())),
            tool_config: None,
        }
    }

    pub fn with_tool_config(mut self, config: serde_json::Value) -> Self {
        self.tool_config = Some(config);
        self
    }

    /// 获取工具配置中的某个字段值
    pub fn get_config<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.tool_config
            .as_ref()
            .and_then(|cfg| cfg.get(key))
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    pub fn send(&self, event: &str, data: serde_json::Value) {
        (self.emit)(event, data);
    }
}

/// 工具产生的产物
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolArtifact {
    pub kind: String, // document | ppt | drawio | sheet | image | code | mixed
    pub title: String,
    pub content: serde_json::Value,
}

/// 工具结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifacts: Option<Vec<ToolArtifact>>,
    /// 给 LLM 的观察文本（ReAct Observation）
    pub observation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub continue_loop: Option<bool>,
}

impl ToolResult {
    pub fn ok(observation: impl Into<String>, artifacts: Vec<ToolArtifact>) -> Self {
        Self {
            success: true,
            data: None,
            error: None,
            artifacts: Some(artifacts),
            observation: observation.into(),
            continue_loop: None,
        }
    }

    pub fn err(observation: impl Into<String>) -> Self {
        let obs = observation.into();
        Self {
            success: false,
            data: None,
            error: Some(obs.clone()),
            artifacts: None,
            observation: obs,
            continue_loop: None,
        }
    }
}

/// 工具定义 trait
#[async_trait]
pub trait OfficeTool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    /// JSON Schema 参数定义
    fn parameters(&self) -> serde_json::Value;
    fn is_read_only(&self) -> bool {
        false
    }
    fn produces_artifact(&self) -> bool {
        true
    }

    async fn call(&self, input: serde_json::Value, ctx: &ToolContext) -> ToolResult;
}

/// 便利类型别名
pub type DynTool = Arc<dyn OfficeTool>;

impl ToolResult {
    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }
}
