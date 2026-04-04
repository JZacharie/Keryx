use async_trait::async_trait;
use anyhow::{Result, anyhow};
use std::path::PathBuf;
use std::process::Command;
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
        // Use ffmpeg to detect scene changes and extract frames
        // ffmpeg -i video.mp4 -vf "select='gt(scene,0.03)',setpts=N/FRAME_RATE/TB" -vsync vfr out%03d.png

        let output_pattern = self.output_dir.join("frame_%03d.png");

        let status = Command::new("ffmpeg")
            .arg("-i")
            .arg(video_path)
            .arg("-vf")
            .arg("select='gt(scene,0.03)',setpts=N/FRAME_RATE/TB")
            .arg("-vsync")
            .arg("vfr")
            .arg(&output_pattern)
            .status()?;

        if !status.success() {
            return Err(anyhow!("ffmpeg scene detection failed"));
        }

        // We also need timestamps. This is harder with just a single command.
        // For simplicity in this demo, we'll assume frames are extracted and we'll just list them.
        let mut slides = Vec::new();
        let mut index = 0;

        // Let's assume we find frames in the directory
        let entries = std::fs::read_dir(&self.output_dir)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("png") {
                // In a real implementation, we'd parse timestamps from ffmpeg output
                slides.push((index, index as f64 * 10.0, path));
                index += 1;
            }
        }

        Ok(slides)
    }
}
