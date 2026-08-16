use anyhow::{Context, Result};
use regex::Regex;
use serde::Serialize;
use serde_json::{json, Value};
use std::io::{Cursor, Read};
use std::process::Command;
use zip::ZipArchive;

const MAX_EXTRACTED_CHARS: usize = 200_000;

#[derive(Debug, Clone, Serialize)]
pub struct ExtractedFileText {
    pub text: String,
    pub parser: String,
    pub truncated: bool,
}

/// 结构化提取结果 — 用于前端富预览
#[derive(Debug, Clone, Serialize)]
pub struct StructuredPreview {
    pub preview_type: String, // "presentation" | "spreadsheet" | "document"
    pub data: Value,
    pub parser: String,
    pub truncated: bool,
}

pub fn extract_text_from_bytes(name: &str, mime_type: &str, bytes: &[u8]) -> ExtractedFileText {
    let result = extract_text(name, mime_type, bytes);
    let (text, parser) = match result {
        Ok((text, parser)) => (text, parser),
        Err(err) => (format!("文件解析失败：{err}"), "error".to_string()),
    };
    truncate_text(text, parser)
}

/// 结构化提取 — 返回 JSON，供前端富预览渲染
pub fn extract_structured(name: &str, mime_type: &str, bytes: &[u8]) -> StructuredPreview {
    let ext = extension(name);
    match ext.as_str() {
        "pptx" => match extract_pptx_structured(bytes) {
            Ok(data) => StructuredPreview {
                preview_type: "presentation".into(),
                data,
                parser: "pptx".into(),
                truncated: false,
            },
            Err(e) => StructuredPreview {
                preview_type: "presentation".into(),
                data: json!({ "error": format!("解析失败: {e}"), "slides": [] }),
                parser: "error".into(),
                truncated: false,
            },
        },
        "xlsx" => match extract_xlsx_structured(bytes) {
            Ok(data) => StructuredPreview {
                preview_type: "spreadsheet".into(),
                data,
                parser: "xlsx".into(),
                truncated: false,
            },
            Err(e) => StructuredPreview {
                preview_type: "spreadsheet".into(),
                data: json!({ "error": format!("解析失败: {e}"), "sheets": [] }),
                parser: "error".into(),
                truncated: false,
            },
        },
        "docx" => match extract_docx_structured(bytes) {
            Ok(data) => StructuredPreview {
                preview_type: "document".into(),
                data,
                parser: "docx".into(),
                truncated: false,
            },
            Err(e) => StructuredPreview {
                preview_type: "document".into(),
                data: json!({ "error": format!("解析失败: {e}"), "sections": [] }),
                parser: "error".into(),
                truncated: false,
            },
        },
        "csv" | "tsv" => {
            let delimiter = if ext == "tsv" { b'\t' } else { b',' };
            match extract_csv_structured(bytes, delimiter) {
                Ok(data) => StructuredPreview {
                    preview_type: "spreadsheet".into(),
                    data,
                    parser: "csv".into(),
                    truncated: false,
                },
                Err(e) => StructuredPreview {
                    preview_type: "spreadsheet".into(),
                    data: json!({ "error": format!("解析失败: {e}"), "sheets": [] }),
                    parser: "error".into(),
                    truncated: false,
                },
            }
        }
        _ => {
            // 非结构化类型，回退到纯文本
            let extracted = extract_text_from_bytes(name, mime_type, bytes);
            StructuredPreview {
                preview_type: "text".into(),
                data: json!({ "text": extracted.text }),
                parser: extracted.parser,
                truncated: extracted.truncated,
            }
        }
    }
}

fn extract_text(name: &str, mime_type: &str, bytes: &[u8]) -> Result<(String, String)> {
    let extension = extension(name);
    match extension.as_str() {
        "txt" | "md" | "markdown" | "csv" | "tsv" | "json" | "xml" | "html" | "htm" => {
            Ok((decode_text(bytes), "plain_text".into()))
        }
        "docx" => Ok((extract_docx(bytes)?, "docx".into())),
        "xlsx" => Ok((extract_xlsx(bytes)?, "xlsx".into())),
        "pptx" => Ok((extract_pptx(bytes)?, "pptx".into())),
        "pdf" => Ok((extract_pdf(name, bytes)?, "pdf".into())),
        _ if mime_type.starts_with("text/") => Ok((decode_text(bytes), "plain_text".into())),
        _ => Ok((String::new(), "unsupported".into())),
    }
}

