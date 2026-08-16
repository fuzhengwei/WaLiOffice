/**
 * WaLiOffice Rust 后端 - Agent 代理路由
 *
 * 将 Rust axum 的 chat_stream 请求代理到 DSH Agent Engine。
 * 当 DSH 引擎可用时，使用 DSH 处理 agent 请求；
 * 当 DSH 不可用时，回退到 Rust 自研的 agent_loop。
 *
 * 新增路由：
 *   POST /api/agent/dsh/stream  — DSH Agent SSE 流式对话
 *   POST /api/agent/dsh/status  — DSH Agent Engine 状态检查
 *
 * Rust 端实现（本文件是 TypeScript 参考设计，实际需在 Rust 中实现）
 */

// ─── Rust 端实现参考 ─────────────────────────────────────────────────
//
// 在 server/src/routes/ 下新增 dsh_proxy.rs：
//
// ```rust
// use axum::{
//     extract::State,
//     response::sse::{Event, Sse},
//     Json,
// };
// use reqwest::Client;
// use serde::{Deserialize, Serialize};
// use std::convert::Infallible;
// use tokio_stream::Stream;
//
// #[derive(Deserialize)]
// pub struct DshChatRequest {
//     pub message: String,
//     pub session_id: Option<String>,
//     pub user_id: Option<String>,
//     pub model: Option<String>,
//     pub tool_kind: Option<String>,
//     pub attachments: Option<Vec<serde_json::Value>>,
//     pub tool_config: Option<serde_json::Value>,
// }
//
// #[derive(Serialize)]
// pub struct DshStatusResponse {
//     pub dsh_available: bool,
//     pub engine: String,
//     pub version: String,
// }
//
// const DSH_ENGINE_URL: &str = "http://127.0.0.1:3780";
//
// /// 检查 DSH Agent Engine 是否可用
// pub async fn check_dsh_available() -> bool {
//     let client = Client::new();
//     client
//         .get(format!("{}/agent/status", DSH_ENGINE_URL))
//         .timeout(std::time::Duration::from_secs(2))
//         .send()
//         .await
//         .is_ok()
// }
//
// /// 代理 SSE 流式对话到 DSH
// pub async fn dsh_chat_stream(
//     State(state): State<AppState>,
//     Json(req): Json<DshChatRequest>,
// ) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
//     let client = Client::new();
//     let dsh_url = format!("{}/agent/stream", DSH_ENGINE_URL);
//
//     let resp = client
//         .post(&dsh_url)
//         .json(&req)
//         .send()
//         .await;
//
//     // 将 DSH 的 SSE 流转换为 axum SSE 流
//     // ...（具体实现见 Rust 端代码）
// }
//
// /// DSH Agent Engine 状态
// pub async fn dsh_status() -> Json<DshStatusResponse> {
//     let available = check_dsh_available().await;
//     Json(DshStatusResponse {
//         dsh_available: available,
//         engine: "dsh".to_string(),
//         version: "0.1.0-rc.6".to_string(),
//     })
// }
// ```
//
// ─── 路由注册 ─────────────────────────────────────────────────────────
//
// 在 server/src/routes/mod.rs 中添加：
//
// ```rust
// pub mod dsh_proxy;
// ```
//
// 在 build_router() 中添加：
//
// ```rust
// let dsh_routes = Router::new()
//     .route("/dsh/stream", post(dsh_proxy::dsh_chat_stream))
//     .route("/dsh/status", get(dsh_proxy::dsh_status));
//
// let api_routes = Router::new()
//     .nest("/agent", dsh_routes)
//     // ... 其他路由
//     ;
// ```
//
// ─── 前端切换 ─────────────────────────────────────────────────────────
//
// 前端 chatApi 新增 dshStream 方法：
//
// ```typescript
// // src/api/index.ts
// dshStream: async (params: ChatStreamParams, signal?: AbortSignal) => {
//   const response = await fetch('/api/agent/dsh/stream', {
//     method: 'POST',
//     headers: { 'Content-Type': 'application/json' },
//     body: JSON.stringify(params),
//     signal,
//   });
//   // ... SSE 解析逻辑与 chatApi.stream 相同
// }
// ```
//
// 在 Studio.tsx 中根据 /api/agent/dsh/status 判断是否使用 DSH：
//
// ```typescript
// const [useDsh, setUseDsh] = useState(false);
//
// useEffect(() => {
//   fetch('/api/agent/dsh/status')
//     .then(r => r.json())
//     .then(data => setUseDsh(data.dsh_available))
//     .catch(() => setUseDsh(false));
// }, []);
//
// // 发送消息时选择后端
// const streamFn = useDsh ? chatApi.dshStream : chatApi.stream;
// ```

export {}
