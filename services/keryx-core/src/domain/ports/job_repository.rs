use async_trait::async_trait;
use crate::domain::entities::job::Job;
use anyhow::Result;
use uuid::Uuid;

#[async_trait]
pub trait JobRepository: Send + Sync {
    async fn save(&self, job: &Job) -> Result<()>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Job>>;
    async fn update_status(&self, id: Uuid, status: crate::domain::entities::job::JobStatus) -> Result<()>;
    async fn update_progress(&self, id: Uuid, progress: f32) -> Result<()>;
    /// Append a log line for a job (stored separately in Redis for streaming)
    async fn append_log(&self, id: Uuid, message: &str) -> Result<()>;
    /// Retrieve all log lines for a job
    async fn get_logs(&self, id: Uuid) -> Result<Vec<String>>;
    /// List all jobs (limited)
    async fn list(&self, limit: usize) -> Result<Vec<Job>>;
}
