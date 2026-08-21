// PPT 幻灯片渲染（2-1 节 stub，完整实现在 3-3 节）
//
// 底层手写 OOXML（ZIP 包结构），生成 HTML 幻灯片由前端渲染。
// 完整实现包含：slide 尺寸（SLIDE_W=12_192_000 EMU）、主题、元素序列化、ZIP 打包。

use std::path::Path;
use std::io::Write;

// ─── 渲染函数（2-1 stub） ────────────────────────────────────────────────────

/// 渲染 PPT 项目（2-1 节暂不实现，输出空文件）
pub fn render_pptx(
    project: &crate::models::PptProject,
    path: &Path,
) -> anyhow::Result<()> {
    // 完整实现在 3-3 节：
    // 1. 手写 OOXML：presentation.xml / slideMasters / slides / _rels 等
    // 2. SLIDE_W=12_192_000 EMU，SLIDE_H=6_858_000 EMU，INCH_EMU=914_400
    // 3. 生成 HTML preview 由前端 SlideRenderer 渲染
    tracing::warn!(
        "render_pptx stub: {} ({} slides), real impl in 3-3",
        project.title,
        project.slides.len()
    );
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::File::create(path)?.write_all(b"")?;
    Ok(())
}
