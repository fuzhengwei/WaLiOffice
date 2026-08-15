pub mod docx_render;
pub mod pptx_render;
pub mod xlsx_render;

use std::path::PathBuf;

pub fn output_path(filename: &str) -> PathBuf {
    let dir = &crate::config::config().render_output_dir;
    std::fs::create_dir_all(dir).ok();
    PathBuf::from(dir).join(filename)
}
