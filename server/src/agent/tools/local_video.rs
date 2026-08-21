use anyhow::{anyhow, Context, Result};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use tokio::process::Command;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct LocalVideoOutput {
    pub public_url: String,
    pub file_path: PathBuf,
    pub seconds: f32,
    pub size: String,
    pub frame_count: u32,
}

#[derive(Debug)]
struct Canvas {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl Canvas {
    fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; (width * height * 3) as usize],
        }
    }

    fn set_pixel(&mut self, column: i32, row: i32, color: [u8; 3]) {
        if column < 0 || row < 0 || column >= self.width as i32 || row >= self.height as i32 {
            return;
        }
        let offset = ((row as u32 * self.width + column as u32) * 3) as usize;
        self.pixels[offset..offset + 3].copy_from_slice(&color);
    }

    fn blend_pixel(&mut self, column: i32, row: i32, color: [u8; 3], alpha: f32) {
        if column < 0 || row < 0 || column >= self.width as i32 || row >= self.height as i32 {
            return;
        }
        let clamped_alpha = alpha.clamp(0.0, 1.0);
        let offset = ((row as u32 * self.width + column as u32) * 3) as usize;
        for channel in 0..3 {
            let base = self.pixels[offset + channel] as f32;
            let blended = base * (1.0 - clamped_alpha) + color[channel] as f32 * clamped_alpha;
            self.pixels[offset + channel] = blended.round().clamp(0.0, 255.0) as u8;
        }
    }

    fn fill_gradient(&mut self, top: [u8; 3], bottom: [u8; 3]) {
        let height_scale = (self.height.saturating_sub(1)).max(1) as f32;
        for row in 0..self.height {
            let progress = row as f32 / height_scale;
            let color = mix_color(top, bottom, progress);
            for column in 0..self.width {
                self.set_pixel(column as i32, row as i32, color);
            }
        }
    }

    fn fill_rect(&mut self, left: i32, top: i32, right: i32, bottom: i32, color: [u8; 3]) {
        for row in top.max(0)..bottom.min(self.height as i32) {
            for column in left.max(0)..right.min(self.width as i32) {
                self.set_pixel(column, row, color);
            }
        }
    }

    fn draw_circle(&mut self, center_column: f32, center_row: f32, radius: f32, color: [u8; 3]) {
        let left = (center_column - radius).floor() as i32;
        let right = (center_column + radius).ceil() as i32;
        let top = (center_row - radius).floor() as i32;
        let bottom = (center_row + radius).ceil() as i32;
        let radius_squared = radius * radius;
        for row in top..=bottom {
            for column in left..=right {
                let delta_column = column as f32 - center_column;
                let delta_row = row as f32 - center_row;
                if delta_column * delta_column + delta_row * delta_row <= radius_squared {
                    self.set_pixel(column, row, color);
                }
            }
        }
    }

    fn draw_ellipse(
        &mut self,
        center_column: f32,
        center_row: f32,
        radius_column: f32,
        radius_row: f32,
        color: [u8; 3],
    ) {
        let left = (center_column - radius_column).floor() as i32;
        let right = (center_column + radius_column).ceil() as i32;
        let top = (center_row - radius_row).floor() as i32;
        let bottom = (center_row + radius_row).ceil() as i32;
        for row in top..=bottom {
            for column in left..=right {
                let normalized_column = (column as f32 - center_column) / radius_column.max(1.0);
                let normalized_row = (row as f32 - center_row) / radius_row.max(1.0);
                if normalized_column * normalized_column + normalized_row * normalized_row <= 1.0 {
                    self.set_pixel(column, row, color);
                }
            }
        }
    }

    fn draw_line(
        &mut self,
        start_column: f32,
        start_row: f32,
        end_column: f32,
        end_row: f32,
        width: f32,
        color: [u8; 3],
    ) {
        let steps = ((end_column - start_column)
            .abs()
            .max((end_row - start_row).abs())
            * 1.5)
            .max(1.0) as u32;
        for step in 0..=steps {
            let progress = step as f32 / steps as f32;
            let column = start_column + (end_column - start_column) * progress;
            let row = start_row + (end_row - start_row) * progress;
            self.draw_circle(column, row, width, color);
        }
    }

    fn draw_triangle(&mut self, points: [(f32, f32); 3], color: [u8; 3]) {
        let min_column = points
            .iter()
            .map(|point| point.0)
            .fold(f32::INFINITY, f32::min)
            .floor() as i32;
        let max_column = points
            .iter()
            .map(|point| point.0)
            .fold(f32::NEG_INFINITY, f32::max)
            .ceil() as i32;
        let min_row = points
            .iter()
            .map(|point| point.1)
            .fold(f32::INFINITY, f32::min)
            .floor() as i32;
        let max_row = points
            .iter()
            .map(|point| point.1)
            .fold(f32::NEG_INFINITY, f32::max)
            .ceil() as i32;
        for row in min_row..=max_row {
            for column in min_column..=max_column {
                if point_in_triangle((column as f32, row as f32), points) {
                    self.set_pixel(column, row, color);
                }
            }
        }
    }

    fn write_ppm(&self, path: &Path) -> Result<()> {
        let file =
            File::create(path).with_context(|| format!("创建视频帧失败：{}", path.display()))?;
        let mut writer = BufWriter::new(file);
        write!(writer, "P6\n{} {}\n255\n", self.width, self.height)?;
        writer.write_all(&self.pixels)?;
        Ok(())
    }
}

