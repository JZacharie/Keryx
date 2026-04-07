use std::sync::Arc;
use uuid::Uuid;
use anyhow::{Result, Context};
use crate::domain::ports::job_repository::JobRepository;
use crate::domain::ports::storage_repository::StorageRepository;
use crate::domain::ports::video_repository::{VideoDownloader, VideoAnalyzer};
use crate::domain::ports::stt_repository::STTRepository;
use crate::domain::ports::translator_repository::TranslatorRepository;
use crate::domain::ports::stylizer_repository::StylizerRepository;
use crate::domain::ports::pptx_repository::{PptxRepository, SlideInput};
use crate::domain::ports::scaling_repository::ScalingRepository;
use crate::domain::entities::job::{JobStatus, SlideAsset, TranslationAsset};

pub struct IngestVideoUseCase {
    job_repo: Arc<dyn JobRepository>,
    storage_repo: Arc<dyn StorageRepository>,
    downloader: Arc<dyn VideoDownloader>,
    analyzer: Arc<dyn VideoAnalyzer>,
    stt_repo: Arc<dyn STTRepository>,
    translator: Arc<dyn TranslatorRepository>,
    stylizer: Arc<dyn StylizerRepository>,
    pptx_repo: Arc<dyn PptxRepository>,
    scaling_repo: Arc<dyn ScalingRepository>,
}

impl IngestVideoUseCase {
    pub fn new(
        job_repo: Arc<dyn JobRepository>,
        storage_repo: Arc<dyn StorageRepository>,
        downloader: Arc<dyn VideoDownloader>,
        analyzer: Arc<dyn VideoAnalyzer>,
        stt_repo: Arc<dyn STTRepository>,
        translator: Arc<dyn TranslatorRepository>,
        stylizer: Arc<dyn StylizerRepository>,
        pptx_repo: Arc<dyn PptxRepository>,
        scaling_repo: Arc<dyn ScalingRepository>,
    ) -> Self {
        Self { job_repo, storage_repo, downloader, analyzer, stt_repo, translator, stylizer, pptx_repo, scaling_repo }
    }

    pub fn get_job_repo(&self) -> Arc<dyn JobRepository> {
        self.job_repo.clone()
    }

