use async_trait::async_trait;
use anyhow::Result;
use std::path::PathBuf;

#[async_trait]
pub trait TTSRepository: Send + Sync {
    async fn generate(&self, text: &str, language: &str, target_path: &PathBuf) -> Result<PathBuf>;
}
