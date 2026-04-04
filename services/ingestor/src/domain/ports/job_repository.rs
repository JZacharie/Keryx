use async_trait::async_trait;
use crate::domain::entities::job::Job;
use anyhow::Result;
use uuid::Uuid;

#[async_trait]
pub trait JobRepository: Send + Sync {
    async fn save(&self, job: &Job) -> Result<()>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Job>>;
    async fn update_status(&self, id: Uuid, status: crate::domain::entities::job::JobStatus) -> Result<()>;
}
