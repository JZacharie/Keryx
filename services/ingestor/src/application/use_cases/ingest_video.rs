use std::sync::Arc;
use uuid::Uuid;
use anyhow::{Result, Context};
use crate::domain::ports::job_repository::JobRepository;
use crate::domain::ports::storage_repository::StorageRepository;
use crate::domain::ports::video_repository::{VideoDownloader, VideoAnalyzer};
use crate::domain::ports::stt_repository::STTRepository;
use crate::domain::ports::translator_repository::TranslatorRepository;
use crate::domain::entities::job::{JobStatus, SlideAsset, TranslationAsset};

pub struct IngestVideoUseCase {
    job_repo: Arc<dyn JobRepository>,
    storage_repo: Arc<dyn StorageRepository>,
    downloader: Arc<dyn VideoDownloader>,
    analyzer: Arc<dyn VideoAnalyzer>,
    stt_repo: Arc<dyn STTRepository>,
    translator: Arc<dyn TranslatorRepository>,
}

impl IngestVideoUseCase {
    pub fn new(
        job_repo: Arc<dyn JobRepository>,
        storage_repo: Arc<dyn StorageRepository>,
        downloader: Arc<dyn VideoDownloader>,
        analyzer: Arc<dyn VideoAnalyzer>,
        stt_repo: Arc<dyn STTRepository>,
        translator: Arc<dyn TranslatorRepository>,
    ) -> Self {
        Self { job_repo, storage_repo, downloader, analyzer, stt_repo, translator }
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

        // 5. Transcribe
        let transcription = self.stt_repo.transcribe(&audio_path).await?;

        // Match transcription segments to slides
        let slide_offsets: Vec<(f64, Option<f64>)> = job.assets_map.iter().enumerate().map(|(i, s)| {
            let next = job.assets_map.get(i+1).map(|ns| ns.timestamp);
            (s.timestamp, next)
        }).collect();

        for (i, slide) in job.assets_map.iter_mut().enumerate() {
            let (start, next_start) = slide_offsets[i];
            let slide_text: Vec<String> = transcription.segments.iter()
                .filter(|s| s.start >= start && next_start.map_or(true, |ns: f64| s.end <= ns))
                .map(|s| s.text.clone())
                .collect();

            let original_text = slide_text.join(" ");

            // 6. Translate
            for lang in &job.target_langs {
                let translated = self.translator.translate(&original_text, lang).await?;
                slide.translations.insert(lang.clone(), TranslationAsset {
                    text: translated,
                    styled_image: None,
                    audio: None,
                    duration: 0.0,
                });
            }
        }

        job.status = JobStatus::GeneratingVisuals;
        self.job_repo.save(&job).await?;

        Ok(())
    }
}
