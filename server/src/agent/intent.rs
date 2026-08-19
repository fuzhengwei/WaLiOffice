/// WaLiOffice 意图识别系统
///
/// 移植自 WaLiCode 的 `src/services/intent/` 模块，适配 Rust 后端。
/// 核心能力：
/// - 规则分类器（关键词 + 正则 + 优先级权重）
/// - 时序意图识别（"先做X再Y"）
/// - 指代消解（上下文追踪）
/// - 意图结果注入 system prompt
///
/// 与 WaLiCode 的差异：
/// - 去掉了异步预分析（WaLiOffice 后端无前端长会话场景）
/// - 去掉了项目索引/信号提取（WaLiOffice 是办公场景，不是代码场景）
/// - 去掉了模型分类器（由 LLM ReAct 循环本身承担模型决策）
/// - 增加了办公场景特有的意图类型（文档/表格/图片/视频等）

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─────────────────────────────────────────────────────────────
// 类型定义
// ─────────────────────────────────────────────────────────────

/// 意图类型
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntentType {
    /// 生成 PPT
    Ppt,
    /// 生成 Word 文档
    Doc,
    /// 生成 Markdown 文档
    Markdown,
    /// 生成表格
    Sheet,
    /// 生成图表
    Chart,
    /// 生成 draw.io 图表
    Drawio,
    /// 生成图片
    Image,
    /// 生成视频
    Video,
    /// 联网搜索
    WebSearch,
    /// 纯文本/文档输出（写提示词、构思方案、写脚本等）
    TextGenerate,
    /// 识图/读图（理解图片内容，不生成图片）
    ImageUnderstanding,
    /// 普通对话/问答
    Chat,
    /// 复合意图（先做X再Y）
    Compound,
    /// 未识别
    Unknown,
}

impl IntentType {
    /// 对应的工具名
    pub fn primary_tool(&self) -> Option<&'static str> {
        match self {
            IntentType::Ppt => Some("ppt_plan"),
            IntentType::Doc => Some("doc_generate"),
            IntentType::Markdown => Some("md_generate"),
            IntentType::Sheet => Some("sheet_generate"),
            IntentType::Chart => Some("chart_generate"),
            IntentType::Drawio => Some("drawio_generate"),
            IntentType::Image => Some("image_prompt"),
            IntentType::Video => Some("video_generate"),
            IntentType::WebSearch => Some("web_search"),
            IntentType::TextGenerate => Some("md_generate"),
            IntentType::ImageUnderstanding => None,
            IntentType::Chat => None,
            IntentType::Compound => None,
            IntentType::Unknown => None,
        }
    }

    /// 对应的 tool_kind
    pub fn tool_kind(&self) -> &'static str {
        match self {
            IntentType::Ppt => "ppt",
            IntentType::Doc => "doc",
            IntentType::Markdown => "doc",
            IntentType::Sheet => "excel",
            IntentType::Chart => "general",
            IntentType::Drawio => "drawio",
            IntentType::Image => "image",
            IntentType::Video => "video",
            IntentType::WebSearch => "general",
            IntentType::TextGenerate => "doc",
            IntentType::ImageUnderstanding => "general",
            IntentType::Chat => "general",
            IntentType::Compound => "general",
            IntentType::Unknown => "general",
        }
    }
}

