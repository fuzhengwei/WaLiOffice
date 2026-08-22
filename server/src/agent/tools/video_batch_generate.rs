use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use std::time::{Duration, Instant};
use tokio::time::sleep;

use crate::agent::tool::{OfficeTool, ToolArtifact, ToolContext, ToolResult};

use super::video_generate::{normalize_aspect_ratio, collect_video_audios, collect_video_refs};
use crate::agent::tools::agnes_media::{
    agnes_video_model, get_json, http_client, post_json, resolve_video_credentials,
};

pub struct VideoBatchGenerateTool;

/// 单镜头定义（从 storyboard artifact 中解析）
#[derive(Debug, Clone, serde::Deserialize)]
struct StoryboardShot {
    index: u32,
    title: String,
    description: String,
    prompt: String,
    mode: String,
    seconds: u8,
    #[serde(default)]
    first_frame: Option<String>,
    #[serde(default)]
    last_frame: Option<String>,
    #[serde(default)]
    reference_images: Vec<String>,
    #[serde(default)]
    audio_urls: Vec<String>,
    #[serde(default)]
    transition: Option<String>,
}

/// 分镜方案 artifact 内容
#[derive(Debug, serde::Deserialize)]
struct StoryboardContent {
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    aspect_ratio: String,
    shots: Vec<StoryboardShot>,
}

/// Agnes V2.5 创建任务响应
#[derive(Debug, Deserialize)]
struct CreateVideoResponse {
    id: String,
    #[serde(default)]
    progress: Option<i32>,
}

/// Agnes V2.5 查询任务响应
#[derive(Debug, Deserialize)]
struct QueryVideoResponse {
    status: String,
    #[serde(default)]
    progress: Option<i32>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    seconds: Option<String>,
    #[serde(default)]
    size: Option<String>,
    #[serde(default)]
    id: String,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    error: Option<serde_json::Value>,
}

#[async_trait]
impl OfficeTool for VideoBatchGenerateTool {
    fn name(&self) -> &str {
        "video_batch_generate"
    }

