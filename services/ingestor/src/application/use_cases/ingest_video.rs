use std::sync::Arc;
use uuid::Uuid;
use anyhow::{Result, Context};
use crate::domain::ports::job_repository::JobRepository;
use crate::domain::ports::storage_repository::StorageRepository;
use crate::domain::ports::video_repository::{VideoDownloader, VideoAnalyzer};
use crate::domain::entities::job::{Job, JobStatus, SlideAsset};

pub struct IngestVideoUseCase {
    job_repo: Arc<dyn JobRepository>,
    storage_repo: Arc<dyn StorageRepository>,
    downloader: Arc<dyn VideoDownloader>,
    analyzer: Arc<dyn VideoAnalyzer>,
}

impl IngestVideoUseCase {
    pub fn new(
        job_repo: Arc<dyn JobRepository>,
        storage_repo: Arc<dyn StorageRepository>,
        downloader: Arc<dyn VideoDownloader>,
        analyzer: Arc<dyn VideoAnalyzer>,
    ) -> Self {
        Self { job_repo, storage_repo, downloader, analyzer }
    }

    pub fn get_job_repo(&self) -> Arc<dyn JobRepository> {
        self.job_repo.clone()
    }

    pub async fn execute(&self, job_id: Uuid) -> Result<()> {
        let mut job = self.job_repo.find_by_id(job_id).await?
            .context("Job not found")?;

        // 1. Download
        self.job_repo.update_status(job_id, JobStatus::Downloading).await?;
        let (video_path, audio_path) = self.downloader.download(&job.source_url).await?;

        // 2. Upload audio
        let audio_remote = format!("jobs/{}/raw/audio.wav", job_id);
        self.storage_repo.upload_file(&audio_path, &audio_remote).await?;

        // 3. Analyze
        self.job_repo.update_status(job_id, JobStatus::Analyzing).await?;
        let slides = self.analyzer.detect_slides(&video_path).await?;

        // 4. Upload frames and build job asset map
        let mut slide_assets = Vec::new();
        for (index, timestamp, frame_path) in slides {
            let frame_remote = format!("jobs/{}/raw/frame_{}.png", job_id, index);
            let frame_url = self.storage_repo.upload_file(&frame_path, &frame_remote).await?;
            
            slide_assets.push(SlideAsset {
                slide_index: index,
                original_frame: frame_url,
                timestamp,
                translations: std::collections::HashMap::new(),
            });
        }

        job.assets_map = slide_assets;
        job.status = JobStatus::Transcribing;
        self.job_repo.save(&job).await?;

        Ok(())
    }
}