fn extract_docx(bytes: &[u8]) -> Result<String> {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).context("打开 DOCX 压缩包")?;
    let mut xml = String::new();
    archive
        .by_name("word/document.xml")
        .context("读取 word/document.xml")?
        .read_to_string(&mut xml)?;
    Ok(xml_text(&xml))
}

fn extract_pptx(bytes: &[u8]) -> Result<String> {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).context("打开 PPTX 压缩包")?;
    let mut slide_names = Vec::new();
    for index in 0..archive.len() {
        let file = archive.by_index(index)?;
        let name = file.name().to_string();
        if name.starts_with("ppt/slides/slide") && name.ends_with(".xml") {
            slide_names.push(name);
        }
    }
    slide_names.sort_by_key(|name| numeric_suffix(name));

    let mut output = Vec::new();
    for (index, name) in slide_names.iter().enumerate() {
        let mut xml = String::new();
        archive.by_name(name)?.read_to_string(&mut xml)?;
        let text = xml_text(&xml);
        if !text.trim().is_empty() {
            output.push(format!("第 {} 页\n{}", index + 1, text));
        }
    }
    Ok(output.join("\n\n"))
}

fn extract_xlsx(bytes: &[u8]) -> Result<String> {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).context("打开 XLSX 压缩包")?;
    let shared_strings = read_shared_strings(&mut archive).unwrap_or_default();
    let mut sheet_names = Vec::new();
    for index in 0..archive.len() {
        let file = archive.by_index(index)?;
        let name = file.name().to_string();
        if name.starts_with("xl/worksheets/sheet") && name.ends_with(".xml") {
            sheet_names.push(name);
        }
    }
    sheet_names.sort_by_key(|name| numeric_suffix(name));

    let mut output = Vec::new();
    for (sheet_index, name) in sheet_names.iter().enumerate() {
        let mut xml = String::new();
        archive.by_name(name)?.read_to_string(&mut xml)?;
        let rows = xlsx_rows(&xml, &shared_strings);
        if !rows.is_empty() {
            output.push(format!("Sheet {}\n{}", sheet_index + 1, rows.join("\n")));
        }
    }
    Ok(output.join("\n\n"))
}

fn extract_pdf(name: &str, bytes: &[u8]) -> Result<String> {
    let temp_path = std::env::temp_dir().join(format!(
        "walioffice-pdf-{}-{}",
        uuid::Uuid::new_v4(),
        sanitize_temp_name(name)
    ));
    std::fs::write(&temp_path, bytes).context("写入临时 PDF")?;
    let output = Command::new("pdftotext")
        .arg("-layout")
        .arg(&temp_path)
        .arg("-")
        .output();
    let _ = std::fs::remove_file(&temp_path);

    match output {
        Ok(result) if result.status.success() => {
            Ok(String::from_utf8_lossy(&result.stdout).trim().to_string())
        }
        _ => Ok(extract_printable_text(bytes)),
    }
}

// ===== 结构化提取函数（用于前端富预览） =====

/// PPTX 结构化提取：返回幻灯片列表，每页包含元素
fn extract_pptx_structured(bytes: &[u8]) -> Result<Value> {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).context("打开 PPTX 压缩包")?;

    // 收集幻灯片文件名
    let mut slide_names = Vec::new();
    for index in 0..archive.len() {
        let file = archive.by_index(index)?;
        let name = file.name().to_string();
        if name.starts_with("ppt/slides/slide") && name.ends_with(".xml") {
            slide_names.push(name);
        }
    }
    slide_names.sort_by_key(|name| numeric_suffix(name));

    // 尝试读取演示文稿主题名
    let mut title = String::new();
    if let Ok(mut presentation_xml) = archive.by_name("ppt/presentation.xml") {
        let mut xml = String::new();
        let _ = presentation_xml.read_to_string(&mut xml);
        // 不太容易从 presentation.xml 拿标题，跳过
    }

    let mut slides = Vec::new();
    for (index, slide_name) in slide_names.iter().enumerate() {
        let mut xml = String::new();
        archive.by_name(slide_name)?.read_to_string(&mut xml)?;

        // 提取所有文本元素
        let text_elements = extract_pptx_text_elements(&xml);
        // 第一行非空文本作为标题
        let slide_title = text_elements
            .iter()
            .find(|t| !t.trim().is_empty())
            .cloned()
            .unwrap_or_else(|| format!("第 {} 页", index + 1));

        // 提取形状的填充色（简易）
        let backgrounds = extract_pptx_backgrounds(&xml);

        slides.push(json!({
            "index": index + 1,
            "title": slide_title,
            "elements": text_elements.iter().map(|t| {
                json!({ "type": "text", "text": t })
            }).collect::<Vec<_>>(),
            "background": backgrounds.first().cloned().unwrap_or_default(),
        }));
    }

    Ok(json!({
        "title": title,
        "slide_count": slides.len(),
        "slides": slides,
    }))
}

