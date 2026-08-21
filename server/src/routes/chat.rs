// 聊天路由（2-1 节 stub，完整实现在 2-7 节）
//
// 2-1 节目标：路由可注册，POST /api/chat/stream 返回 SSE "功能开发中"。
// 完整流式对话将在第 2-7 节实现。

use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::post;
use axum::{extract::State, Json, Router};
use futures::stream::Stream;
use std::time::Duration;
use tokio_stream::wrappers::ReceiverStream;
use tokio::sync::mpsc;

use crate::auth::middleware::AuthUser;
use crate::error::AppError;
use crate::models::ChatRequest;

// ─── 路由注册 ────────────────────────────────────────────────────────────────

pub fn router() -> Router {
    Router::new().route("/api/chat/stream", post(chat_stream))
}

// ─── /api/chat/stream 处理函数（2-1 stub） ───────────────────────────────────

/// SSE 流式聊天端点
///
/// 完整实现在 2-7 节「后端 Chat 路由与 SSE 端点」：
/// 1. AuthUser 提取器鉴权
/// 2. 获取或创建会话（session_repo）
/// 3. IntentAnalyzer 分析用户意图（2-6 节）
/// 4. run_agent_loop 启动 ReAct 循环（2-5 节）
/// 5. 将 AgentEvent 转换为 SSE Event 并推送
/// 6. 写入消息到数据库（session_repo）
async fn chat_stream(
    _auth: AuthUser,
    _body: Json<ChatRequest>,
) -> Sse<ReceiverStream<Result<Event, std::convert::Infallible>>> {
    // 2-1 节：暂不接入 LLM，通过 channel 返回开发中提示
    let (tx, rx) = mpsc::channel::<Result<Event, std::convert::Infallible>>(8);

    // 发送"功能开发中"消息
    let _ = tx.send(Ok(Event::default()
        .event("message")
        .data(r#"{"type":"message","content":"🤖 Agent 功能开发中，预计第 2-5 节接入 ReAct 循环后开启聊天。"}"#)));

    let _ = tx.send(Ok(Event::default()
        .event("done")
        .data(r#"{"type":"done","summary":"Agent 功能开发中"}"#)));

    // 发送 end 事件后关闭 channel
    drop(tx);

    let stream = ReceiverStream::new(rx);
    Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(30)))
}
