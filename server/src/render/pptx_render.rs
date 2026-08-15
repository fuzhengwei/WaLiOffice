use crate::models::{PptProject, Slide, SlideElement};
use anyhow::Result;
use std::fs::File;
use std::io::Write;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

const SLIDE_W: i64 = 12_192_000;
const SLIDE_H: i64 = 6_858_000;
const INCH_EMU: f64 = 914_400.0;

pub fn render_pptx(project: &PptProject, output_path: &std::path::Path) -> Result<()> {
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let file = File::create(output_path)?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    write_part(
        &mut zip,
        options,
        "[Content_Types].xml",
        &content_types(project.slides.len()),
    )?;
    write_part(&mut zip, options, "_rels/.rels", ROOT_RELS)?;
    write_part(&mut zip, options, "docProps/app.xml", &app_props(project))?;
    write_part(&mut zip, options, "docProps/core.xml", &core_props(project))?;
    write_part(
        &mut zip,
        options,
        "ppt/presentation.xml",
        &presentation(project.slides.len()),
    )?;
    write_part(
        &mut zip,
        options,
        "ppt/_rels/presentation.xml.rels",
        &presentation_rels(project.slides.len()),
    )?;
    write_part(&mut zip, options, "ppt/theme/theme1.xml", THEME)?;
    write_part(
        &mut zip,
        options,
        "ppt/slideMasters/slideMaster1.xml",
        SLIDE_MASTER,
    )?;
    write_part(
        &mut zip,
        options,
        "ppt/slideMasters/_rels/slideMaster1.xml.rels",
        SLIDE_MASTER_RELS,
    )?;
    write_part(
        &mut zip,
        options,
        "ppt/slideLayouts/slideLayout1.xml",
        SLIDE_LAYOUT,
    )?;
    write_part(
        &mut zip,
        options,
        "ppt/slideLayouts/_rels/slideLayout1.xml.rels",
        SLIDE_LAYOUT_RELS,
    )?;

    for (index, slide) in project.slides.iter().enumerate() {
        write_part(
            &mut zip,
            options,
            &format!("ppt/slides/slide{}.xml", index + 1),
            &slide_xml(slide),
        )?;
        write_part(
            &mut zip,
            options,
            &format!("ppt/slides/_rels/slide{}.xml.rels", index + 1),
            SLIDE_RELS,
        )?;
    }

    zip.finish()?;
    Ok(())
}

fn write_part(
    zip: &mut ZipWriter<File>,
    options: SimpleFileOptions,
    path: &str,
    content: &str,
) -> Result<()> {
    zip.start_file(path, options)?;
    zip.write_all(content.as_bytes())?;
    Ok(())
}

fn content_types(slide_count: usize) -> String {
    let mut overrides = String::new();
    for index in 1..=slide_count.max(1) {
        overrides.push_str(&format!(
            r#"<Override PartName="/ppt/slides/slide{index}.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>"#
        ));
    }
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="xml" ContentType="application/xml"/>
<Override PartName="/docProps/app.xml" ContentType="application/vnd.openxmlformats-officedocument.extended-properties+xml"/>
<Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/>
<Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
<Override PartName="/ppt/slideMasters/slideMaster1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml"/>
<Override PartName="/ppt/slideLayouts/slideLayout1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml"/>
<Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
{overrides}
</Types>"#
    )
}

fn presentation(slide_count: usize) -> String {
    let mut slide_ids = String::new();
    for index in 1..=slide_count.max(1) {
        slide_ids.push_str(&format!(
            r#"<p:sldId id="{}" r:id="rId{}"/>"#,
            255 + index,
            index + 1
        ));
    }
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:sldMasterIdLst><p:sldMasterId id="2147483648" r:id="rId1"/></p:sldMasterIdLst>
<p:sldIdLst>{slide_ids}</p:sldIdLst>
<p:sldSz cx="{SLIDE_W}" cy="{SLIDE_H}" type="screen16x9"/>
<p:notesSz cx="6858000" cy="9144000"/>
<p:defaultTextStyle/>
</p:presentation>"#
    )
}

fn presentation_rels(slide_count: usize) -> String {
    let mut rels = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>"#,
    );
    for index in 1..=slide_count.max(1) {
        rels.push_str(&format!(
            r#"<Relationship Id="rId{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide{index}.xml"/>"#,
            index + 1
        ));
    }
    rels.push_str("</Relationships>");
    rels
}

fn app_props(project: &PptProject) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties" xmlns:vt="http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes">
<Application>WaLiOffice</Application><PresentationFormat>On-screen Show (16:9)</PresentationFormat><Slides>{}</Slides><Company>WaLiOffice</Company>
</Properties>"#,
        project.slides.len()
    )
}

