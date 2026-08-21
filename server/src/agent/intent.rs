// 意图识别系统（2-1 节 stub，完整实现在 2-6 节）
//
// 完整实现见第 2-6 节「System Prompt 与意图识别」：
// - 14 种 IntentType：Ppt/Doc/Markdown/Sheet/Chart/Drawio/Image/Video/WebSearch/TextGenerate/ImageUnderstanding/Chat/Compound/Unknown
// - primary_tool / tool_kind 映射
// - 办公场景子意图识别、时序规则、指代消解

use serde::{Deserialize, Serialize};

/// 意图类型枚举
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IntentType {
    Ppt,
    Doc,
    Markdown,
    Sheet,
    Chart,
    Drawio,
    Image,
    Video,
    WebSearch,
    TextGenerate,
    ImageUnderstanding,
    Chat,
    Compound,
    Unknown,
}

/// 意图分析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentResult {
    pub intent: IntentType,
    pub primary_tool: String,
    pub tool_kind: String,
    pub confidence: f32,
}

/// 意图分析器
pub struct IntentAnalyzer;

impl IntentAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// 分析用户消息意图（2-1 节暂不实现，返回 Unknown）
    ///
    /// 完整实现在 2-6 节：
    /// - 基于关键词规则 + LLM 分类器判断意图
    /// - 识别时序规则（"先做 X 再做 Y"）
    /// - 识别指代消解（接续上文任务）
    pub fn analyze(&self, _message: &str) -> IntentResult {
        IntentResult {
            intent: IntentType::Unknown,
            primary_tool: "chat".to_string(),
            tool_kind: "general".to_string(),
            confidence: 0.0,
        }
    }
}

impl Default for IntentAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
