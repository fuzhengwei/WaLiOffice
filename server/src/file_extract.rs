use anyhow::{Context, Result};
use regex::Regex;
use serde::Serialize;
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

pub fn extract_text_from_bytes(name: &str, mime_type: &str, bytes: &[u8]) -> ExtractedFileText {
    let result = extract_text(name, mime_type, bytes);
    let (text, parser) = match result {
        Ok((text, parser)) => (text, parser),
        Err(err) => (format!("文件解析失败：{err}"), "error".to_string()),
    };
    truncate_text(text, parser)
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
