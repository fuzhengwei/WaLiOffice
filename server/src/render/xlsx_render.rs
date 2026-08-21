// Excel 表格渲染（2-1 节 stub，完整实现在 3-4 节）
//
// 底层使用 rust_xlsxwriter 库，纯 Rust 生成 .xlsx 文件。
// 完整实现包含：多 Sheet 构建、单元格格式化、图表数据渲染。

use std::io::Write;
use std::path::Path;
use serde::Deserialize;

// ─── 数据类型（与 doc_export.rs 耦合，完整实现见 3-4 节） ─────────────────────

#[derive(Deserialize)]
pub struct SheetData {
    pub title: String,
    pub tables: Vec<SheetTable>,
}

#[derive(Deserialize)]
pub struct SheetTable {
    pub title: String,
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

// ─── 渲染函数（2-1 stub） ────────────────────────────────────────────────────

/// 渲染 Excel 表格（2-1 节暂不实现，输出空文件）
pub fn render_xlsx(data: &SheetData, path: &Path) -> anyhow::Result<()> {
    // 完整实现在 3-4 节：
    // 1. 用 rust_xlsxwriter 构建 workbook
    // 2. 多 Sheet 时 sheet 名为 idx+1_标题
    // 3. set_bold 表头格式化
    // 4. 返回文件路径
    tracing::warn!(
        "render_xlsx stub: {} ({} tables), real impl in 3-4",
        data.title,
        data.tables.len()
    );
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::File::create(path)?.write_all(b"")?;
    Ok(())
}
