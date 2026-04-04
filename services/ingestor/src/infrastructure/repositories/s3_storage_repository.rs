use async_trait::async_trait;
use anyhow::Result;
use crate::domain::ports::storage_repository::StorageRepository;
use aws_sdk_s3::{Client, primitives::ByteStream};
use std::path::Path;

pub struct S3StorageRepository {
    client: Client,
    bucket: String,
}

impl S3StorageRepository {
    pub async fn new(region: &str, bucket: &str, endpoint: Option<&str>) -> Self {
        let mut config_loader = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(region.to_string()));

        if let Some(ep) = endpoint {
            config_loader = config_loader.endpoint_url(ep);
        }

        let config = config_loader.load().await;
        let client = Client::new(&config);
        Self { client, bucket: bucket.to_string() }
    }
}

#[async_trait]
impl StorageRepository for S3StorageRepository {
    async fn upload_file(&self, local_path: &Path, remote_path: &str) -> Result<String> {
        let body = ByteStream::from_path(local_path).await?;
        self.client.put_object()
            .bucket(&self.bucket)
            .key(remote_path)
            .body(body)
            .send()
            .await?;

        Ok(format!("s3://{}/{}", self.bucket, remote_path))
    }

    async fn get_presigned_url(&self, _remote_path: &str) -> Result<String> {
        // Implementation for presigned URL if needed for frontend access
        Ok("".to_string())
    }
}
