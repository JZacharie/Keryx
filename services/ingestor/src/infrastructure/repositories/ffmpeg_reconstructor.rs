use async_trait::async_trait;
use anyhow::{Result, anyhow};
use std::path::PathBuf;
use tokio::process::Command;
use keryx_core::domain::ports::video_repository::VideoReconstructor;

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

    async fn concat_videos(&self, videos: &[PathBuf], output_path: &PathBuf) -> Result<PathBuf> {
        let mut filter_complex = String::new();
        let mut cmd = Command::new("ffmpeg");
        
        for (i, p) in videos.iter().enumerate() {
            cmd.arg("-i").arg(p);
            // We need to sync scales and pixel formats for concat to work reliably
            filter_complex.push_str(&format!("[{}:v][{}:a]", i, i));
        }
        
        filter_complex.push_str(&format!("concat=n={}:v=1:a=1[outv][outa]", videos.len()));

        let status = cmd
            .arg("-filter_complex").arg(filter_complex)
            .arg("-map").arg("[outv]")
            .arg("-map").arg("[outa]")
            .arg("-c:v").arg("libx264")
            .arg("-c:a").arg("aac")
            .arg("-y")
            .arg(output_path)
            .status()
            .await?;

        if status.success() {
            Ok(output_path.clone())
        } else {
            Err(anyhow!("FFmpeg video concat failed"))
        }
    }

    async fn concat_with_transition(&self, v1: &PathBuf, v2: &PathBuf, output_path: &PathBuf) -> Result<PathBuf> {
        // 1. Get duration of V1
        let output = Command::new("ffprobe")
            .arg("-v").arg("error")
            .arg("-show_entries").arg("format=duration")
            .arg("-of").arg("default=noprint_wrappers=1:nokey=1")
            .arg(v1)
            .output()
            .await?;
        
        let duration_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let duration: f64 = duration_str.parse().map_err(|_| anyhow!("Failed to parse duration: {}", duration_str))?;

        // 2. Complex filter logic:
        // - Pad V1 with 3s of its last frame (1s hold + 2s crossfade)
        // - Pad V1 audio with 3s of silence
        // - Pad V2 with 2s of its first frame (the fade target)
        // - Delay V2 audio by 2s to match the fade
        let offset = duration + 1.0;
        let filter = format!(
            "[0:v]tpad=stop_duration=3:stop_mode=clone[v1_ext]; \
             [1:v]tpad=start_duration=2:start_mode=clone[v2_ext]; \
             [v1_ext][v2_ext]xfade=transition=fade:duration=2:offset={:.3}[outv]; \
             [0:a]apad=pad_dur=3[a1_ext]; \
             [1:a]adelay=2000:all=1[a2_ext]; \
             [a1_ext][a2_ext]concat=n=2:v=0:a=1[outa]", 
            offset
        );

        let status = Command::new("ffmpeg")
            .arg("-i").arg(v1)
            .arg("-i").arg(v2)
            .arg("-filter_complex").arg(filter)
            .arg("-map").arg("[outv]")
            .arg("-map").arg("[outa]")
            .arg("-c:v").arg("libx264")
            .arg("-pix_fmt").arg("yuv420p")
            .arg("-c:a").arg("aac")
            .arg("-y")
            .arg(output_path)
            .status()
            .await?;

        if status.success() {
            Ok(output_path.clone())
        } else {
            Err(anyhow!("FFmpeg transition concat failed"))
        }
    }
}