pub async fn generate_local_video(
    topic: &str,
    aspect_ratio: &str,
    requested_width: u32,
    requested_height: u32,
    requested_frame_count: u32,
    frame_rate: u32,
) -> Result<LocalVideoOutput> {
    ensure_ffmpeg_available().await?;

    let render_dir = PathBuf::from(&crate::config::config().render_output_dir);
    let video_id = Uuid::new_v4().to_string();
    let video_dir = render_dir.join("videos").join(&video_id);
    let frames_dir = video_dir.join("frames");
    fs::create_dir_all(&frames_dir)?;

    let (width, height) = local_dimensions(aspect_ratio, requested_width, requested_height);
    let frame_count = requested_frame_count.clamp(frame_rate * 3, frame_rate * 8);
    let seconds = frame_count as f32 / frame_rate as f32;

    for frame_index in 0..frame_count {
        let mut canvas = Canvas::new(width, height);
        let progress = frame_index as f32 / frame_count.saturating_sub(1).max(1) as f32;
        draw_scene(&mut canvas, topic, progress);
        let frame_path = frames_dir.join(format!("frame_{frame_index:04}.ppm"));
        canvas.write_ppm(&frame_path)?;
    }

    let output_path = video_dir.join("video.mp4");
    encode_video(&frames_dir, &output_path, frame_rate, seconds).await?;
    let _ = fs::remove_dir_all(&frames_dir);

    Ok(LocalVideoOutput {
        public_url: format!("/outputs/videos/{video_id}/video.mp4"),
        file_path: output_path,
        seconds,
        size: format!("{}x{}", width, height),
        frame_count,
    })
}

async fn ensure_ffmpeg_available() -> Result<()> {
    let output = Command::new("ffmpeg").arg("-version").output().await;
    match output {
        Ok(result) if result.status.success() => Ok(()),
        Ok(_) => Err(anyhow!("本地 ffmpeg 不可用，无法生成兜底视频")),
        Err(err) => Err(anyhow!(
            "未找到 ffmpeg，请先安装 ffmpeg 后再生成本地视频：{err}"
        )),
    }
}

async fn encode_video(
    frames_dir: &Path,
    output_path: &Path,
    frame_rate: u32,
    seconds: f32,
) -> Result<()> {
    let input_pattern = frames_dir.join("frame_%04d.ppm");
    let fade_out_start = (seconds - 0.8).max(0.0).to_string();
    let duration = format!("{seconds:.3}");
    let status = Command::new("ffmpeg")
        .arg("-y")
        .arg("-framerate")
        .arg(frame_rate.to_string())
        .arg("-i")
        .arg(input_pattern)
        .arg("-f")
        .arg("lavfi")
        .arg("-i")
        .arg(format!(
            "sine=frequency=523:duration={duration}:sample_rate=44100"
        ))
        .arg("-filter:a")
        .arg(format!(
            "volume=0.035,afade=t=in:ss=0:d=0.4,afade=t=out:st={fade_out_start}:d=0.8"
        ))
        .arg("-c:v")
        .arg("libx264")
        .arg("-pix_fmt")
        .arg("yuv420p")
        .arg("-movflags")
        .arg("+faststart")
        .arg("-shortest")
        .arg(output_path)
        .status()
        .await?;

    if status.success() {
        Ok(())
    } else {
        Err(anyhow!("ffmpeg 视频编码失败，退出码：{}", status))
    }
}

fn local_dimensions(aspect_ratio: &str, requested_width: u32, requested_height: u32) -> (u32, u32) {
    let (fallback_width, fallback_height) = match aspect_ratio {
        "9:16" => (540, 960),
        "1:1" => (720, 720),
        "4:3" => (800, 600),
        "3:4" => (600, 800),
        _ => (960, 540),
    };
    let width = requested_width.min(fallback_width).max(320);
    let height = requested_height.min(fallback_height).max(320);
    (width - width % 2, height - height % 2)
}

