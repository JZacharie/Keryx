use anyhow::Result;
use async_trait::async_trait;
use std::path::{Path, PathBuf};

#[async_trait]
pub trait VoiceClonerRepository: Send + Sync {
    async fn voice_clone(
        &self,
        text: &str,
        language: &str,
        speaker_wav: Option<&str>,
        target_path: &Path,
    ) -> Result<PathBuf>;
}
