use crate::domain::entities::job::{Job, JobStatus};
use crate::domain::ports::job_repository::JobRepository;
use std::sync::Arc;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct CreateJobInput {
    pub media_url: String,
    pub language: String,
}

#[derive(Debug, Serialize)]
pub struct CreateJobOutput {
    pub job_id: String,
}

pub struct CreateJobUseCase {
    repository: Arc<dyn JobRepository>,
}

impl CreateJobUseCase {
    pub fn new(repository: Arc<dyn JobRepository>) -> Self {
        Self { repository }
    }

    pub async fn execute(&self, input: CreateJobInput) -> anyhow::Result<CreateJobOutput> {
        let job_id = Uuid::new_v4().to_string();
        
        // In a real scenario, we would add fields to Job entity
        let job = Job {
            id: job_id.clone(),
            status: JobStatus::Ingested,
        };

        // Logging the input for now since Job entity is minimal
        tracing::info!("Creating job for media: {} ({})", input.media_url, input.language);

        self.repository.save(&job).await?;

        Ok(CreateJobOutput { job_id })
    }
}
