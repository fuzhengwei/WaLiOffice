// Word 文档渲染（2-1 节 stub，完整实现在 3-2 节）
//
// 底层使用 docx-rs 库，纯 Rust 生成 .docx 文件。
// 完整实现包含：标题/段落/列表/表格的渲染逻辑，以及文件 ZIP 打包。

use std::io::Write;
use std::path::Path;
use serde::Deserialize;

// ─── 数据类型（与 doc_export.rs 耦合，完整实现见 3-2 节） ─────────────────────

#[derive(Deserialize)]
pub struct DocData {
    pub title: String,
    pub sections: Vec<DocSection>,
}

#[derive(Deserialize)]
pub struct DocSection {
    pub heading: String,
    pub heading_level: i32,
    pub paragraphs: Vec<String>,
    pub bullets: Vec<String>,
    pub table: Option<DocTable>,
}

#[derive(Deserialize)]
pub struct DocTable {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

// ─── 渲染函数（2-1 stub） ────────────────────────────────────────────────────

/// 渲染 Word 文档（2-1 节暂不实现，输出空文件）
pub fn render_docx(data: &DocData, path: &Path) -> anyhow::Result<()> {
    // 完整实现在 3-2 节：
    // 1. 用 docx-rs 构建 document.xml
    // 2. 写入 ZIP 包（[Content_Types].xml / _rels/.rels / word/document.xml 等部件）
    // 3. 返回文件路径
    tracing::warn!(
        "render_docx stub: {} ({} sections), real impl in 3-2",
        data.title,
        data.sections.len()
    );
    // 写一个空文件占位
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::File::create(path)?.write_all(b"")?;
    Ok(())
}