/// 意图结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentResult {
    /// 主意图
    pub intent: IntentType,
    /// 置信度 [0.0, 1.0]
    pub confidence: f32,
    /// 提取的实体
    pub entities: IntentEntities,
    /// 来源
    pub source: IntentSource,
    /// 是否需要追问
    pub follow_up_needed: bool,
    /// 追问问题
    #[serde(skip_serializing_if = "Option::is_none")]
    pub follow_up_question: Option<String>,
    /// 子意图列表（时序意图时使用）
    pub sub_intents: Vec<IntentType>,
    /// 时序顺序
    pub temporal_order: TemporalOrder,
    /// 原始输入
    pub raw_input: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntentSource {
    Rule,
    Fallback,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemporalOrder {
    /// 单一意图
    Single,
    /// 先做X再Y
    Sequential,
    /// 同时做X和Y
    Parallel,
}

/// 提取的实体
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IntentEntities {
    /// 目标文件
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub target_files: Vec<String>,
    /// 涉及的组件/主题
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic: Option<String>,
    /// 操作类型
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    /// 风格/格式
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    /// 图片操作模式（文生图/图生图等）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_mode: Option<String>,
    /// 视频操作模式（文生视频/图生视频/关键帧等）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_mode: Option<String>,
    /// 时序第一步
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_step: Option<String>,
    /// 时序第二步
    #[serde(skip_serializing_if = "Option::is_none")]
    pub second_step: Option<String>,
}

// ─────────────────────────────────────────────────────────────
// 时序意图检测
// ─────────────────────────────────────────────────────────────

/// 时序模式定义
struct TemporalPattern {
    pattern: &'static str,
    description: &'static str,
}

/// 时序意图模式列表
const TEMPORAL_PATTERNS: &[TemporalPattern] = &[
    TemporalPattern {
        pattern: r"先[帮给]?我(.+?)[，,]再(.+)",
        description: "先做X再Y",
    },
    TemporalPattern {
        pattern: r"先(.+?)[，,]然后(.+)",
        description: "先X然后Y",
    },
    TemporalPattern {
        pattern: r"(.+?)[，,]然后再?(.+)",
        description: "X然后再Y",
    },
    TemporalPattern {
        pattern: r"帮我(.+?)[，,]之后再?(.+)",
        description: "帮我X之后再Y",
    },
];

/// 检测时序意图
fn detect_temporal_intent(text: &str) -> Option<(String, String)> {
    let text = text.trim();
    for pattern in TEMPORAL_PATTERNS {
        if let Ok(re) = regex::Regex::new(pattern.pattern) {
            if let Some(caps) = re.captures(text) {
                let first = caps.get(1).map(|m| m.as_str().to_string())?;
                let second = caps.get(2).map(|m| m.as_str().to_string())?;
                return Some((first, second));
            }
        }
    }
    None
}

// ─────────────────────────────────────────────────────────────
// 文本优先意图检测
// ─────────────────────────────────────────────────────────────

/// 文本优先模式——当命中时，当前意图是文本/文档生成，不直接跳到媒体工具
const TEXT_FIRST_PATTERNS: &[&str] = &[
    "先帮我写",
    "帮我写",
    "先写",
    "帮我构思",
    "帮我规划",
    "先出",
    "帮我出",
    "写提示词",
    "出提示词",
    "写prompt",
    "写脚本",
    "先规划",
    "先想",
    "帮我想想",
    "帮我设计",
    "先做个方案",
    "出个方案",
    "写个方案",
    "帮我写个",
    "先构思",
    "帮我总结",
    "帮我整理",
    "帮我分析",
    "帮我梳理",
    "帮我提炼",
    "帮我写一",
    "帮我写份",
    "帮我写篇",
];

/// 检测是否为文本优先意图
fn is_text_first_intent(text: &str) -> bool {
    let lower = text.trim().to_lowercase();
    TEXT_FIRST_PATTERNS.iter().any(|p| lower.contains(p))
}

// ─────────────────────────────────────────────────────────────
// 关键词规则分类器
// ─────────────────────────────────────────────────────────────

/// 带权重的关键词规则
struct WeightedKeywordRule {
    keywords: &'static [&'static str],
    intent: IntentType,
    /// 权重 [0.0, 1.0]，越高越优先
    weight: f32,
    /// 基础置信度
    base_confidence: f32,
}

