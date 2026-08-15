use anyhow::Result;
use docx_rs::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocSection {
    pub heading: String,
    #[serde(default = "default_level")]
    pub heading_level: u32,
    #[serde(default)]
    pub paragraphs: Vec<String>,
    #[serde(default)]
    pub bullets: Vec<String>,
    #[serde(default)]
    pub table: Option<DocTable>,
}

fn default_level() -> u32 { 1 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocTable {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocData {
    pub title: String,
    pub sections: Vec<DocSection>,
}

/// 渲染 DOCX 文件，返回文件路径
pub fn render_docx(data: &DocData, output_path: &std::path::Path) -> Result<()> {
    let mut doc = Docx::new();

    // 标题
    doc = doc.add_paragraph(
        Paragraph::new().add_run(Run::new().add_text(&data.title).bold().size(56))
    );

    for section in &data.sections {
        let heading_size = match section.heading_level {
            1 => 36,
            2 => 30,
            _ => 26,
        };
        doc = doc.add_paragraph(
            Paragraph::new().add_run(Run::new().add_text(&section.heading).bold().size(heading_size))
        );

        for para in &section.paragraphs {
            let cleaned = strip_markdown(para);
            doc = doc.add_paragraph(
                Paragraph::new().add_run(Run::new().add_text(&cleaned).size(22))
            );
        }

        for bullet in &section.bullets {
            let cleaned = strip_markdown(bullet);
            doc = doc.add_paragraph(
                Paragraph::new()
                    .add_run(Run::new().add_text(format!("• {cleaned}")).size(22))
            );
        }

        if let Some(table) = &section.table {
            if !table.headers.is_empty() {
                // 构建表头行
                let mut header_cells: Vec<TableCell> = Vec::new();
                for header in &table.headers {
                    header_cells.push(
                        TableCell::new().add_paragraph(
                            Paragraph::new().add_run(Run::new().add_text(header).bold().size(20))
                        )
                    );
                }
                let mut rows: Vec<TableRow> = vec![TableRow::new(header_cells)];

                // 数据行
                for row in &table.rows {
                    let mut cells: Vec<TableCell> = Vec::new();
                    for cell in row {
                        cells.push(
                            TableCell::new().add_paragraph(
                                Paragraph::new().add_run(Run::new().add_text(cell).size(20))
                            )
                        );
                    }
                    rows.push(TableRow::new(cells));
                }

                let docx_table = Table::new(rows);
                doc = doc.add_table(docx_table);
            }
        }
    }

    let file = std::fs::File::create(output_path)?;
    doc.build().pack(file)?;
    Ok(())
}

fn strip_markdown(text: &str) -> String {
    text.replace("**", "").replace('*', "")
}
