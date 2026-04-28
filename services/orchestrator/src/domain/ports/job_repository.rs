use async_trait::async_trait;
use crate::domain::entities::job::Job;

#[async_trait]
pub trait JobRepository: Send + Sync {
    async fn save(&self, job: &Job) -> anyhow::Result<()>;
    async fn get_by_id(&self, job_id: &str) -> anyhow::Result<Option<Job>>;
    async fn exists(&self, job_id: &str) -> anyhow::Result<bool>;
}