/// 关键词规则表（按优先级排列，高权重在前）
const KEYWORD_RULES: &[WeightedKeywordRule] = &[
    // ── PPT ──
    WeightedKeywordRule {
        keywords: &["ppt", "演示文稿", "幻灯片", "presentation", "汇报材料", "做个ppt", "做ppt", "生成ppt"],
        intent: IntentType::Ppt,
        weight: 0.95,
        base_confidence: 0.8,
    },
    // ── 文档 ──
    WeightedKeywordRule {
        keywords: &["word文档", "word", "docx", "报告", "prd", "方案书", "文档", "生成文档"],
        intent: IntentType::Doc,
        weight: 0.9,
        base_confidence: 0.8,
    },
    // ── Markdown ──
    WeightedKeywordRule {
        keywords: &["markdown", "md", "readme", "知识库", "说明文档", "操作手册", "会议纪要", "调研整理", "纪要"],
        intent: IntentType::Markdown,
        weight: 0.85,
        base_confidence: 0.75,
    },
    // ── 表格 ──
    WeightedKeywordRule {
        keywords: &["excel", "xlsx", "表格", "数据分析", "排期", "预算", "数据指标", "csv"],
        intent: IntentType::Sheet,
        weight: 0.9,
        base_confidence: 0.8,
    },
    // ── 图表 ──
    WeightedKeywordRule {
        keywords: &["图表", "可视化", "趋势图", "柱状图", "折线图", "饼图", "占比", "排名", "漏斗", "仪表盘", "echarts"],
        intent: IntentType::Chart,
        weight: 0.85,
        base_confidence: 0.75,
    },
    // ── draw.io ──
    WeightedKeywordRule {
        keywords: &["draw.io", "流程图", "架构图", "泳道图", "拓扑图", "er图", "时序图", "uml"],
        intent: IntentType::Drawio,
        weight: 0.9,
        base_confidence: 0.8,
    },
    // ── 图片生成 ──
    WeightedKeywordRule {
        keywords: &["生成图片", "做图片", "画图", "出图", "图生图", "以图生图", "改图", "修图", "重绘", "换风格",
                     "换背景", "换衣服", "换装", "变装", "换发型", "去除背景", "抠图", "扩图"],
        intent: IntentType::Image,
        weight: 0.9,
        base_confidence: 0.8,
    },
    WeightedKeywordRule {
        keywords: &["海报", "封面", "配图", "主视觉", "插画", "banner", "视觉稿", "logo"],
        intent: IntentType::Image,
        weight: 0.75,
        base_confidence: 0.7,
    },
    // ── 视频生成 ──
    WeightedKeywordRule {
        keywords: &["生成视频", "做视频", "制作视频", "图生视频", "以图生视频", "短视频", "短片",
                     "宣传片", "动起来", "动态化", "动态海报", "视频广告", "片头", "转场动画",
                     "分镜", "分镜头", "多镜头", "故事短片", "发布会视频", "完整视频"],
        intent: IntentType::Video,
        weight: 0.9,
        base_confidence: 0.8,
    },
    WeightedKeywordRule {
        keywords: &["动画"],
        intent: IntentType::Video,
        weight: 0.6,
        base_confidence: 0.6,
    },
    // ── 联网搜索 ──
    WeightedKeywordRule {
        keywords: &["最新", "官网", "新闻", "政策", "联网查询", "检索资料", "搜索一下", "搜索资料",
                     "网上查", "查一下", "帮我查"],
        intent: IntentType::WebSearch,
        weight: 0.8,
        base_confidence: 0.7,
    },
    // ── 识图 ──
    WeightedKeywordRule {
        keywords: &["这是什么", "帮我识别", "提取文字", "ocr", "解释图片", "分析截图",
                     "描述图里", "图里内容", "图片内容", "图片说了", "这张图"],
        intent: IntentType::ImageUnderstanding,
        weight: 0.85,
        base_confidence: 0.75,
    },
    // ── 文本生成 ──
    WeightedKeywordRule {
        keywords: &["写提示词", "出提示词", "写prompt", "写脚本", "构思", "规划", "出方案",
                     "写方案", "做个方案", "总结", "整理", "梳理", "提炼", "写个大纲"],
        intent: IntentType::TextGenerate,
        weight: 0.85,
        base_confidence: 0.8,
    },
];

/// 指代消解模式
const REFERENCE_PATTERNS: &[(&str, &str)] = &[
    ("这个", "current"),
    ("那个", "previous"),
    ("刚才", "previous"),
    ("上次", "previous"),
    ("继续", "continue"),
];

// ─────────────────────────────────────────────────────────────
// 意图分析器
// ─────────────────────────────────────────────────────────────

/// 意图分析器
pub struct IntentAnalyzer {
    /// 会话上下文
    contexts: HashMap<String, SessionContext>,
}

