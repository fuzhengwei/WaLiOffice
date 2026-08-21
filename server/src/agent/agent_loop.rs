// Agent 循环核心（2-1 节 stub，完整实现在 2-5 节）
//
// 2-1 节目标：项目可编译、可启动，聊天功能返回"功能开发中"。
// 完整 ReAct 循环将在第 2-5 节实现。

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use super::context::{compact_context, DEFAULT_CONTEXT_CONFIG};
use super::registry::REGISTRY;
use super::tool::{ToolContext, ToolResult};
use crate::llm::FunctionDef;
use crate::models::{Artifact, ChatMessage};

// ─── AgentEvent ──────────────────────────────────────────────────────────────

/// Agent 事件（通过 channel 向上层推送）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AgentEvent {
    Thinking { content: String },
    ToolCall { tool: String, input: serde_json::Value },
    ToolResult { tool: String, success: bool, result: serde_json::Value, error: Option<String> },
    Artifact { artifact: Artifact },
    Message { content: String },
    TurnEnd { turn: usize },
    Done { summary: String, artifacts: Vec<Artifact> },
    Error { message: String },
}

// ─── AgentConfig ─────────────────────────────────────────────────────────────

/// Agent 配置
#[derive(Clone)]
pub struct AgentConfig {
    pub max_turns: usize,
    pub system_prompt: String,
    pub allowed_tools: Option<Vec<String>>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_turns: 8,
            system_prompt: String::new(),
            allowed_tools: None,
        }
    }
}

// ─── run_agent_loop (2-1 stub) ───────────────────────────────────────────────

/// 运行 Agent 循环（2-1 节 stub 版本）
///
/// 完整实现见第 2-5 节「ReAct 循环」：
/// - 从 REGISTRY 获取工具定义，组装 system prompt
/// - 调用 compact_context 做上下文压缩
/// - 进入 ReAct 循环：LLM 推理 → 工具调用 → 结果回写 → 重复
/// - 通过 channel 推送 AgentEvent（Thinking/ToolCall/Message/Done 等）
pub async fn run_agent_loop(
    _history: Vec<ChatMessage>,
    _user_message: String,
    _user_attachments: Vec<crate::models::ChatAttachment>,
    _ctx: ToolContext,
    _config: AgentConfig,
    _client: std::sync::Arc<crate::llm::LlmClient>,
) -> mpsc::Receiver<AgentEvent> {
    let (tx, rx) = mpsc::channel(8);

    tokio::spawn(async move {
        // 2-1 节：暂不接入 LLM，直接告知用户功能开发中
        let _ = tx
            .send(AgentEvent::Error {
                message: "Agent 功能开发中（第 2-5 节接入 LLM）".to_string(),
            })
            .await;
        let _ = tx
            .send(AgentEvent::Done {
                summary: "Agent 功能开发中，请期待第 2-5 节！".to_string(),
                artifacts: vec![],
            })
            .await;
    });

    rx
}
