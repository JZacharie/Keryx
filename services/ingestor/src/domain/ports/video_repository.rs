use async_trait::async_trait;
use anyhow::Result;
use std::path::PathBuf;

#[async_trait]
pub trait VideoDownloader: Send + Sync {
    async fn download(&self, url: &str) -> Result<(PathBuf, PathBuf)>; // (video_path, audio_path)
}

#[async_trait]
pub trait VideoAnalyzer: Send + Sync {
    async fn detect_slides(&self, video_path: &PathBuf) -> Result<Vec<(u32, f64, PathBuf)>>; // (index, timestamp, frame_path)
}
