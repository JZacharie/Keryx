use async_trait::async_trait;
use anyhow::Result;
use std::path::Path;

#[async_trait]
pub trait StorageRepository: Send + Sync {
    async fn upload_file(&self, local_path: &Path, remote_path: &str) -> Result<String>;
    async fn upload_buffer(&self, buffer: Vec<u8>, remote_path: &str, content_type: &str) -> Result<String>;
    async fn get_file_content(&self, remote_path: &str) -> Result<Vec<u8>>;
    async fn get_presigned_url(&self, remote_path: &str) -> Result<String>;
}
