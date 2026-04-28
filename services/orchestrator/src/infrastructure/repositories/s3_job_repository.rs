use async_trait::async_trait;
use aws_sdk_s3::Client;
use crate::domain::entities::job::Job;
use crate::domain::ports::job_repository::JobRepository;

pub struct S3JobRepository {
    client: Client,
    bucket: String,
}

impl S3JobRepository {
    pub fn new(client: Client, bucket: String) -> Self {
        Self { client, bucket }
    }
}

#[async_trait]
impl JobRepository for S3JobRepository {
    async fn save(&self, job: &Job) -> anyhow::Result<()> {
        let key = format!("jobs/{}.json", job.id);
        let body = serde_json::to_vec(job)?;

        self.client.put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(body.into())
            .send()
            .await?;

        Ok(())
    }

    async fn get_by_id(&self, job_id: &str) -> anyhow::Result<Option<Job>> {
        let key = format!("jobs/{}.json", job_id);

        let result = self.client.get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await;

        match result {
            Ok(output) => {
                let bytes = output.body.collect().await?.into_bytes();
                let job = serde_json::from_slice(&bytes)?;
                Ok(Some(job))
            },
            Err(e) => {
                let service_error = e.into_service_error();
                if service_error.is_no_such_key() {
                    Ok(None)
                } else {
                    Err(anyhow::anyhow!("S3 error: {}", service_error))
                }
            }
        }
    }

    async fn exists(&self, job_id: &str) -> anyhow::Result<bool> {
        let key = format!("jobs/{}.json", job_id);

        let result = self.client.head_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await;

        match result {
            Ok(_) => Ok(true),
            Err(e) => {
                let service_error = e.into_service_error();
                // HeadObject returns 404 for missing keys
                if service_error.is_not_found() {
                    Ok(false)
                } else {
                    Err(anyhow::anyhow!("S3 error: {}", service_error))
                }
            }
        }
    }
}
