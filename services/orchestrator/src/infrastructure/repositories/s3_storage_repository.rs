use async_trait::async_trait;
use anyhow::Result;
use keryx_core::domain::ports::storage_repository::StorageRepository;
use aws_sdk_s3::{Client, primitives::ByteStream};
use std::path::Path;

pub struct S3StorageRepository {
    client: Client,
    bucket: String,
    endpoint: String,
}

impl S3StorageRepository {
    pub async fn new(region: &str, bucket: &str, endpoint: Option<&str>) -> Self {
        let access_key = std::env::var("S3_ACCESS_KEY_ID")
            .or_else(|_| std::env::var("AWS_ACCESS_KEY_ID"))
            .ok();
        let secret_key = std::env::var("S3_SECRET_ACCESS_KEY")
            .or_else(|_| std::env::var("AWS_SECRET_ACCESS_KEY"))
            .ok();

        let mut config_loader = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(region.to_string()));

        if let (Some(ak), Some(sk)) = (access_key, secret_key) {
            config_loader = config_loader.credentials_provider(
                aws_sdk_s3::config::Credentials::new(ak, sk, None, None, "manual")
            );
        }

        let ep_str = endpoint.unwrap_or("https://s3.amazonaws.com").to_string();

        if let Some(ep) = endpoint {
            config_loader = config_loader.endpoint_url(ep);
        }

        let config = config_loader.load().await;
        let s3_config = aws_sdk_s3::config::Builder::from(&config)
            .force_path_style(true)
            .build();
        let client = Client::from_conf(s3_config);
        Self { client, bucket: bucket.to_string(), endpoint: ep_str }
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

        let url = if self.endpoint.ends_with('/') {
            format!("{}{}/{}", self.endpoint, self.bucket, remote_path)
        } else {
            format!("{}/{}/{}", self.endpoint, self.bucket, remote_path)
        };
        Ok(url)
    }

    async fn upload_buffer(&self, buffer: Vec<u8>, remote_path: &str, content_type: &str) -> Result<String> {
        let body = ByteStream::from(buffer);
        self.client.put_object()
            .bucket(&self.bucket)
            .key(remote_path)
            .content_type(content_type)
            .body(body)
            .send()
            .await?;

        let url = if self.endpoint.ends_with('/') {
            format!("{}{}/{}", self.endpoint, self.bucket, remote_path)
        } else {
            format!("{}/{}/{}", self.endpoint, self.bucket, remote_path)
        };
        Ok(url)
    }

    async fn get_file_content(&self, remote_path: &str) -> Result<Vec<u8>> {
        let output = self.client.get_object()
            .bucket(&self.bucket)
            .key(remote_path)
            .send()
            .await?;

        let data = output.body.collect().await?.to_vec();
        Ok(data)
    }

    async fn get_presigned_url(&self, _remote_path: &str) -> Result<String> {
        // Implementation for presigned URL if needed for frontend access
        Ok("".to_string())
    }
}
