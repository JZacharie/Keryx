use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

use crate::domain::entities::job::{Job, JobStatus};
use crate::domain::ports::job_repository::JobRepository;
use crate::infrastructure::docker::{ContainerManager, WorkerSpec};

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
    container_manager: Arc<ContainerManager>,
}

impl CreateJobUseCase {
    pub fn new(
        repository: Arc<dyn JobRepository>,
        container_manager: Arc<ContainerManager>,
    ) -> Self {
        Self {
            repository,
            container_manager,
        }
    }

    pub async fn execute(&self, input: CreateJobInput) -> anyhow::Result<CreateJobOutput> {
        let job_id = uuid::Uuid::new_v4().to_string();
        let pipeline_start = Instant::now();

        let mut job = Job {
            id: job_id.clone(),
            status: JobStatus::Ingested,
        };
        self.repository.save(&job).await?;

        tracing::info!(
            "=================================================================================="
        );
        tracing::info!(
            "PIPELINE START | job={} | media={} | lang={}",
            job_id,
            input.media_url,
            input.language
        );
        tracing::info!(
            "=================================================================================="
        );

        let base_env = self.build_env();

        // Phase 1: Extractor (downloads video, no GPU)
        job.status = JobStatus::Downloading;
        self.repository.save(&job).await?;
        self.run_worker("keryx-extractor", "Downloading", false, &base_env).await?;

        // Phase 2: Voice Extractor (Whisper STT, GPU)
        job.status = JobStatus::Transcribing;
        self.repository.save(&job).await?;
        self.run_worker("keryx-voice-extractor", "Transcribing", true, &base_env).await?;

        // Phase 3: Video Composer (slide detection, GPU for encoding)
        job.status = JobStatus::Analyzing;
        self.repository.save(&job).await?;
        self.run_worker("keryx-video-composer", "Analyzing", true, &base_env).await?;

        // Phase 3B: Dewatermark (GPU)
        self.run_worker("keryx-dewatermark", "Dewatermark", true, &base_env).await?;

        // Phase 4: Texts Translation (no GPU)
        job.status = JobStatus::Translating;
        self.repository.save(&job).await?;
        self.run_worker("keryx-texts-translation", "Translating", false, &base_env).await?;

        // Phase 4B: Voices Cloner (GPU)
        job.status = JobStatus::CloningVoice;
        self.repository.save(&job).await?;
        self.run_worker("keryx-voices-cloner", "CloningVoice", true, &base_env).await?;

        // Phase 5: Final Video Composer (GPU)
        job.status = JobStatus::Composing;
        self.repository.save(&job).await?;
        self.run_worker("keryx-video-composer", "Composing", true, &base_env).await?;

        job.status = JobStatus::Completed;
        self.repository.save(&job).await?;

        let elapsed = pipeline_start.elapsed();
        tracing::info!(
            "=================================================================================="
        );
        tracing::info!(
            "PIPELINE COMPLETE | job={} | duration={:.1}s",
            job_id,
            elapsed.as_secs_f64()
        );
        tracing::info!(
            "=================================================================================="
        );
        Ok(CreateJobOutput { job_id })
    }

    fn build_env(&self) -> Vec<String> {
        let mut env = Vec::new();
        for key in &[
            "S3_ENDPOINT",
            "S3_BUCKET",
            "S3_ACCESS_KEY_ID",
            "S3_SECRET_ACCESS_KEY",
            "API_KEY",
            "REDIS_URL",
            "HF_TOKEN",
        ] {
            if let Ok(val) = std::env::var(key) {
                env.push(format!("{}={}", key, val));
            }
        }
        if let Ok(val) = std::env::var("LOG_LEVEL") {
            env.push(format!("LOG_LEVEL={}", val));
        } else {
            env.push("LOG_LEVEL=INFO".to_string());
        }
        env.push("PYTHONUNBUFFERED=1".to_string());
        env.push("PORT=8000".to_string());
        env
    }

    async fn run_worker(
        &self,
        image: &str,
        phase: &str,
        needs_gpu: bool,
        base_env: &[String],
    ) -> anyhow::Result<()> {
        let container_name = format!("{}-{}", image, uuid::Uuid::new_v4());
        let worker_start = Instant::now();

        let spec = WorkerSpec {
            image: image.to_string(),
            container_name: container_name.clone(),
            env: base_env.to_vec(),
            needs_gpu,
            port: 8000,
            volumes: vec![],
        };

        tracing::info!("[{}] >>> Starting phase: {} (image={}, gpu={})", container_name, phase, image, needs_gpu);
        match self.container_manager.start_worker(&spec).await {
            Ok(handle) => {
                let start_elapsed = worker_start.elapsed();
                tracing::info!("[{}] <<< {} ready in {:.1}s", container_name, phase, start_elapsed.as_secs_f64());

                self.container_manager.stop_worker(&handle).await?;

                let total_elapsed = worker_start.elapsed();
                tracing::info!("[{}] === Phase {} completed in {:.1}s ===", container_name, phase, total_elapsed.as_secs_f64());
                Ok(())
            }
            Err(e) => {
                tracing::error!("[{}] Phase {} FAILED: {}", container_name, phase, e);
                Err(e)
            }
        }
    }
}
