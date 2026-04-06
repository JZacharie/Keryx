use async_trait::async_trait;
use anyhow::{Result, anyhow};
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use tokio::io::{BufReader, AsyncBufReadExt};
use crate::domain::ports::video_repository::VideoAnalyzer;

pub struct FfmpegAnalyzer {
    output_dir: PathBuf,
}

impl FfmpegAnalyzer {
    pub fn new(output_dir: PathBuf) -> Self {
        Self { output_dir }
    }
}

#[async_trait]
impl VideoAnalyzer for FfmpegAnalyzer {
    async fn detect_slides(&self, video_path: &PathBuf) -> Result<Vec<(u32, f64, PathBuf)>> {
        let output_pattern = self.output_dir.join("frame_%04d.jpg");

        // Use tokio::process::Command for non-blocking and streamed output
        let mut child = Command::new("ffmpeg")
            .arg("-i")
            .arg(video_path)
            .arg("-vf")
            .arg("select='gt(scene,0.2)',showinfo")
            .arg("-vsync")
            .arg("vfr")
            .arg("-q:v")
            .arg("2")
            .arg("-y")
            .arg(&output_pattern)
            .stderr(Stdio::piped())
            .stdout(Stdio::null())
            .spawn()
            .map_err(|e| anyhow!("Failed to spawn ffmpeg: {}", e))?;

        let stderr = child.stderr.take().ok_or_else(|| anyhow!("Failed to capture ffmpeg stderr"))?;
        let mut reader = BufReader::new(stderr).lines();

        let mut slides = Vec::new();
        let mut frame_count = 1;

        while let Some(line) = reader.next_line().await? {
            tracing::debug!("[FFmpeg] {}", line);
            if line.contains("showinfo") && line.contains("pts_time") {
                if let Some(pts_idx) = line.find("pts_time:") {
                    let part = &line[pts_idx + 9..];
                    if let Some(space_idx) = part.find(' ') {
                        if let Ok(ts) = part[..space_idx].parse::<f64>() {
                            let frame_name = format!("frame_{:04}.jpg", frame_count);
                            let frame_path = self.output_dir.join(&frame_name);
                            // We don't check existence yet as ffmpeg might be writing it
                            slides.push((frame_count as u32, ts, frame_path));
                            frame_count += 1;
                        }
                    }
                }
            }
        }

        let status = child.wait().await?;
        if !status.success() {
            return Err(anyhow!("ffmpeg scene detection failed with code {}", status.code().unwrap_or(-1)));
        }

        // Final check: confirm files actually exist on disk
        let mut final_slides = Vec::new();
        for (idx, ts, path) in slides {
            if path.exists() {
                final_slides.push((idx, ts, path));
            }
        }

        Ok(final_slides)
    }
}
