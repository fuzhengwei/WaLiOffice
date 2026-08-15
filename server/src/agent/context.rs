use crate::llm::LlmClient;
use crate::models::ChatMessage;

#[derive(Clone)]
pub struct ContextConfig {
    pub message_threshold: usize,
    pub max_context_tokens: usize,
    pub keep_recent: usize,
    pub tool_result_clear_threshold: usize,
}

pub const DEFAULT_CONTEXT_CONFIG: ContextConfig = ContextConfig {
    message_threshold: 20,
    max_context_tokens: 16000,
    keep_recent: 8,
    tool_result_clear_threshold: 500,
};

/// Token 估算（粗略：1 中文字 ≈ 2 token，1 英文字 ≈ 0.25 token）
pub fn estimate_tokens(messages: &[ChatMessage]) -> usize {
    messages
        .iter()
        .map(|msg| {
            let content = &msg.content;
            let chinese = content
                .chars()
                .filter(|c| ('\u{4e00}'..='\u{9fff}').contains(c))
                .count();
            let other = content.chars().count() - chinese;
            chinese * 2 + (other as f64 * 0.25) as usize
        })
        .sum()
}

/// microCompact：清除旧 tool_result 内容
pub fn micro_compact(messages: &[ChatMessage], config: &ContextConfig) -> Vec<ChatMessage> {
    let len = messages.len();
    messages
        .iter()
        .enumerate()
        .map(|(idx, msg)| {
            // 保留最近 4 条消息中的 tool_result 不清除
            if msg.role == "tool" && idx + 4 < len {
                if msg.content.len() > config.tool_result_clear_threshold {
                    let mut m = msg.clone();
                    m.content = "[tool result cleared]".to_string();
                    return m;
                }
            }
            msg.clone()
        })
        .collect()
}

/// summaryCompact：用 LLM 摘要替换旧消息
pub async fn summary_compact(
    messages: Vec<ChatMessage>,
    config: &ContextConfig,
    client: &LlmClient,
) -> Vec<ChatMessage> {
    if messages.len() <= config.message_threshold {
        return messages;
    }

    let tokens = estimate_tokens(&messages);
    if tokens <= config.max_context_tokens {
        return messages;
    }

    let split_at = messages.len() - config.keep_recent;
    let old_messages = &messages[..split_at];
    let recent_messages = messages[split_at..].to_vec();

    let old_content: String = old_messages
        .iter()
        .filter(|m| m.role == "user" || m.role == "assistant")
        .map(|m| {
            format!(
                "[{}]: {}",
                m.role,
                m.content.chars().take(300).collect::<String>()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    if old_content.trim().is_empty() {
        return recent_messages;
    }

    let summary_messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: "你是对话摘要助手。请简洁总结以下对话的关键信息，保留用户需求、已完成的工具调用和关键结论。不要编造信息。".to_string(),
            tool_calls: None,
            tool_call_id: None,
        },
        ChatMessage {
            role: "user".to_string(),
            content: format!("请总结以下对话历史：\n{old_content}\n\n返回简洁摘要（200字以内）："),
            tool_calls: None,
            tool_call_id: None,
        },
    ];

    match client.chat(&summary_messages, None).await {
        Ok(resp) => {
            let summary = resp
                .choices
                .first()
                .and_then(|c| c.message.content.as_deref())
                .unwrap_or("")
                .trim()
                .to_string();

            let mut result = vec![ChatMessage {
                role: "system".to_string(),
                content: format!("[对话摘要] {summary}"),
                tool_calls: None,
                tool_call_id: None,
            }];
            result.extend(recent_messages);
            result
        }
        Err(e) => {
            tracing::warn!("[ContextManager] summaryCompact failed: {e}");
            recent_messages
        }
    }
}

/// 完整压缩管道
pub async fn compact_context(
    messages: Vec<ChatMessage>,
    config: &ContextConfig,
    client: &LlmClient,
) -> Vec<ChatMessage> {
    let micro = micro_compact(&messages, config);
    summary_compact(micro, config, client).await
}
