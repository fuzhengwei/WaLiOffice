// 渲染模块（2-1 节 stub，完整实现在第 3 部分）
//
// docx_render.rs → 第 3-2 节 doc_generate 工具（底层：docx-rs 库，纯 Rust 生成 .docx）
// xlsx_render.rs → 第 3-4 节 sheet_generate 工具（底层：rust_xlsxwriter 库，纯 Rust 生成 .xlsx）
// pptx_render.rs → 第 3-3 节 ppt_generate 工具（底层：手写 OOXML，生成 HTML 幻灯片由前端渲染）

pub mod docx_render;
pub mod xlsx_render;
pub mod pptx_render;

use std::path::PathBuf;

/// 渲染产物输出路径（2-1 节 stub，完整实现在各渲染器中）
pub fn output_path(filename: &str) -> PathBuf {
    let cfg = crate::config::config();
    let sanitized: String = filename
        .chars()
        .filter(|c| !matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'))
        .collect();
    PathBuf::from(&cfg.render_output_dir).join(sanitized)
}
