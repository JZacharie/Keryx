use async_trait::async_trait;
use anyhow::Result;
use std::path::PathBuf;

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
    async fn transcribe(&self, audio_path: &PathBuf) -> Result<TranscriptionResult>;
}
