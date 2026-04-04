use async_trait::async_trait;
use anyhow::{Result, Context};
use crate::domain::ports::job_repository::JobRepository;
use crate::domain::entities::job::{Job, JobStatus};
use redis::AsyncCommands;
use uuid::Uuid;

pub struct RedisJobRepository {
    client: redis::Client,
}

impl RedisJobRepository {
    pub fn new(url: &str) -> Result<Self> {
        let client = redis::Client::open(url)?;
        Ok(Self { client })
    }
}

#[async_trait]
impl JobRepository for RedisJobRepository {
    async fn save(&self, job: &Job) -> Result<()> {
        let mut conn = self.client.get_async_connection().await?;
        let json = serde_json::to_string(job)?;
        let _: () = conn.set(job.id.to_string(), json).await?;
        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Job>> {
        let mut conn = self.client.get_async_connection().await?;
        let json: Option<String> = conn.get(id.to_string()).await?;
        match json {
            Some(s) => Ok(Some(serde_json::from_str(&s)?)),
            None => Ok(None),
        }
    }

    async fn update_status(&self, id: Uuid, status: JobStatus) -> Result<()> {
        let mut job = self.find_by_id(id).await?.context("Job not found")?;
        job.status = status;
        self.save(&job).await
    }
}
