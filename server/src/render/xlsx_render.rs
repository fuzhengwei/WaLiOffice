use anyhow::Result;
use rust_xlsxwriter::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SheetTable {
    pub title: String,
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SheetData {
    pub title: String,
    pub tables: Vec<SheetTable>,
}

pub fn render_xlsx(data: &SheetData, output_path: &std::path::Path) -> Result<()> {
    let mut workbook = Workbook::new();

    for (idx, table) in data.tables.iter().enumerate() {
        let sheet_name = if data.tables.len() == 1 {
            sanitize_sheet_name(&data.title)
        } else {
            sanitize_sheet_name(&format!("{}_{}", idx + 1, table.title))
        };
        let worksheet = workbook.add_worksheet();
        worksheet.set_name(&sheet_name)?;

        // 标题行
        if data.tables.len() > 1 {
            worksheet.write_string(0, 0, &table.title)?;
            worksheet.set_column_width(0, 20)?;
        }

        let header_row = if data.tables.len() > 1 { 1u32 } else { 0u32 };

        // 表头
        let header_format = Format::new()
            .set_bold()
            .set_background_color(0xDC2626)
            .set_font_color(0xFFFFFF);

        for (col, header) in table.headers.iter().enumerate() {
            worksheet.write_string_with_format(header_row, col as u16, header, &header_format)?;
            worksheet.set_column_width(col as u16, 18)?;
        }

        // 数据行
        for (row_idx, row) in table.rows.iter().enumerate() {
            for (col, cell) in row.iter().enumerate() {
                let r = header_row + 1 + row_idx as u32;
                // 尝试写数字，否则写字符串
                if let Ok(num) = cell.parse::<f64>() {
                    worksheet.write_number(r, col as u16, num)?;
                } else {
                    worksheet.write_string(r, col as u16, cell)?;
                }
            }
        }

        // 自适应列宽（简单版）
        worksheet.autofit();
    }

    workbook.save(output_path)?;
    Ok(())
}

fn sanitize_sheet_name(name: &str) -> String {
    // Excel sheet 名限制：≤31 字符，不能含 : \ / ? * [ ]
    let cleaned: String = name
        .chars()
        .filter(|c| !matches!(c, ':' | '\\' | '/' | '?' | '*' | '[' | ']'))
        .collect();
    let cleaned = if cleaned.is_empty() { "Sheet".to_string() } else { cleaned };
    cleaned.chars().take(31).collect()
}
