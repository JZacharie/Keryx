use async_trait::async_trait;
use anyhow::{Result, anyhow};
use std::path::PathBuf;
use std::process::Command;
use crate::domain::ports::video_repository::VideoDownloader;

pub struct YtDlpRepository {
    download_dir: PathBuf,
}

impl YtDlpRepository {
    pub fn new(download_dir: PathBuf) -> Self {
        Self { download_dir }
    }
}

#[async_trait]
impl VideoDownloader for YtDlpRepository {
    async fn download(&self, url: &str) -> Result<(PathBuf, PathBuf, Option<PathBuf>)> {
        let video_id = Uuid::new_v4().to_string();
        let video_path = self.download_dir.join(format!("{}.mp4", video_id));
        let audio_path = self.download_dir.join(format!("{}.wav", video_id));

        // Download video + audio + subtitles
        let status = Command::new("yt-dlp")
            .arg("-v")
            .arg("-f")
            .arg("bestvideo[ext=mp4]+bestaudio[ext=m4a]/best[ext=mp4]/best")
            .arg("--merge-output-format")
            .arg("mp4")
            .arg("--external-downloader")
            .arg("aria2c")
            .arg("--js-runtimes")
            .arg("node")
            .arg("--write-subs")
            .arg("--write-auto-subs")
            .arg("--sub-format")
            .arg("vtt")
            .arg("--no-playlist")
            .arg("--no-check-certificates")
            .arg("--geo-bypass")
            .arg("-o")
            .arg(&video_path)
            .arg(url)
            .status()?;

        if !status.success() {
            return Err(anyhow!("yt-dlp failed with status: {}", status));
        }

        // Check for subtitle file
        let mut sub_path = None;
        let entries = std::fs::read_dir(&self.download_dir)?;
        for entry in entries {
            let entry = entry?;
            let name = entry.file_name().into_string().unwrap_or_default();
            if name.starts_with(&video_id) && name.ends_with(".vtt") {
                sub_path = Some(entry.path());
                break;
            }
        }

        // Extract audio to wav
        let status = Command::new("ffmpeg")
            .arg("-i")
            .arg(&video_path)
            .arg("-vn")
            .arg("-acodec")
            .arg("pcm_s16le")
            .arg("-ar")
            .arg("16000")
            .arg("-ac")
            .arg("1")
            .arg("-y")
            .arg(&audio_path)
            .status()?;

        if !status.success() {
            return Err(anyhow!("ffmpeg audio extraction failed with status: {}", status));
        }

        Ok((video_path, audio_path, sub_path))
    }
}

use uuid::Uuid;