fn core_props(project: &PptProject) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/" xmlns:dcmitype="http://purl.org/dc/dcmitype/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
<dc:title>{}</dc:title><dc:creator>WaLiOffice</dc:creator><cp:lastModifiedBy>WaLiOffice</cp:lastModifiedBy><dcterms:created xsi:type="dcterms:W3CDTF">{}</dcterms:created><dcterms:modified xsi:type="dcterms:W3CDTF">{}</dcterms:modified>
</cp:coreProperties>"#,
        escape_xml(&project.title),
        escape_xml(&project.created_at),
        escape_xml(&project.updated_at)
    )
}

fn slide_xml(slide: &Slide) -> String {
    let background = normalize_color(&slide.background, "FFFFFF");
    let mut shapes = String::new();
    for (index, element) in slide.elements.iter().enumerate() {
        shapes.push_str(&element_xml(index + 2, element, &background));
    }
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:cSld><p:bg><p:bgPr><a:solidFill><a:srgbClr val="{background}"/></a:solidFill><a:effectLst/></p:bgPr></p:bg><p:spTree>
<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="{SLIDE_W}" cy="{SLIDE_H}"/><a:chOff x="0" y="0"/><a:chExt cx="{SLIDE_W}" cy="{SLIDE_H}"/></a:xfrm></p:grpSpPr>
{shapes}
</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr>
</p:sld>"#
    )
}

fn element_xml(id: usize, element: &SlideElement, slide_background: &str) -> String {
    match element.element_type.as_str() {
        "text" => text_shape_xml(id, element, slide_background),
        "shape" => basic_shape_xml(id, element),
        "table" => table_text_xml(id, element, slide_background),
        "image" => image_placeholder_xml(id, element, slide_background),
        _ => text_shape_xml(id, element, slide_background),
    }
}

fn basic_shape_xml(id: usize, element: &SlideElement) -> String {
    let (x, y, w, h) = bounds(element);
    let fill = normalize_color(element.fill.as_deref().unwrap_or("E2E8F0"), "E2E8F0");
    let preset = match element.shape.as_deref().unwrap_or("rect") {
        "roundRect" => "roundRect",
        "ellipse" => "ellipse",
        _ => "rect",
    };
    format!(
        r#"<p:sp><p:nvSpPr><p:cNvPr id="{id}" name="Shape {id}"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="{x}" y="{y}"/><a:ext cx="{w}" cy="{h}"/></a:xfrm><a:prstGeom prst="{preset}"><a:avLst/></a:prstGeom><a:solidFill><a:srgbClr val="{fill}"/></a:solidFill><a:ln><a:noFill/></a:ln></p:spPr><p:txBody><a:bodyPr/><a:lstStyle/><a:p/></p:txBody></p:sp>"#
    )
}