    pub async fn execute(&self, job_id: Uuid) -> Result<()> {
        let mut job = self.job_repo.find_by_id(job_id).await?
            .context("Job not found")?;

        tracing::info!("[Job {}] Starting ingestion for {}", job_id, job.source_url);

        // 1. Download
        tracing::info!("[Job {}] Phase 1: Downloading video and audio...", job_id);
        self.job_repo.update_status(job_id, JobStatus::Downloading).await?;
        let (video_path, audio_path, subtitle_path) = self.downloader.download(&job.source_url).await?;

        // 2. Upload raw assets
        tracing::info!("[Job {}] Phase 2: Uploading raw assets to storage...", job_id);
        let audio_remote = format!("jobs/{}/raw/audio.wav", job_id);
        self.storage_repo.upload_file(&audio_path, &audio_remote).await?;

        let video_remote = format!("jobs/{}/raw/video.mp4", job_id);
        self.storage_repo.upload_file(&video_path, &video_remote).await?;

        if let Some(sub_path) = &subtitle_path {
            let sub_remote = format!("jobs/{}/raw/subtitles.vtt", job_id);
            self.storage_repo.upload_file(sub_path, &sub_remote).await?;
        }

        // 3. Analyze
        tracing::info!("[Job {}] Phase 3: Analyzing video for slide transitions...", job_id);
        self.job_repo.update_status(job_id, JobStatus::Analyzing).await?;
        let slides = self.analyzer.detect_slides(&video_path).await?;
        tracing::info!("[Job {}] Analysis complete. Detected {} slides.", job_id, slides.len());

        // 4. Upload frames, clean watermarks and build job asset map
        self.job_repo.update_status(job_id, JobStatus::GeneratingVisuals).await?; // Use GeneratingVisuals status for cleaning too
        let mut slide_assets = Vec::new();
        for (index, timestamp, frame_path) in slides {
            let frame_remote = format!("jobs/{}/raw/frame_{:04}.jpg", job_id, index);
            let frame_url = self.storage_repo.upload_file(&frame_path, &frame_remote).await?;

            tracing::info!("[Job {}] Cleaning watermark for slide {}...", job_id, index);
            let clean_remote = format!("jobs/{}/cleaned/frame_{:04}.jpg", job_id, index);
            
            // Dynamic Scaling: Scale up Diffusion Engine
            self.scaling_repo.scale_up("keryx", "keryx-diffusion-engine").await?;
            let cleaned_url = self.stylizer.clean_watermark(&frame_url, &clean_remote).await?;

            slide_assets.push(SlideAsset {
                slide_index: index,
                original_frame: cleaned_url, // Use the cleaned frame as the base/original for downstream
                timestamp,
                translations: std::collections::HashMap::new(),
            });
        }

        job.assets_map = slide_assets;
        job.status = JobStatus::Transcribing;
        self.job_repo.save(&job).await?;

        // 5. Build editable PPTX from cleaned frames
        tracing::info!("[Job {}] Phase 4b: Building PPTX from cleaned slides...", job_id);
        let pptx_slides: Vec<SlideInput> = job.assets_map.iter().map(|s| SlideInput {
            image_url: s.original_frame.clone(),
            text: String::new(), // transcript will be filled after STT
        }).collect();
        
        // Dynamic Scaling: Scale up PPTX Builder
        self.scaling_repo.scale_up("keryx", "keryx-pptx-builder").await?;
        let pptx_url = self.pptx_repo.build(&job_id.to_string(), pptx_slides).await?;
        
        // Scale down PPTX Builder after use
        self.scaling_repo.scale_down("keryx", "keryx-pptx-builder").await?;
        
        tracing::info!("[Job {}] PPTX available at: {}", job_id, pptx_url);

        // 5. Transcribe
        tracing::info!("[Job {}] Phase 4: Transcribing audio...", job_id);
        
        // Dynamic Scaling: Scale up Whisper
        self.scaling_repo.scale_up("openai-whisper-asr-webservice", "openai-whisper-asr-webservice").await?;
        let transcription = self.stt_repo.transcribe(&audio_path).await?;
        
        // Scale down Whisper after use
        self.scaling_repo.scale_down("openai-whisper-asr-webservice", "openai-whisper-asr-webservice").await?;
        
        tracing::info!("[Job {}] Transcription complete. Generated {} segments.", job_id, transcription.segments.len());

        // 6. Generate Sync Metadata
        let sync_metadata = serde_json::json!({
            "job_id": job_id.to_string(),
            "source_url": job.source_url,
            "slides": job.assets_map.iter().map(|s| {
                serde_json::json!({
                    "index": s.slide_index,
                    "timestamp": s.timestamp,
                    "frame_url": s.original_frame
                })
            }).collect::<Vec<_>>(),
            "transcription": transcription.segments.iter().map(|s| {
                serde_json::json!({
                    "start": s.start,
                    "end": s.end,
                    "text": s.text
                })
            }).collect::<Vec<_>>()
        });

        let metadata_path = video_path.with_file_name(format!("{}_metadata.json", job_id));
        std::fs::write(&metadata_path, serde_json::to_string_pretty(&sync_metadata)?)?;

        let metadata_remote = format!("jobs/{}/sync_metadata.json", job_id);
        self.storage_repo.upload_file(&metadata_path, &metadata_remote).await?;

        // Match transcription segments to slides
        let slide_offsets: Vec<(f64, Option<f64>)> = job.assets_map.iter().enumerate().map(|(i, s)| {
            let next = job.assets_map.get(i+1).map(|ns| ns.timestamp);
            (s.timestamp, next)
        }).collect();

        let total_slides = job.assets_map.len();
        for (i, slide) in job.assets_map.iter_mut().enumerate() {
            let (start, next_start) = slide_offsets[i];
            let slide_text: Vec<String> = transcription.segments.iter()
                .filter(|s| s.start >= start && next_start.map_or(true, |ns: f64| s.end < ns))
                .map(|s| s.text.clone())
                .collect();

            let original_text = slide_text.join(" ");

            // 7. Translate & Style
            for lang in &job.target_langs {
                tracing::info!("[Job {}] Phase 5: Translating and restyling slide {}/{} (lang: {})", job_id, i+1, total_slides, lang);
                
                // Dynamic Scaling: Scale up Ollama
                self.scaling_repo.scale_up("ollama", "ollama").await?;
                let translated = self.translator.translate(&original_text, lang).await?;

                // 8. Style Image
                let style_prompt = &job.style_config.prompt;
                self.scaling_repo.scale_up("keryx", "keryx-diffusion-engine").await?;
                let styled_url = self.stylizer.style_image(&slide.original_frame, style_prompt).await?;

                slide.translations.insert(lang.clone(), TranslationAsset {
                    text: translated,
                    styled_image: Some(styled_url),
                    audio: None,
                    duration: 0.0,
                });
            }
        }

        // 9. Generate S3 Reconstruction Metadata (ffconcat) for video rebuild
        tracing::info!("[Job {}] Phase 6: Generating reconstruction metadata...", job_id);
        let mut concat_content = String::from("ffconcat version 1.0\n");
        let last_timestamp = slide_offsets.last().map(|(t, _)| *t).unwrap_or(0.0);

        // We'll estimate the end of the last slide using the transcription segments
        let total_duration = transcription.segments.last().map(|s| s.end).unwrap_or(last_timestamp + 5.0);

        for (i, slide) in job.assets_map.iter().enumerate() {
            let (start, next_start) = slide_offsets[i];
            let duration = next_start.unwrap_or(total_duration) - start;

            // Re-format S3 link to be relative if needed, or use full URL
            // FFmpeg concat expects local or accessible paths.
            // In a k8s environment, we'll use the S3 URLs or local cache.
            concat_content.push_str(&format!("file '{}'\n", slide.original_frame));
            concat_content.push_str(&format!("duration {:.3}\n", duration));
        }

        let concat_path = video_path.with_file_name(format!("{}_reconstruct.ffconcat", job_id));
        std::fs::write(&concat_path, concat_content)?;

        let concat_remote = format!("jobs/{}/reconstruct.ffconcat", job_id);
        self.storage_repo.upload_file(&concat_path, &concat_remote).await?;

        tracing::info!("[Job {}] Ingestion and reconstruction metadata complete.", job_id);

        // Final Scale Down of all services
        let _ = self.scaling_repo.scale_down("keryx", "keryx-diffusion-engine").await;
        let _ = self.scaling_repo.scale_down("ollama", "ollama").await;
        let _ = self.scaling_repo.scale_down("openai-whisper-asr-webservice", "openai-whisper-asr-webservice").await;
        let _ = self.scaling_repo.scale_down("keryx", "keryx-pptx-builder").await;

        Ok(())
    }
}