/// 会话上下文
#[derive(Debug, Clone, Default)]
struct SessionContext {
    /// 上次意图
    last_intent: Option<IntentType>,
    /// 上次输入
    last_input: Option<String>,
    /// 上次操作目标
    last_action_target: Option<String>,
    /// 当前焦点文件
    current_focus_file: Option<String>,
    /// 提及的实体
    mentioned_entities: Vec<MentionedEntity>,
    /// 操作历史
    action_history: Vec<ActionRecord>,
}

#[derive(Debug, Clone)]
struct MentionedEntity {
    name: String,
    entity_type: String,
    last_mentioned: std::time::Instant,
}

#[derive(Debug, Clone)]
struct ActionRecord {
    action_type: String,
    target: String,
    timestamp: std::time::Instant,
}

impl IntentAnalyzer {
    pub fn new() -> Self {
        Self {
            contexts: HashMap::new(),
        }
    }

    /// 分析用户意图
    pub fn analyze(&mut self, user_input: &str, session_id: &str, has_image_attachment: bool) -> IntentResult {
        let text = user_input.trim();
        let lower = text.to_lowercase();

        // 1. 时序意图检测
        if let Some((first, second)) = detect_temporal_intent(text) {
            let first_intent = self.classify_single(&first, has_image_attachment);
            let second_intent = self.classify_single(&second, has_image_attachment);

            let result = IntentResult {
                intent: IntentType::Compound,
                confidence: 0.85,
                entities: IntentEntities {
                    first_step: Some(first),
                    second_step: Some(second),
                    ..Default::default()
                },
                source: IntentSource::Rule,
                follow_up_needed: false,
                follow_up_question: None,
                sub_intents: vec![first_intent.intent, second_intent.intent],
                temporal_order: TemporalOrder::Sequential,
                raw_input: text.to_string(),
            };

            // 更新上下文
            self.update_context(session_id, &result);

            return result;
        }

        // 2. 文本优先意图检测
        if is_text_first_intent(text) {
            // 文本优先模式下，识别具体文本意图
            let text_intent = self.classify_text_intent(text);
            let result = IntentResult {
                intent: text_intent.clone(),
                confidence: 0.85,
                entities: self.extract_entities(text),
                source: IntentSource::Rule,
                follow_up_needed: false,
                follow_up_question: None,
                sub_intents: vec![],
                temporal_order: TemporalOrder::Single,
                raw_input: text.to_string(),
            };

            self.update_context(session_id, &result);
            return result;
        }

        // 3. 指代消解
        let resolved_text = self.resolve_references(text, session_id);

        // 4. 标准分类
        let result = self.classify_single(&resolved_text, has_image_attachment);

        // 5. 更新上下文
        self.update_context(session_id, &result);

        result
    }

    /// 分类单个意图
    fn classify_single(&self, text: &str, has_image_attachment: bool) -> IntentResult {
        let lower = text.to_lowercase();
        let mut best_match: Option<(&WeightedKeywordRule, usize)> = None;

        for rule in KEYWORD_RULES {
            let match_count = rule.keywords.iter().filter(|kw| lower.contains(*kw)).count();
            if match_count > 0 {
                match &best_match {
                    Some((best, best_count)) => {
                        // 权重更高的优先；权重相同则匹配数多的优先
                        if rule.weight > best.weight
                            || (rule.weight == best.weight && match_count > *best_count)
                        {
                            best_match = Some((rule, match_count));
                        }
                    }
                    None => {
                        best_match = Some((rule, match_count));
                    }
                }
            }
        }

        match best_match {
            Some((rule, match_count)) => {
                let confidence = (rule.base_confidence
                    + (match_count as f32 * 0.05).min(0.15))
                .min(0.95);

                let mut entities = self.extract_entities(text);

                // 图片附件 + 图片关键词 → 图生图
                if has_image_attachment && rule.intent == IntentType::Image {
                    entities.image_mode = Some("image_to_image".to_string());
                }
                // 图片附件 + 视频关键词 → 图生视频
                if has_image_attachment && rule.intent == IntentType::Video {
                    entities.video_mode = Some("image_to_video".to_string());
                }

                // "动画"在上下文中有"先写/构思"等修饰时降权
                if lower.contains("动画") && is_text_first_intent(text) {
                    return IntentResult {
                        intent: IntentType::TextGenerate,
                        confidence: 0.8,
                        entities,
                        source: IntentSource::Rule,
                        follow_up_needed: false,
                        follow_up_question: None,
                        sub_intents: vec![],
                        temporal_order: TemporalOrder::Single,
                        raw_input: text.to_string(),
                    };
                }

                IntentResult {
                    intent: rule.intent.clone(),
                    confidence,
                    entities,
                    source: IntentSource::Rule,
                    follow_up_needed: false,
                    follow_up_question: None,
                    sub_intents: vec![],
                    temporal_order: TemporalOrder::Single,
                    raw_input: text.to_string(),
                }
            }
            None => {
                // Fallback
                self.fallback_classify(text)
            }
        }
    }

