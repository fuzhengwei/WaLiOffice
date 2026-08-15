use anyhow::{Context, Result};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::models::ChatAttachment;

#[derive(Debug, Deserialize)]
struct OcrResult {
    text: String,
}

fn decode_data_url(data_url: &str) -> Result<Vec<u8>> {
    let (_, payload) = data_url
        .split_once(',')
        .context("image data url is missing payload")?;
    STANDARD
        .decode(payload)
        .context("decode image data url payload")
}

fn extension_for_mime(mime_type: &str) -> &'static str {
    match mime_type {
        "image/png" => "png",
        "image/jpeg" | "image/jpg" => "jpg",
        "image/webp" => "webp",
        "image/gif" => "gif",
        "image/svg+xml" => "svg",
        _ => "png",
    }
}

fn script_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("scripts")
        .join("image_ocr.swift")
}

pub fn extract_text_from_attachment(attachment: &ChatAttachment) -> Result<Option<String>> {
    if attachment.kind != "image" {
        return Ok(None);
    }

    let data_url = match attachment
        .data_url
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        Some(value) => value,
        None => return Ok(None),
    };

    let image_bytes = decode_data_url(data_url)?;
    let temp_path = std::env::temp_dir().join(format!(
        "walioffice-ocr-{}.{}",
        uuid::Uuid::new_v4(),
        extension_for_mime(&attachment.mime_type)
    ));
    fs::write(&temp_path, image_bytes).context("write temp image for ocr")?;

    let output = Command::new("/usr/bin/swift")
        .arg(script_path())
        .arg(&temp_path)
        .output()
        .context("run swift ocr script")?;

    let _ = fs::remove_file(&temp_path);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("swift ocr failed: {stderr}");
    }

    let stdout = String::from_utf8(output.stdout).context("read swift ocr stdout")?;
    let parsed: OcrResult = serde_json::from_str(stdout.trim()).context("parse swift ocr json")?;
    let text = parsed.text.trim().to_string();
    if text.is_empty() {
        Ok(None)
    } else {
        Ok(Some(text))
    }
}