fn text_shape_xml(id: usize, element: &SlideElement, slide_background: &str) -> String {
    let (x, y, w, h) = bounds(element);
    let fill = element
        .fill
        .as_deref()
        .map(|value| normalize_color(value, slide_background));
    let fill_xml = fill
        .map(|value| format!(r#"<a:solidFill><a:srgbClr val="{value}"/></a:solidFill>"#))
        .unwrap_or_else(|| "<a:noFill/>".to_string());
    let color = normalize_color(element.color.as_deref().unwrap_or("111827"), "111827");
    let size = ((element.font_size.unwrap_or(18.0) * 100.0).round() as i64).max(800);
    let bold = if element.bold.unwrap_or(false) {
        " b=\"1\""
    } else {
        ""
    };
    let italic = if element.italic.unwrap_or(false) {
        " i=\"1\""
    } else {
        ""
    };
    let align = match element.align.as_deref().unwrap_or("left") {
        "center" => "ctr",
        "right" => "r",
        _ => "l",
    };
    let valign = match element.valign.as_deref().unwrap_or("top") {
        "middle" => "mid",
        "bottom" => "b",
        _ => "t",
    };
    let text = element.text.as_deref().unwrap_or("");
    let paragraphs = text
        .split('\n')
        .map(|line| {
            format!(
                r#"<a:p><a:pPr algn="{align}"/><a:r><a:rPr lang="zh-CN" sz="{size}"{bold}{italic}><a:solidFill><a:srgbClr val="{color}"/></a:solidFill></a:rPr><a:t>{}</a:t></a:r></a:p>"#,
                escape_xml(line)
            )
        })
        .collect::<String>();
    format!(
        r#"<p:sp><p:nvSpPr><p:cNvPr id="{id}" name="Text {id}"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="{x}" y="{y}"/><a:ext cx="{w}" cy="{h}"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom>{fill_xml}<a:ln><a:noFill/></a:ln></p:spPr><p:txBody><a:bodyPr wrap="square" anchor="{valign}" lIns="91440" tIns="45720" rIns="91440" bIns="45720"/><a:lstStyle/>{paragraphs}</p:txBody></p:sp>"#
    )
}

fn table_text_xml(id: usize, element: &SlideElement, slide_background: &str) -> String {
    let mut lines = Vec::new();
    if let Some(rows) = &element.table_data {
        for row in rows {
            lines.push(row.join("  |  "));
        }
    }
    let mut text_element = element.clone();
    text_element.element_type = "text".into();
    text_element.text = Some(lines.join("\n"));
    text_element.font_size = Some(element.font_size.unwrap_or(12.0));
    text_shape_xml(id, &text_element, slide_background)
}

fn image_placeholder_xml(id: usize, element: &SlideElement, slide_background: &str) -> String {
    let mut text_element = element.clone();
    text_element.element_type = "text".into();
    text_element.text = Some(
        element
            .path
            .as_deref()
            .map(|path| format!("图片占位：{path}"))
            .unwrap_or_else(|| "图片占位".into()),
    );
    text_element.fill = Some("F1F5F9".into());
    text_element.color = Some("64748B".into());
    text_element.font_size = Some(12.0);
    text_shape_xml(id, &text_element, slide_background)
}

fn bounds(element: &SlideElement) -> (i64, i64, i64, i64) {
    (
        (element.x * INCH_EMU).round() as i64,
        (element.y * INCH_EMU).round() as i64,
        (element.w * INCH_EMU).round().max(1.0) as i64,
        (element.h * INCH_EMU).round().max(1.0) as i64,
    )
}

fn normalize_color(color: &str, fallback: &str) -> String {
    let value = color.trim().trim_start_matches('#');
    if value.len() == 6 && value.chars().all(|item| item.is_ascii_hexdigit()) {
        value.to_uppercase()
    } else {
        fallback.to_string()
    }
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

const ROOT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/>
<Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties" Target="docProps/app.xml"/>
</Relationships>"#;

const SLIDE_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/>
</Relationships>"#;

const SLIDE_MASTER_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/>
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="../theme/theme1.xml"/>
</Relationships>"#;

const SLIDE_LAYOUT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="../slideMasters/slideMaster1.xml"/>
</Relationships>"#;

const SLIDE_LAYOUT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" type="blank" preserve="1">
<p:cSld name="Blank"><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr></p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sldLayout>"#;

const SLIDE_MASTER: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:cSld><p:bg><p:bgPr><a:solidFill><a:srgbClr val="FFFFFF"/></a:solidFill><a:effectLst/></p:bgPr></p:bg><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr></p:spTree></p:cSld>
<p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
<p:sldLayoutIdLst><p:sldLayoutId id="2147483649" r:id="rId1"/></p:sldLayoutIdLst><p:txStyles><p:titleStyle/><p:bodyStyle/><p:otherStyle/></p:txStyles></p:sldMaster>"#;

const THEME: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="WaLiOffice"><a:themeElements><a:clrScheme name="Office"><a:dk1><a:srgbClr val="111827"/></a:dk1><a:lt1><a:srgbClr val="FFFFFF"/></a:lt1><a:dk2><a:srgbClr val="1F2937"/></a:dk2><a:lt2><a:srgbClr val="F8FAFC"/></a:lt2><a:accent1><a:srgbClr val="2563EB"/></a:accent1><a:accent2><a:srgbClr val="7C3AED"/></a:accent2><a:accent3><a:srgbClr val="059669"/></a:accent3><a:accent4><a:srgbClr val="F97316"/></a:accent4><a:accent5><a:srgbClr val="0EA5E9"/></a:accent5><a:accent6><a:srgbClr val="EC4899"/></a:accent6><a:hlink><a:srgbClr val="2563EB"/></a:hlink><a:folHlink><a:srgbClr val="7C3AED"/></a:folHlink></a:clrScheme><a:fontScheme name="Office"><a:majorFont><a:latin typeface="Aptos Display"/><a:ea typeface="Microsoft YaHei"/><a:cs typeface="Arial"/></a:majorFont><a:minorFont><a:latin typeface="Aptos"/><a:ea typeface="Microsoft YaHei"/><a:cs typeface="Arial"/></a:minorFont></a:fontScheme><a:fmtScheme name="Office"><a:fillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:fillStyleLst><a:lnStyleLst><a:ln w="6350" cap="flat" cmpd="sng" algn="ctr"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:prstDash val="solid"/></a:ln></a:lnStyleLst><a:effectStyleLst><a:effectStyle><a:effectLst/></a:effectStyle></a:effectStyleLst><a:bgFillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:bgFillStyleLst></a:fmtScheme></a:themeElements><a:objectDefaults/><a:extraClrSchemeLst/></a:theme>"#;