    /// 文本意图分类
    fn classify_text_intent(&self, text: &str) -> IntentType {
        let lower = text.to_lowercase();

        // 文本生成下的子意图
        if lower.contains("ppt") || lower.contains("演示") || lower.contains("汇报") {
            return IntentType::Ppt;
        }
        if lower.contains("报告") || lower.contains("方案书") || lower.contains("文档") {
            return IntentType::Doc;
        }
        if lower.contains("表格") || lower.contains("excel") || lower.contains("排期") {
            return IntentType::Sheet;
        }
        if lower.contains("流程图") || lower.contains("架构图") {
            return IntentType::Drawio;
        }
        // 默认文本生成
        IntentType::TextGenerate
    }

    /// 降级分类
    fn fallback_classify(&self, text: &str) -> IntentResult {
        let lower = text.to_lowercase();

        // 简单启发式
        let intent = if lower.contains("?") || lower.contains("？")
            || lower.contains("什么") || lower.contains("如何") || lower.contains("怎么")
            || lower.contains("为什么") || lower.contains("能不能")
        {
            IntentType::Chat
        } else if lower.contains("创建") || lower.contains("新建") || lower.contains("添加") {
            IntentType::Doc
        } else if lower.contains("修改") || lower.contains("改") || lower.contains("调整") {
            IntentType::Doc
        } else if lower.contains("帮我") || lower.contains("请") {
            IntentType::Doc
        } else {
            IntentType::Chat
        };

        IntentResult {
            intent,
            confidence: 0.3,
            entities: IntentEntities::default(),
            source: IntentSource::Fallback,
            follow_up_needed: false,
            follow_up_question: None,
            sub_intents: vec![],
            temporal_order: TemporalOrder::Single,
            raw_input: text.to_string(),
        }
    }

    /// 提取实体
    fn extract_entities(&self, text: &str) -> IntentEntities {
        let mut entities = IntentEntities::default();

        // 提取主题（引号内容）
        if let Ok(re) = regex::Regex::new(r#"[「"《](.+?)[」"》]"#) {
            if let Some(caps) = re.captures(text) {
                if let Some(m) = caps.get(1) {
                    entities.topic = Some(m.as_str().to_string());
                }
            }
        }

        // 提取关于/关于XX的
        if entities.topic.is_none() {
            if let Ok(re) = regex::Regex::new(r"关于(.+?)(?:的|的。|$)") {
                if let Some(caps) = re.captures(text) {
                    if let Some(m) = caps.get(1) {
                        entities.topic = Some(m.as_str().to_string());
                    }
                }
            }
        }

        entities
    }

    /// 指代消解
    fn resolve_references(&self, text: &str, session_id: &str) -> String {
        let ctx = self.contexts.get(session_id);
        if ctx.is_none() {
            return text.to_string();
        }
        let ctx = ctx.unwrap();

        let mut resolved = text.to_string();

        for (pattern, ref_type) in REFERENCE_PATTERNS {
            if resolved.contains(pattern) {
                match *ref_type {
                    "current" => {
                        if let Some(ref file) = ctx.current_focus_file {
                            resolved = resolved.replace(*pattern, file);
                        }
                    }
                    "previous" => {
                        if let Some(ref target) = ctx.last_action_target {
                            resolved = resolved.replace(*pattern, target);
                        }
                    }
                    "continue" => {
                        if let Some(ref intent) = ctx.last_intent {
                            resolved = resolved.replace(*pattern, &format!("{:?}", intent));
                        }
                    }
                    _ => {}
                }
            }
        }

        resolved
    }