/// 从 slide XML 中提取所有 <a:t> 文本
fn extract_pptx_text_elements(xml: &str) -> Vec<String> {
    let text_re = Regex::new(r"(?s)<(?:a:)?t[^>]*>(.*?)</(?:a:)?t>").expect("text regex");
    text_re
        .captures_iter(xml)
        .filter_map(|cap| cap.get(1).map(|m| decode_xml_entities(m.as_str())))
        .filter(|s| !s.trim().is_empty())
        .collect()
}

/// 从 slide XML 中提取背景色（简易）
fn extract_pptx_backgrounds(xml: &str) -> Vec<String> {
    let bg_re = Regex::new(r#"(?:a:)?srgbClr\s+val="([0-9A-Fa-f]{6})""#).unwrap_or_else(|_| Regex::new(r"placeholder").unwrap());
    bg_re
        .captures_iter(xml)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect()
}

/// XLSX 结构化提取：返回 sheet 列表，每个 sheet 包含行列数据
fn extract_xlsx_structured(bytes: &[u8]) -> Result<Value> {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).context("打开 XLSX 压缩包")?;

    // 读取共享字符串
    let shared_strings = read_shared_strings(&mut archive).unwrap_or_default();

    // 读取 workbook.xml 获取 sheet 名称映射
    let mut sheet_display_names: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let wb_xml_text = {
        let mut text = String::new();
        if let Ok(mut f) = archive.by_name("xl/workbook.xml") {
            let _ = f.read_to_string(&mut text);
        }
        text
    };
    if !wb_xml_text.is_empty() {
        let xml = &wb_xml_text;
        let sheet_re = Regex::new(r#"<sheet[^>]+name="([^"]+)"[^>]+sheetId="[^"]+"[^>]+r:id="(rId[^"]+)"[^>]*/>"#)
            .unwrap_or_else(|_| Regex::new(r"placeholder").unwrap());
        let rel_re = Regex::new(r#"<Relationship[^>]+Id="([^"]+)"[^>]+Target="([^"]+)"[^>]*/>"#)
            .unwrap_or_else(|_| Regex::new(r"placeholder").unwrap());

        // 读取 rels
        let mut rels_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        let rels_text = {
            let mut text = String::new();
            if let Ok(mut f) = archive.by_name("xl/_rels/workbook.xml.rels") {
                let _ = f.read_to_string(&mut text);
            }
            text
        };
        for cap in rel_re.captures_iter(&rels_text) {
            if let (Some(id), Some(target)) = (cap.get(1), cap.get(2)) {
                rels_map.insert(id.as_str().to_string(), target.as_str().to_string());
            }
        }

        for cap in sheet_re.captures_iter(xml) {
            if let (Some(name), Some(r_id)) = (cap.get(1), cap.get(2)) {
                if let Some(target) = rels_map.get(r_id.as_str()) {
                    // target 形如 "worksheets/sheet1.xml"
                    let full_path = format!("xl/{}", target);
                    sheet_display_names.insert(full_path, name.as_str().to_string());
                }
            }
        }
    }

    // 收集 worksheet 文件
    let mut sheet_files = Vec::new();
    for index in 0..archive.len() {
        let file = archive.by_index(index)?;
        let name = file.name().to_string();
        if name.starts_with("xl/worksheets/sheet") && name.ends_with(".xml") {
            sheet_files.push(name);
        }
    }
    sheet_files.sort_by_key(|name| numeric_suffix(name));

    let mut sheets = Vec::new();
    for sheet_file in &sheet_files {
        let mut xml = String::new();
        archive.by_name(sheet_file)?.read_to_string(&mut xml)?;

        let display_name = sheet_display_names
            .get(sheet_file)
            .cloned()
            .unwrap_or_else(|| format!("Sheet{}", sheets.len() + 1));

        let rows = xlsx_rows_structured(&xml, &shared_strings);

        // 限制预览行数
        let max_rows = 200;
        let truncated_rows = rows.len() > max_rows;
        let preview_rows = if truncated_rows {
            rows.into_iter().take(max_rows).collect::<Vec<_>>()
        } else {
            rows
        };

        sheets.push(json!({
            "name": display_name,
            "rows": preview_rows,
            "row_count": if truncated_rows { -1 } else { preview_rows.len() as i64 },
            "truncated": truncated_rows,
        }));
    }

    Ok(json!({
        "sheet_count": sheets.len(),
        "sheets": sheets,
    }))
}

/// XLSX 行提取 — 返回 Vec<Vec<String>>（每行每列）
fn xlsx_rows_structured(xml: &str, shared_strings: &[String]) -> Vec<Vec<String>> {
    let row_re = Regex::new(r"(?s)<row[^>]*>(.*?)</row>").expect("row regex");
    let cell_re = Regex::new(r#"(?s)<c([^>]*)>(.*?)</c>"#).expect("cell regex");
    let value_re = Regex::new(r"(?s)<v[^>]*>(.*?)</v>").expect("value regex");
    let inline_re = Regex::new(r"(?s)<is[^>]*>(.*?)</is>").expect("inline string regex");

    row_re
        .captures_iter(xml)
        .filter_map(|row| {
            let cells = cell_re
                .captures_iter(row.get(1)?.as_str())
                .map(|cell| {
                    let attrs = cell.get(1).map(|item| item.as_str()).unwrap_or("");
                    let body = cell.get(2).map(|item| item.as_str()).unwrap_or("");
                    if attrs.contains(r#"t="s""#) {
                        value_re
                            .captures(body)
                            .and_then(|captures| captures.get(1))
                            .and_then(|item| item.as_str().trim().parse::<usize>().ok())
                            .and_then(|index| shared_strings.get(index).cloned())
                            .unwrap_or_default()
                    } else if attrs.contains(r#"t="inlineStr""#) {
                        inline_re
                            .captures(body)
                            .and_then(|captures| captures.get(1))
                            .map(|item| xml_text(item.as_str()))
                            .unwrap_or_default()
                    } else {
                        value_re
                            .captures(body)
                            .and_then(|captures| captures.get(1))
                            .map(|item| decode_xml_entities(item.as_str().trim()))
                            .unwrap_or_default()
                    }
                })
                .collect::<Vec<_>>();
            // 去掉全空行
            if cells.iter().all(|c| c.trim().is_empty()) {
                None
            } else {
                Some(cells)
            }
        })
        .collect()
}

/// DOCX 结构化提取：返回段落和表格的列表
fn extract_docx_structured(bytes: &[u8]) -> Result<Value> {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).context("打开 DOCX 压缩包")?;
    let mut xml = String::new();
    archive
        .by_name("word/document.xml")
        .context("读取 word/document.xml")?
        .read_to_string(&mut xml)?;

    let sections = parse_docx_body(&xml);
    let doc_title = sections
        .iter()
        .find(|s| s.get("type").and_then(|t| t.as_str()) == Some("heading"))
        .and_then(|s| s.get("text").and_then(|t| t.as_str()))
        .unwrap_or("")
        .to_string();

    Ok(json!({
        "title": doc_title,
        "section_count": sections.len(),
        "sections": sections,
    }))
}

/// 解析 docx body XML — 按段落和表格拆分
fn parse_docx_body(xml: &str) -> Vec<Value> {
    // 提取 <w:body> 内容
    let body_re = Regex::new(r"(?s)<w:body[^>]*>(.*?)</w:body>").expect("body regex");
    let body_content = body_re
        .captures(xml)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str())
        .unwrap_or(xml);

    let mut sections = Vec::new();

    // 匹配段落 <w:p> 和表格 <w:tbl>
    // 使用按顺序匹配的方式
    let p_re = Regex::new(r"(?s)<w:p[\s>].*?</w:p>").expect("para regex");
    let tbl_re = Regex::new(r"(?s)<w:tbl[\s>].*?</w:tbl>").expect("table regex");

    // 收集所有段落和表格的位置
    let mut items: Vec<(usize, &str, regex::Match)> = Vec::new();
    for m in p_re.captures_iter(body_content) {
        if let Some(full) = m.get(0) {
            items.push((full.start(), "p", full));
        }
    }
    for m in tbl_re.captures_iter(body_content) {
        if let Some(full) = m.get(0) {
            items.push((full.start(), "tbl", full));
        }
    }
    items.sort_by_key(|(pos, _, _)| *pos);

    for (_, kind, m) in &items {
        let content = m.as_str();
        if *kind == "p" {
            let para = parse_docx_paragraph(content);
            if let Some(section) = para {
                sections.push(section);
            }
        } else if *kind == "tbl" {
            let table = parse_docx_table(content);
            if let Some(section) = table {
                sections.push(section);
            }
        }
    }

    sections
}

/// 解析单个 <w:p> 段落
fn parse_docx_paragraph(xml: &str) -> Option<Value> {
    // 检查是否标题样式
    let style_re = Regex::new(r#"<w:pStyle\s+w:val="([^"]+)"#).unwrap_or_else(|_| Regex::new(r"placeholder").unwrap());
    let style = style_re
        .captures(xml)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str().to_string())
        .unwrap_or_default();

    // 提取文本
    let text = xml_text(xml);
    if text.trim().is_empty() {
        return None;
    }

    let is_heading = style.contains("Heading") || style.starts_with("heading") || style.starts_with("\u{6807}\u{9898}");
    let level: u32 = if is_heading {
        // 尝试从样式名提取级别
        let style_name = style_re
            .captures(xml)
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();
        let digits: String = style_name.chars().filter(|c| c.is_ascii_digit()).collect();
        digits.parse::<u32>().unwrap_or(1)
    } else {
        0
    };

    // 检查是否列表项
    let is_bullet = xml.contains("<w:numPr>") || style.contains("List") || style.contains("list");

    if is_heading {
        Some(json!({
            "type": "heading",
            "level": level,
            "text": text,
        }))
    } else if is_bullet {
        Some(json!({
            "type": "bullet",
            "text": text,
        }))
    } else {
        Some(json!({
            "type": "paragraph",
            "text": text,
        }))
    }
}

/// 解析 <w:tbl> 表格
fn parse_docx_table(xml: &str) -> Option<Value> {
    let row_re = Regex::new(r"(?s)<w:tr[\s>].*?</w:tr>").expect("row regex");
    let cell_re = Regex::new(r"(?s)<w:tc[\s>].*?</w:tc>").expect("cell regex");

    let mut rows = Vec::new();
    for row_match in row_re.captures_iter(xml) {
        if let Some(row_content) = row_match.get(0) {
            let mut cells = Vec::new();
            for cell_match in cell_re.captures_iter(row_content.as_str()) {
                if let Some(cell_content) = cell_match.get(0) {
                    let text = xml_text(cell_content.as_str());
                    cells.push(text.trim().to_string());
                }
            }
            if !cells.is_empty() {
                rows.push(cells);
            }
        }
    }

    if rows.is_empty() {
        return None;
    }

    // 第一行作为表头
    let headers = rows.first().cloned().unwrap_or_default();
    let body_rows: Vec<Vec<String>> = rows.into_iter().skip(1).collect();

    Some(json!({
        "type": "table",
        "headers": headers,
        "rows": body_rows,
    }))
}

/// CSV/TSV 结构化提取
fn extract_csv_structured(bytes: &[u8], delimiter: u8) -> Result<Value> {
    let text = String::from_utf8_lossy(bytes);
    let mut rows: Vec<Vec<String>> = Vec::new();

    for line in text.lines() {
        let cells: Vec<String> = if delimiter == b'\t' {
            line.split('\t').map(|s| s.to_string()).collect()
        } else {
            // 简易 CSV 解析（不处理引号内逗号）
            line.split(',').map(|s| s.trim().to_string()).collect()
        };
        if !cells.iter().all(|c| c.is_empty()) {
            rows.push(cells);
        }
    }

    let max_rows = 200;
    let truncated = rows.len() > max_rows;
    let preview_rows = if truncated {
        rows.into_iter().take(max_rows).collect::<Vec<_>>()
    } else {
        rows
    };

    Ok(json!({
        "sheet_count": 1,
        "sheets": [{
            "name": "Sheet1",
            "rows": preview_rows,
            "row_count": if truncated { -1 } else { preview_rows.len() as i64 },
            "truncated": truncated,
        }],
    }))
}

fn read_shared_strings(archive: &mut ZipArchive<Cursor<&[u8]>>) -> Result<Vec<String>> {
    let mut xml = String::new();
    archive
        .by_name("xl/sharedStrings.xml")
        .context("读取 sharedStrings.xml")?
        .read_to_string(&mut xml)?;
    let si_re = Regex::new(r"(?s)<si[^>]*>(.*?)</si>")?;
    Ok(si_re
        .captures_iter(&xml)
        .filter_map(|captures| captures.get(1).map(|item| xml_text(item.as_str())))
        .collect())
}

fn xlsx_rows(xml: &str, shared_strings: &[String]) -> Vec<String> {
    let row_re = Regex::new(r"(?s)<row[^>]*>(.*?)</row>").expect("row regex");
    let cell_re = Regex::new(r#"(?s)<c([^>]*)>(.*?)</c>"#).expect("cell regex");
    let value_re = Regex::new(r"(?s)<v[^>]*>(.*?)</v>").expect("value regex");
    let inline_re = Regex::new(r"(?s)<is[^>]*>(.*?)</is>").expect("inline string regex");

    row_re
        .captures_iter(xml)
        .filter_map(|row| {
            let cells = cell_re
                .captures_iter(row.get(1)?.as_str())
                .map(|cell| {
                    let attrs = cell.get(1).map(|item| item.as_str()).unwrap_or("");
                    let body = cell.get(2).map(|item| item.as_str()).unwrap_or("");
                    if attrs.contains(r#"t="s""#) {
                        value_re
                            .captures(body)
                            .and_then(|captures| captures.get(1))
                            .and_then(|item| item.as_str().trim().parse::<usize>().ok())
                            .and_then(|index| shared_strings.get(index).cloned())
                            .unwrap_or_default()
                    } else if attrs.contains(r#"t="inlineStr""#) {
                        inline_re
                            .captures(body)
                            .and_then(|captures| captures.get(1))
                            .map(|item| xml_text(item.as_str()))
                            .unwrap_or_default()
                    } else {
                        value_re
                            .captures(body)
                            .and_then(|captures| captures.get(1))
                            .map(|item| decode_xml_entities(item.as_str().trim()))
                            .unwrap_or_default()
                    }
                })
                .collect::<Vec<_>>();
            let line = cells.join("\t").trim().to_string();
            if line.is_empty() {
                None
            } else {
                Some(line)
            }
        })
        .collect()
}

fn xml_text(xml: &str) -> String {
    let text_re = Regex::new(r"(?s)<(?:a:)?t[^>]*>(.*?)</(?:a:)?t>").expect("xml text regex");
    let mut values = text_re
        .captures_iter(xml)
        .filter_map(|captures| {
            captures
                .get(1)
                .map(|item| decode_xml_entities(item.as_str()))
        })
        .collect::<Vec<_>>();
    if values.is_empty() {
        let tag_re = Regex::new(r"(?s)<[^>]+>").expect("tag regex");
        values.push(decode_xml_entities(&tag_re.replace_all(xml, " ")));
    }
    normalize_whitespace(&values.join("\n"))
}

fn decode_text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).trim().to_string()
}