fn draw_scene(canvas: &mut Canvas, topic: &str, progress: f32) {
    let normalized_topic = topic.to_lowercase();
    if normalized_topic.contains('猫')
        || normalized_topic.contains("cat")
        || normalized_topic.contains("kitten")
    {
        draw_cat_scene(canvas, progress);
    } else {
        draw_motion_scene(canvas, progress);
    }
}

fn draw_cat_scene(canvas: &mut Canvas, progress: f32) {
    let width = canvas.width as f32;
    let height = canvas.height as f32;
    canvas.fill_gradient([255, 243, 220], [255, 207, 175]);
    canvas.fill_rect(
        0,
        (height * 0.74) as i32,
        canvas.width as i32,
        canvas.height as i32,
        [238, 190, 144],
    );

    let orbit = progress * std::f32::consts::TAU;
    let sun_column = width * 0.82 + orbit.sin() * width * 0.025;
    let sun_row = height * 0.19 + orbit.cos() * height * 0.02;
    canvas.draw_circle(
        sun_column,
        sun_row,
        width.min(height) * 0.08,
        [255, 204, 88],
    );

    for sparkle_index in 0..12 {
        let sparkle_progress = (progress + sparkle_index as f32 * 0.071) % 1.0;
        let column = width * (0.08 + sparkle_progress * 0.84);
        let row = height * (0.22 + ((sparkle_index * 37) % 45) as f32 / 100.0);
        canvas.draw_circle(
            column,
            row + (orbit + sparkle_index as f32).sin() * 12.0,
            3.0,
            [255, 172, 82],
        );
    }

    let cat_column = width * 0.5 + orbit.sin() * width * 0.03;
    let cat_row = height * 0.58 + (orbit * 2.0).sin() * height * 0.01;
    let unit = width.min(height) / 9.5;
    canvas.draw_ellipse(
        cat_column,
        cat_row + unit * 1.55,
        unit * 1.7,
        unit * 1.2,
        [226, 135, 78],
    );
    canvas.draw_ellipse(
        cat_column,
        cat_row,
        unit * 1.35,
        unit * 1.18,
        [241, 157, 92],
    );
    canvas.draw_triangle(
        [
            (cat_column - unit * 0.9, cat_row - unit * 0.72),
            (cat_column - unit * 0.52, cat_row - unit * 1.75),
            (cat_column - unit * 0.18, cat_row - unit * 0.78),
        ],
        [241, 157, 92],
    );
    canvas.draw_triangle(
        [
            (cat_column + unit * 0.9, cat_row - unit * 0.72),
            (cat_column + unit * 0.52, cat_row - unit * 1.75),
            (cat_column + unit * 0.18, cat_row - unit * 0.78),
        ],
        [241, 157, 92],
    );
    canvas.draw_ellipse(
        cat_column - unit * 0.48,
        cat_row - unit * 0.1,
        unit * 0.16,
        unit * 0.24,
        [48, 43, 42],
    );
    canvas.draw_ellipse(
        cat_column + unit * 0.48,
        cat_row - unit * 0.1,
        unit * 0.16,
        unit * 0.24,
        [48, 43, 42],
    );
    canvas.draw_triangle(
        [
            (cat_column - unit * 0.1, cat_row + unit * 0.28),
            (cat_column + unit * 0.1, cat_row + unit * 0.28),
            (cat_column, cat_row + unit * 0.43),
        ],
        [219, 98, 111],
    );
    canvas.draw_line(
        cat_column - unit * 0.25,
        cat_row + unit * 0.38,
        cat_column - unit * 1.15,
        cat_row + unit * 0.23,
        1.8,
        [76, 55, 50],
    );
    canvas.draw_line(
        cat_column + unit * 0.25,
        cat_row + unit * 0.38,
        cat_column + unit * 1.15,
        cat_row + unit * 0.23,
        1.8,
        [76, 55, 50],
    );
    canvas.draw_line(
        cat_column - unit * 0.2,
        cat_row + unit * 0.52,
        cat_column - unit * 1.05,
        cat_row + unit * 0.58,
        1.8,
        [76, 55, 50],
    );
    canvas.draw_line(
        cat_column + unit * 0.2,
        cat_row + unit * 0.52,
        cat_column + unit * 1.05,
        cat_row + unit * 0.58,
        1.8,
        [76, 55, 50],
    );

    let tail_angle = -0.35 + orbit.sin() * 0.34;
    let tail_base_column = cat_column + unit * 1.35;
    let tail_base_row = cat_row + unit * 1.28;
    canvas.draw_line(
        tail_base_column,
        tail_base_row,
        tail_base_column + tail_angle.cos() * unit * 1.7,
        tail_base_row - unit * 1.2 + tail_angle.sin() * unit,
        unit * 0.22,
        [226, 135, 78],
    );
    canvas.draw_ellipse(
        cat_column - unit * 0.55,
        cat_row + unit * 2.45,
        unit * 0.45,
        unit * 0.24,
        [255, 218, 177],
    );
    canvas.draw_ellipse(
        cat_column + unit * 0.55,
        cat_row + unit * 2.45,
        unit * 0.45,
        unit * 0.24,
        [255, 218, 177],
    );

    let butterfly_column = width * (0.18 + 0.64 * ease(progress));
    let butterfly_row = height * 0.36 + (orbit * 2.4).sin() * height * 0.06;
    canvas.draw_ellipse(
        butterfly_column - unit * 0.22,
        butterfly_row,
        unit * 0.24,
        unit * 0.14,
        [110, 190, 245],
    );
    canvas.draw_ellipse(
        butterfly_column + unit * 0.22,
        butterfly_row,
        unit * 0.24,
        unit * 0.14,
        [110, 190, 245],
    );
    canvas.draw_circle(
        butterfly_column,
        butterfly_row,
        unit * 0.08,
        [245, 103, 139],
    );
}