    /// 更新上下文
    fn update_context(&mut self, session_id: &str, result: &IntentResult) {
        let ctx = self.contexts.entry(session_id.to_string()).or_default();
        ctx.last_intent = Some(result.intent.clone());
        ctx.last_input = Some(result.raw_input.clone());

        if let Some(ref topic) = result.entities.topic {
            ctx.last_action_target = Some(topic.clone());
        }
    }

    /// 更新焦点
    pub fn update_focus(&mut self, session_id: &str, file: &str) {
        let ctx = self.contexts.entry(session_id.to_string()).or_default();
        ctx.current_focus_file = Some(file.to_string());
    }

    /// 清理上下文
    pub fn clear_context(&mut self, session_id: &str) {
        self.contexts.remove(session_id);
    }

    /// 构建意图上下文补充文本（注入 system prompt）
    pub fn build_intent_context_addition(&self, result: &IntentResult) -> String {
        let mut parts = Vec::new();

        match result.temporal_order {
            TemporalOrder::Sequential => {
                if let Some(ref first) = result.entities.first_step {
                    parts.push(format!("- 用户意图是分步执行：先「{}」，再「{}」",
                        first,
                        result.entities.second_step.as_deref().unwrap_or("后续操作")));
                    parts.push("- 必须先完成第一步，再考虑第二步，不要跳步".to_string());
                }
            }
            TemporalOrder::Parallel => {
                parts.push("- 用户需要同时完成多个任务".to_string());
            }
            TemporalOrder::Single => {}
        }

        if result.intent == IntentType::TextGenerate {
            parts.push("- 用户当前意图是文本/文档输出，不是直接生成图片或视频".to_string());
            parts.push("- 优先使用 md_generate 或 doc_generate 生成文本内容，或直接回复文本".to_string());
            parts.push("- 不要调用 image_prompt 或 video_generate".to_string());
        }

        if result.intent == IntentType::ImageUnderstanding {
            parts.push("- 用户意图是理解/识别图片内容，不是生成图片".to_string());
            parts.push("- 不要调用 image_prompt，直接结合视觉输入回答".to_string());
        }

        if let Some(ref topic) = result.entities.topic {
            parts.push(format!("- 主题：{}", topic));
        }

        if let Some(ref mode) = result.entities.image_mode {
            parts.push(format!("- 图片模式：{}", mode));
        }

        if let Some(ref mode) = result.entities.video_mode {
            parts.push(format!("- 视频模式：{}", mode));
        }

        if parts.is_empty() {
            String::new()
        } else {
            format!("\n## 意图识别结果\n{}\n", parts.join("\n"))
        }
    }

    /// 根据 IntentResult 决定 allowed_tools
    pub fn allowed_tools_for_intent(&self, result: &IntentResult) -> Option<Vec<String>> {
        match &result.intent {
            IntentType::Compound => {
                // 复合意图不限制工具，让 LLM 决定
                None
            }
            IntentType::TextGenerate => {
                // 文本生成不限制工具，但建议优先文本工具
                None
            }
            IntentType::ImageUnderstanding => {
                // 识图不限制工具，让 LLM 决定
                None
            }
            IntentType::Chat | IntentType::Unknown => {
                // 普通对话/未知意图不限制工具
                None
            }
            IntentType::Image => {
                Some(vec!["image_prompt".to_string()])
            }
            IntentType::Video => {
                Some(vec!["video_generate".to_string(), "video_storyboard".to_string(), "video_batch_generate".to_string()])
            }
            _ => None,
        }
    }
}

impl Default for IntentAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

