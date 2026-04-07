use async_trait::async_trait;
use anyhow::{Result, anyhow};
use std::path::PathBuf;
use tokio::process::Command;
use crate::domain::ports::video_repository::VideoReconstructor;

pub struct FfmpegReconstructor;

impl FfmpegReconstructor {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl VideoReconstructor for FfmpegReconstructor {
    async fn reconstruct(&self, video_path: &PathBuf, audio_path: &PathBuf, output_path: &PathBuf) -> Result<PathBuf> {
        let status = Command::new("ffmpeg")
            .arg("-i").arg(video_path)
            .arg("-i").arg(audio_path)
            .arg("-c:v").arg("copy")
            .arg("-c:a").arg("aac")
            .arg("-map").arg("0:v:0")
            .arg("-map").arg("1:a:0")
            .arg("-shortest")
            .arg("-y")
            .arg(output_path)
            .status()
            .await?;

        if status.success() {
            Ok(output_path.clone())
        } else {
            Err(anyhow!("FFmpeg reconstruction failed"))
        }
    }

    async fn concat_images(&self, frames: &[(PathBuf, f64)], output_path: &PathBuf) -> Result<PathBuf> {
        let mut concat_content = String::new();
        for (path, duration) in frames {
            concat_content.push_str(&format!("file '{}'\nduration {:.3}\n", path.to_string_lossy(), duration));
        }
        
        // FFmpeg concat requirement: the last file must be repeated or have a duration
        if let Some((path, _)) = frames.last() {
             concat_content.push_str(&format!("file '{}'\n", path.to_string_lossy()));
        }

        let tmp_concat = output_path.with_extension("txt");
        tokio::fs::write(&tmp_concat, concat_content).await?;

        let status = Command::new("ffmpeg")
            .arg("-f").arg("concat")
            .arg("-safe").arg("0")
            .arg("-i").arg(&tmp_concat)
            .arg("-c:v").arg("libx264")
            .arg("-pix_fmt").arg("yuv420p")
            .arg("-r").arg("24")
            .arg("-y")
            .arg(output_path)
            .status()
            .await?;

        let _ = tokio::fs::remove_file(tmp_concat).await;

        if status.success() {
            Ok(output_path.clone())
        } else {
            Err(anyhow!("FFmpeg image concat failed"))
        }
    }

    async fn concat_audio(&self, segments: &[PathBuf], output_path: &PathBuf) -> Result<PathBuf> {
        let mut filter_complex = String::new();
        let mut cmd = Command::new("ffmpeg");
        
        for (i, p) in segments.iter().enumerate() {
            cmd.arg("-i").arg(p);
            filter_complex.push_str(&format!("[{}:a]", i));
        }
        
        filter_complex.push_str(&format!("concat=n={}:v=0:a=1[outa]", segments.len()));

        let status = cmd
            .arg("-filter_complex").arg(filter_complex)
            .arg("-map").arg("[outa]")
            .arg("-y")
            .arg(output_path)
            .status()
            .await?;

        if status.success() {
            Ok(output_path.clone())
        } else {
            Err(anyhow!("FFmpeg audio concat failed"))
        }
    }
}