fn draw_motion_scene(canvas: &mut Canvas, progress: f32) {
    let width = canvas.width as f32;
    let height = canvas.height as f32;
    let orbit = progress * std::f32::consts::TAU;
    canvas.fill_gradient([236, 242, 255], [255, 238, 224]);
    canvas.fill_rect(
        0,
        (height * 0.72) as i32,
        canvas.width as i32,
        canvas.height as i32,
        [232, 225, 213],
    );

    for band_index in 0..8 {
        let band_progress = (progress + band_index as f32 * 0.13) % 1.0;
        let column = width * band_progress;
        let row = height * (0.18 + band_index as f32 * 0.065);
        canvas.draw_line(
            column - width * 0.16,
            row,
            column + width * 0.1,
            row + height * 0.09,
            5.0,
            [98, 115, 255],
        );
    }

    let center_column = width * 0.5;
    let center_row = height * 0.48;
    canvas.draw_ellipse(
        center_column,
        center_row,
        width * 0.18,
        height * 0.17,
        [255, 255, 255],
    );
    canvas.draw_ellipse(
        center_column,
        center_row,
        width * 0.15,
        height * 0.13,
        [255, 181, 96],
    );
    canvas.draw_circle(
        center_column + orbit.cos() * width * 0.17,
        center_row + orbit.sin() * height * 0.13,
        width.min(height) * 0.055,
        [53, 76, 210],
    );
    canvas.draw_circle(
        center_column - orbit.sin() * width * 0.16,
        center_row + orbit.cos() * height * 0.12,
        width.min(height) * 0.04,
        [20, 184, 166],
    );

    for dot_index in 0..18 {
        let angle = orbit + dot_index as f32 * 0.55;
        let radius = width.min(height) * (0.22 + (dot_index % 4) as f32 * 0.035);
        let column = center_column + angle.cos() * radius;
        let row = center_row + angle.sin() * radius * 0.55;
        canvas.blend_pixel(column as i32, row as i32, [255, 255, 255], 0.9);
        canvas.draw_circle(column, row, 4.0, [255, 255, 255]);
    }
}

fn mix_color(start: [u8; 3], end: [u8; 3], progress: f32) -> [u8; 3] {
    [
        mix_channel(start[0], end[0], progress),
        mix_channel(start[1], end[1], progress),
        mix_channel(start[2], end[2], progress),
    ]
}

fn mix_channel(start: u8, end: u8, progress: f32) -> u8 {
    (start as f32 + (end as f32 - start as f32) * progress.clamp(0.0, 1.0)).round() as u8
}

fn ease(progress: f32) -> f32 {
    0.5 - 0.5 * (std::f32::consts::PI * progress.clamp(0.0, 1.0)).cos()
}

fn point_in_triangle(point: (f32, f32), triangle: [(f32, f32); 3]) -> bool {
    let area = triangle_area(triangle[0], triangle[1], triangle[2]);
    let area_one = triangle_area(point, triangle[1], triangle[2]);
    let area_two = triangle_area(triangle[0], point, triangle[2]);
    let area_three = triangle_area(triangle[0], triangle[1], point);
    (area - (area_one + area_two + area_three)).abs() < 0.5
}

fn triangle_area(first: (f32, f32), second: (f32, f32), third: (f32, f32)) -> f32 {
    ((first.0 * (second.1 - third.1)
        + second.0 * (third.1 - first.1)
        + third.0 * (first.1 - second.1))
        .abs())
        / 2.0
}