    fn description(&self) -> &str {
        "批量视频生成：读取分镜方案 artifact，逐镜头调用 Agnes V2.5 生成视频。每个镜头独立生成，完成后返回多个视频 artifact。支持链式一致性——上一镜头视频 URL 自动作为下一镜头的参考素材。"
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn produces_artifact(&self) -> bool {
        true
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "storyboard_artifact_id": {
                    "type": "string",
                    "description": "分镜方案产物的 ID（video_storyboard 工具生成的 artifact 的 id）"
                },
                "chain_consistency": {
                    "type": "boolean",
                    "description": "是否启用链式一致性（上一镜头视频 URL → 下一镜头参考素材），默认 true"
                }
            },
            "required": ["storyboard_artifact_id"]
        })
    }

    async fn call(&self, input: serde_json::Value, ctx: &ToolContext) -> ToolResult {
        let storyboard_id = input
            .get("storyboard_artifact_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        if storyboard_id.is_empty() {
            return ToolResult::err("storyboard_artifact_id 不能为空");
        }

        let chain_consistency = input
            .get("chain_consistency")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        // ---- 从 prior_artifacts 中查找分镜 artifact ----
        let storyboard_artifact = ctx
            .prior_artifacts
            .iter()
            .find(|a| a.id == storyboard_id)
            .cloned();
        let storyboard_artifact = match storyboard_artifact {
            Some(a) => a,
            None => {
                return ToolResult::err(format!(
                    "找不到 ID 为 {storyboard_id} 的分镜产物。请先使用分镜规划工具生成方案。"
                ));
            }
        };

        // 检查是否是分镜类型
        let sb_type = storyboard_artifact
            .content
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if sb_type != "video_storyboard" {
            return ToolResult::err(format!(
                "产物 {storyboard_id} 不是分镜方案（type={sb_type}），请传入 video_storyboard 工具生成的产物 ID"
            ));
        }

        // ---- 解析分镜内容 ----
        let storyboard: StoryboardContent = match serde_json::from_value(storyboard_artifact.content.clone()) {
            Ok(s) => s,
            Err(err) => {
                return ToolResult::err(format!("分镜方案解析失败: {err}"));
            }
        };

        let total_shots = storyboard.shots.len();
        if total_shots == 0 {
            return ToolResult::err("分镜方案中没有镜头");
        }

        ctx.send(
            "state_update",
            json!({
                "phase": "running",
                "step": "批量生成启动",
                "detail": format!("开始批量生成《{}》的 {} 个镜头...", storyboard.title, total_shots),
                "total_shots": total_shots,
                "at": chrono::Utc::now().to_rfc3339(),
            }),
        );

        // ---- Agnes 凭证 ----
        let credentials = match resolve_video_credentials(&ctx.user_id) {
            Ok(c) => c,
            Err(err) => {
                return ToolResult::err(format!("Agnes 凭证不可用：{err}"));
            }
        };

        let video_model = agnes_video_model();
        let client = match http_client(Duration::from_secs(90)) {
            Ok(c) => c,
            Err(err) => {
                return ToolResult::err(format!("初始化 HTTP 客户端失败：{err}"));
            }
        };

        // ---- 逐镜头生成 ----
        let mut video_artifacts: Vec<ToolArtifact> = Vec::new();
        let mut video_urls: Vec<String> = Vec::new();
        let mut prev_video_url: Option<String> = None;

        for (i, shot) in storyboard.shots.iter().enumerate() {
            let shot_num = i + 1;
            ctx.send(
                "state_update",
                json!({
                    "phase": "running",
                    "step": format!("生成镜头 {shot_num}/{total_shots}"),
                    "detail": format!("《{}》— {}（{}模式，{}s）", shot.title, shot.description, shot.mode, shot.seconds),
                    "current_shot": shot_num,
                    "total_shots": total_shots,
                    "at": chrono::Utc::now().to_rfc3339(),
                }),
            );

            // ---- 构建请求体 ----
            let aspect_ratio = normalize_aspect_ratio(&storyboard.aspect_ratio);
            let (size_label, _width, _height) = infer_size_label(aspect_ratio);

            // Agnes V2.5: keyframe 模式传 mode="keyframes"，其他模式省略 mode
            let mut request_body = json!({
                "model": video_model.as_str(),
                "prompt": shot.prompt.clone(),
                "seconds": shot.seconds.to_string(),
                "size": size_label,
                "aspect_ratio": aspect_ratio,
                "negative_prompt": "low quality, blurry, distorted, flicker, watermark, text artifacts",
            });

            if shot.mode == "keyframe" {
                request_body["mode"] = json!("keyframes");
            }

            // 模式专用参数
            match shot.mode.as_str() {
                "keyframe" => {
                    if let Some(ff) = &shot.first_frame {
                        if !ff.is_empty() {
                            request_body["first_frame"] = json!(ff);
                        }
                    }
                    if let Some(lf) = &shot.last_frame {
                        if !lf.is_empty() {
                            request_body["last_frame"] = json!(lf);
                        }
                    }
                    // 音频参考
                    if !shot.audio_urls.is_empty() {
                        request_body["audios"] = json!(shot.audio_urls);
                    }
                }
                "reference" => {
                    let mut refs = shot.reference_images.clone();
                    // 链式一致性：将上一镜头视频 URL 加入参考
                    if chain_consistency {
                        if let Some(prev_url) = &prev_video_url {
                            // 使用 videos[] 参考上一镜头
                            request_body["videos"] = json!([{
                                "url": prev_url,
                                "start_seconds": 0,
                                "require_audio": false
                            }]);
                            // 在 prompt 中添加 <Video 1> 引用
                            if !shot.prompt.contains("<Video") {
                                let video_ref = " Based on <Video 1> for visual continuity, maintain consistent character and scene style.";
                                request_body["prompt"] = json!(format!("{}{}", shot.prompt, video_ref));
                            }
                        }
                        if !refs.is_empty() {
                            request_body["images"] = json!(refs);
                        }
                        if !shot.audio_urls.is_empty() {
                            request_body["audios"] = json!(shot.audio_urls);
                        }
                    } else {
                        if !refs.is_empty() {
                            request_body["images"] = json!(refs);
                        }
                        if !shot.audio_urls.is_empty() {
                            request_body["audios"] = json!(shot.audio_urls);
                        }
                    }
                }
                _ => {
                    // text 模式也可以使用音频参考
                    if !shot.audio_urls.is_empty() {
                        request_body["audios"] = json!(shot.audio_urls);
                    }
                    // 链式一致性：text 模式也可以用 videos[] 参考上一镜头
                    if chain_consistency {
                        if let Some(prev_url) = &prev_video_url {
                            request_body["videos"] = json!([{
                                "url": prev_url,
                                "start_seconds": 0,
                                "require_audio": false
                            }]);
                            if !shot.prompt.contains("<Video") {
                                let video_ref = " Based on <Video 1> for visual continuity, maintain consistent character and scene style.";
                                request_body["prompt"] = json!(format!("{}{}", shot.prompt, video_ref));
                            }
                        }
                    }
                }
            }

            // ---- 提交任务 ----
            let create_url = credentials.endpoint("/v1/videos");
            let create_resp: CreateVideoResponse = match post_json(&client, &create_url, &credentials, &request_body).await {
                Ok(r) => r,
                Err(err) => {
                    // 单镜头失败不影响其他镜头
                    ctx.send("state_update", json!({
                        "phase": "running",
                        "step": format!("镜头 {shot_num} 提交失败"),
                        "detail": format!("镜头 {shot_num}《{}》提交失败：{err}，跳过", shot.title),
                        "at": chrono::Utc::now().to_rfc3339(),
                    }));
                    video_artifacts.push(ToolArtifact {
                        kind: "video".into(),
                        title: format!("镜头{}：{}（失败）", shot.index, shot.title),
                        content: json!({
                            "type": "generated_video",
                            "title": format!("镜头{}：{}", shot.index, shot.title),
                            "description": shot.description,
                            "prompt": shot.prompt,
                            "status": "failed",
                            "error": format!("提交失败：{err}"),
                            "shot_index": shot.index,
                        }),
                    });
                    continue;
                }
            };

            let task_id = create_resp.id.clone();
            let poll_url = credentials.endpoint(&format!("/v1/videos/{}", urlencoding::encode(&task_id)));
            let deadline = Instant::now() + Duration::from_secs(480);
            let mut latest_progress = create_resp.progress.unwrap_or(0);

            // ---- 轮询 ----
            let mut shot_video_url: Option<String> = None;
            let mut shot_failed = false;

            loop {
                if Instant::now() >= deadline {
                    shot_failed = true;
                    break;
                }

                let status_resp: QueryVideoResponse = match get_json(&client, &poll_url, &credentials).await {
                    Ok(r) => r,
                    Err(_) => {
                        sleep(Duration::from_secs(5)).await;
                        continue;
                    }
                };

                latest_progress = status_resp.progress.unwrap_or(latest_progress);
                ctx.send("state_update", json!({
                    "phase": "running",
                    "step": format!("镜头 {shot_num}/{total_shots} 生成中"),
                    "detail": format!("《{}》状态：{}（{}%）", shot.title, status_resp.status, latest_progress),
                    "current_shot": shot_num,
                    "total_shots": total_shots,
                    "progress": latest_progress,
                    "at": chrono::Utc::now().to_rfc3339(),
                }));

                match status_resp.status.as_str() {
                    "completed" => {
                        shot_video_url = status_resp.url.filter(|u| !u.trim().is_empty());
                        break;
                    }
                    "failed" | "error" | "cancelled" => {
                        shot_failed = true;
                        break;
                    }
                    _ => sleep(Duration::from_secs(5)).await,
                }
            }

            // ---- 记录结果 ----
            if let Some(url) = shot_video_url {
                prev_video_url = Some(url.clone());
                video_urls.push(url.clone());
                video_artifacts.push(ToolArtifact {
                    kind: "video".into(),
                    title: format!("镜头{}：{}", shot.index, shot.title),
                    content: json!({
                        "type": "generated_video",
                        "title": format!("镜头{}：{}", shot.index, shot.title),
                        "description": shot.description,
                        "prompt": shot.prompt,
                        "video_url": url,
                        "task_id": task_id,
                        "status": "completed",
                        "seconds": shot.seconds,
                        "aspect_ratio": aspect_ratio,
                        "mode": shot.mode,
                        "shot_index": shot.index,
                        "transition": shot.transition,
                        "provider": "agnes",
                        "api_version": "v2.5",
                    }),
                });
            } else if shot_failed {
                video_artifacts.push(ToolArtifact {
                    kind: "video".into(),
                    title: format!("镜头{}：{}（失败）", shot.index, shot.title),
                    content: json!({
                        "type": "generated_video",
                        "title": format!("镜头{}：{}", shot.index, shot.title),
                        "description": shot.description,
                        "prompt": shot.prompt,
                        "status": "failed",
                        "error": "生成超时或失败",
                        "shot_index": shot.index,
                    }),
                });
            }
        }

        // ---- 汇总 artifact ----
        let success_count = video_urls.len();
        let total_seconds: u32 = storyboard.shots.iter().map(|s| s.seconds as u32).sum();

        let summary_content = json!({
            "type": "video_batch_summary",
            "title": format!("批量生成完成：{}", storyboard.title),
            "storyboard_title": storyboard.title,
            "total_shots": total_shots,
            "success_count": success_count,
            "failed_count": total_shots - success_count,
            "total_seconds": total_seconds,
            "aspect_ratio": storyboard.aspect_ratio,
            "videos": video_urls,
            "chain_consistency": chain_consistency,
            "usage_guide": "所有镜头已生成完毕。可逐个下载视频片段，使用剪映/PR等工具进行拼接和后期剪辑。",
        });

        let mut all_artifacts = video_artifacts;
        all_artifacts.push(ToolArtifact {
            kind: "video".into(),
            title: format!("批量生成汇总：{}", storyboard.title),
            content: summary_content,
        });

        let summary_text = if success_count == total_shots {
            format!(
                "已为《{}》批量生成全部 {} 个镜头（共 {} 秒）。每个镜头可独立下载，可使用剪辑工具拼接。",
                storyboard.title, success_count, total_seconds
            )
        } else {
            format!(
                "《{}》批量生成完成：共 {} 个镜头，成功 {} 个，失败 {} 个。成功的镜头可下载使用。",
                storyboard.title, total_shots, success_count, total_shots - success_count
            )
        };

        ToolResult::ok(summary_text, all_artifacts)
    }
}

/// 从宽高比推断 size label（复用 video_generate 逻辑，避免可见性问题）
fn infer_size_label(aspect_ratio: &str) -> (&'static str, u32, u32) {
    match aspect_ratio {
        "9:16" => ("720P", 720, 1280),
        "1:1" => ("720P", 720, 720),
        "4:3" => ("720P", 960, 720),
        "3:4" => ("720P", 720, 960),
        "21:9" => ("720P", 1680, 720),
        _ => ("720P", 1280, 720),
    }
}
