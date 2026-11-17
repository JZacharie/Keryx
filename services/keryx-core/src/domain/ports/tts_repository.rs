use anyhow::Result;
use async_trait::async_trait;
use std::path::{Path, PathBuf};

#[async_trait]
pub trait TTSRepository: Send + Sync {
    async fn generate(&self, text: &str, language: &str, target_path: &Path) -> Result<PathBuf>;
}
