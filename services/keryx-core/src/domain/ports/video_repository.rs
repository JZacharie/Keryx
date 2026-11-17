use anyhow::Result;
use async_trait::async_trait;
use std::path::{Path, PathBuf};

#[async_trait]
pub trait VideoDownloader: Send + Sync {
    async fn download(&self, url: &str) -> Result<(PathBuf, PathBuf, Option<PathBuf>)>; // (video_path, audio_path, subtitle_path)
}

#[async_trait]
pub trait VideoAnalyzer: Send + Sync {
    async fn detect_slides(&self, video_path: &Path) -> Result<Vec<(u32, f64, PathBuf)>>; // (index, timestamp, frame_path)
}

#[async_trait]
pub trait VideoReconstructor: Send + Sync {
    /// Merges video and audio paths into a final mp4
    async fn reconstruct(
        &self,
        video_path: &Path,
        audio_path: &Path,
        output_path: &Path,
    ) -> Result<PathBuf>;

    /// Concat multiple images with specific durations into a single silent video
    async fn concat_images(&self, frames: &[(PathBuf, f64)], output_path: &Path)
        -> Result<PathBuf>;

    /// Concat multiple audio files into a single one
    async fn concat_audio(&self, segments: &[PathBuf], output_path: &Path) -> Result<PathBuf>;

    /// Concat multiple video files into a single one
    async fn concat_videos(&self, videos: &[PathBuf], output_path: &Path) -> Result<PathBuf>;

    /// Concat two videos with a specialized transition:
    /// 1s hold on last frame of V1 + 2s crossfade to first frame of V2
    async fn concat_with_transition(
        &self,
        v1: &Path,
        v2: &Path,
        output_path: &Path,
    ) -> Result<PathBuf>;
}
