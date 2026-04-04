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
        let output_pattern = self.output_dir.join("frame_%04d.jpg");

        // ffmpeg -i video.mp4 -vf "select='gt(scene,0.05)',showinfo" -vsync vfr -q:v 2 out%04d.jpg
        let output = Command::new("ffmpeg")
            .arg("-i")
            .arg(video_path)
            .arg("-vf")
            .arg("select='gt(scene,0.05)',showinfo")
            .arg("-vsync")
            .arg("vfr")
            .arg("-q:v")
            .arg("2")
            .arg("-y")
            .arg(&output_pattern)
            .output()?; // We capture output to get stderr

        if !output.status.success() {
            return Err(anyhow!("ffmpeg scene detection failed"));
        }

        let stderr = String::from_utf8_lossy(&output.stderr);
        let mut slides = Vec::new();
        let mut frame_count = 1;

        for line in stderr.lines() {
            if line.contains("showinfo") && line.contains("pts_time") {
                // Parse pts_time:xxxx from the line
                if let Some(pts_idx) = line.find("pts_time:") {
                    let part = &line[pts_idx + 9..];
                    if let Some(space_idx) = part.find(' ') {
                        if let Ok(ts) = part[..space_idx].parse::<f64>() {
                            let frame_path = self.output_dir.join(format!("frame_{:04}.jpg", frame_count));
                            if frame_path.exists() {
                                slides.push((frame_count as u32, ts, frame_path));
                                frame_count += 1;
                            }
                        }
                    }
                }
            }
        }

        Ok(slides)
    }
}