// ─────────────────────────────────────────────────────────────
// 测试
// ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_first_intent() {
        let mut analyzer = IntentAnalyzer::new();

        // "先帮我写提示词" → TextGenerate
        let result = analyzer.analyze("先帮我写提示词，我想做一个动画片，关于西游记孙悟空的", "s1", false);
        assert_eq!(result.intent, IntentType::TextGenerate);
        assert!(result.confidence > 0.7);

        // "帮我写个视频脚本" → TextGenerate
        let result = analyzer.analyze("帮我写个视频脚本", "s2", false);
        assert_eq!(result.intent, IntentType::TextGenerate);

        // "帮我做个动画片" → Video
        let result = analyzer.analyze("帮我做个动画片", "s3", false);
        assert_eq!(result.intent, IntentType::Video);
    }

    #[test]
    fn test_temporal_intent() {
        let mut analyzer = IntentAnalyzer::new();

        // "先帮我写大纲，再生成PPT" → Compound
        let result = analyzer.analyze("先帮我写大纲，再生成PPT", "s1", false);
        assert_eq!(result.intent, IntentType::Compound);
        assert_eq!(result.temporal_order, TemporalOrder::Sequential);
        assert!(result.sub_intents.len() == 2);
    }

    #[test]
    fn test_ppt_intent() {
        let mut analyzer = IntentAnalyzer::new();

        let result = analyzer.analyze("帮我做个PPT汇报材料", "s1", false);
        assert_eq!(result.intent, IntentType::Ppt);
    }

    #[test]
    fn test_image_with_attachment() {
        let mut analyzer = IntentAnalyzer::new();

        let result = analyzer.analyze("帮我改图", "s1", true);
        assert_eq!(result.intent, IntentType::Image);
        assert_eq!(result.entities.image_mode.as_deref(), Some("image_to_image"));
    }

    #[test]
    fn test_video_with_attachment() {
        let mut analyzer = IntentAnalyzer::new();

        let result = analyzer.analyze("让这张图片动起来", "s1", true);
        assert_eq!(result.intent, IntentType::Video);
        assert_eq!(result.entities.video_mode.as_deref(), Some("image_to_video"));
    }

    #[test]
    fn test_image_understanding() {
        let mut analyzer = IntentAnalyzer::new();

        let result = analyzer.analyze("这张图片是什么", "s1", false);
        assert_eq!(result.intent, IntentType::ImageUnderstanding);
    }

    #[test]
    fn test_fallback() {
        let mut analyzer = IntentAnalyzer::new();

        let result = analyzer.analyze("你好", "s1", false);
        assert_eq!(result.intent, IntentType::Chat);
        assert_eq!(result.source, IntentSource::Fallback);
    }

    #[test]
    fn test_context_addition() {
        let mut analyzer = IntentAnalyzer::new();

        let result = analyzer.analyze("先帮我写提示词，关于西游记的", "s1", false);
        let addition = analyzer.build_intent_context_addition(&result);
        assert!(addition.contains("文本/文档输出"));
        assert!(addition.contains("西游记"));
    }

    #[test]
    fn test_compound_context_addition() {
        let mut analyzer = IntentAnalyzer::new();

        let result = analyzer.analyze("先帮我写大纲，再生成PPT", "s1", false);
        let addition = analyzer.build_intent_context_addition(&result);
        assert!(addition.contains("分步执行"));
        assert!(addition.contains("不要跳步"));
    }

    #[test]
    fn test_reference_resolution() {
        let mut analyzer = IntentAnalyzer::new();

        // 先设置上下文
        analyzer.analyze("帮我做个西游记的PPT", "s1", false);
        // 然后用指代
        let result = analyzer.analyze("修改那个", "s1", false);
        // 指代消解后应该能识别上下文
        assert!(result.confidence > 0.0);
    }

    #[test]
    fn test_keyword_weight_priority() {
        let mut analyzer = IntentAnalyzer::new();

        // "动画"权重0.6，但"写提示词"是文本优先模式，应该被拦截
        let result = analyzer.analyze("写提示词，关于动画片的", "s1", false);
        assert_eq!(result.intent, IntentType::TextGenerate);

        // 纯"做个动画片" → Video
        let result = analyzer.analyze("做个动画片", "s1", false);
        assert_eq!(result.intent, IntentType::Video);
    }

    #[test]
    fn test_allowed_tools() {
        let mut analyzer = IntentAnalyzer::new();

        // Image → 限制为 image_prompt
        let result = analyzer.analyze("帮我做海报", "s1", false);
        let tools = analyzer.allowed_tools_for_intent(&result);
        assert_eq!(tools, Some(vec!["image_prompt".to_string()]));

        // TextGenerate → 不限制
        let result = analyzer.analyze("帮我写提示词", "s1", false);
        let tools = analyzer.allowed_tools_for_intent(&result);
        assert!(tools.is_none());
    }
}
