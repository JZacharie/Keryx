use async_trait::async_trait;
use anyhow::Result;
use std::path::PathBuf;

#[async_trait]
pub trait VideoDownloader: Send + Sync {
    async fn download(&self, url: &str) -> Result<(PathBuf, PathBuf, Option<PathBuf>)>; // (video_path, audio_path, subtitle_path)
}

#[async_trait]
pub trait VideoAnalyzer: Send + Sync {
    async fn detect_slides(&self, video_path: &PathBuf) -> Result<Vec<(u32, f64, PathBuf)>>; // (index, timestamp, frame_path)
}

#[async_trait]
pub trait VideoReconstructor: Send + Sync {
    /// Merges video and audio paths into a final mp4
    async fn reconstruct(&self, video_path: &PathBuf, audio_path: &PathBuf, output_path: &PathBuf) -> Result<PathBuf>;
    
    /// Concat multiple images with specific durations into a single silent video
    async fn concat_images(&self, frames: &[(PathBuf, f64)], output_path: &PathBuf) -> Result<PathBuf>;

    /// Concat multiple audio files into a single one
    async fn concat_audio(&self, segments: &[PathBuf], output_path: &PathBuf) -> Result<PathBuf>;
}
