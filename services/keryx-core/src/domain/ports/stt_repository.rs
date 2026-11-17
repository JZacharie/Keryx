use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct TranscriptionResult {
    pub segments: Vec<TranscriptionSegment>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct TranscriptionSegment {
    pub start: f64,
    pub end: f64,
    pub text: String,
}

#[async_trait]
pub trait STTRepository: Send + Sync {
    async fn transcribe(&self, audio_path: &Path) -> Result<TranscriptionResult>;
}
