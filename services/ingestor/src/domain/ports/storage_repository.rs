use async_trait::async_trait;
use anyhow::Result;
use std::path::Path;

#[async_trait]
pub trait StorageRepository: Send + Sync {
    async fn upload_file(&self, local_path: &Path, remote_path: &str) -> Result<String>;
    async fn get_presigned_url(&self, remote_path: &str) -> Result<String>;
}