fn extract_printable_text(bytes: &[u8]) -> String {
    let raw = String::from_utf8_lossy(bytes);
    let re = Regex::new(r"[\p{L}\p{N}\p{P}\p{Zs}]{8,}").expect("printable regex");
    normalize_whitespace(
        &re.find_iter(&raw)
            .map(|item| item.as_str())
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

fn truncate_text(text: String, parser: String) -> ExtractedFileText {
    let mut truncated = false;
    let text = if text.chars().count() > MAX_EXTRACTED_CHARS {
        truncated = true;
        text.chars().take(MAX_EXTRACTED_CHARS).collect()
    } else {
        text
    };
    ExtractedFileText {
        text: text.trim().to_string(),
        parser,
        truncated,
    }
}

fn extension(name: &str) -> String {
    std::path::Path::new(name)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_lowercase()
}

fn numeric_suffix(name: &str) -> usize {
    Regex::new(r"(\d+)")
        .expect("numeric regex")
        .captures_iter(name)
        .last()
        .and_then(|captures| captures.get(1))
        .and_then(|item| item.as_str().parse::<usize>().ok())
        .unwrap_or(usize::MAX)
}

fn sanitize_temp_name(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
        .collect::<String>()
        .chars()
        .take(80)
        .collect()
}

fn decode_xml_entities(value: &str) -> String {
    value
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

fn normalize_whitespace(value: &str) -> String {
    value
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}
