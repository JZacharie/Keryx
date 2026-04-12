use async_trait::async_trait;
use anyhow::{Result, Context};
use keryx_core::domain::ports::job_repository::JobRepository;
use keryx_core::domain::entities::job::{Job, JobStatus};
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
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let json = serde_json::to_string(job)?;
        let _: () = conn.set(job.id.to_string(), json).await?;
        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Job>> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
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

    async fn append_log(&self, id: Uuid, message: &str) -> Result<()> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let key = format!("log:{}", id);
        let timestamp = chrono::Utc::now().format("%H:%M:%S").to_string();
        let entry = format!("[{}] {}", timestamp, message);
        let _: () = conn.rpush(&key, &entry).await?;
        // TTL 24h pour le nettoyage automatique
        let _: () = conn.expire(&key, 86400).await?;
        Ok(())
    }

    async fn get_logs(&self, id: Uuid) -> Result<Vec<String>> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let key = format!("log:{}", id);
        let logs: Vec<String> = conn.lrange(&key, 0, -1).await?;
        Ok(logs)
    }
}
