use std::sync::Arc;
use uuid::Uuid;
use anyhow::{Result, Context};
use crate::domain::ports::job_repository::JobRepository;
use crate::domain::ports::storage_repository::StorageRepository;
use crate::domain::ports::video_repository::{VideoDownloader, VideoAnalyzer, VideoReconstructor};
use crate::domain::ports::stt_repository::STTRepository;
use crate::domain::ports::translator_repository::TranslatorRepository;
use crate::domain::ports::stylizer_repository::StylizerRepository;
use crate::domain::ports::pptx_repository::{PptxRepository, SlideInput};
use crate::domain::ports::scaling_repository::ScalingRepository;
use crate::domain::ports::tts_repository::TTSRepository;
use crate::domain::ports::voice_cloner_repository::VoiceClonerRepository;
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
    tts_repo: Arc<dyn TTSRepository>,
    voice_cloner_repo: Arc<dyn VoiceClonerRepository>,
    reconstructor: Arc<dyn VideoReconstructor>,
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
        tts_repo: Arc<dyn TTSRepository>,
        voice_cloner_repo: Arc<dyn VoiceClonerRepository>,
        reconstructor: Arc<dyn VideoReconstructor>,
    ) -> Self {
        Self { job_repo, storage_repo, downloader, analyzer, stt_repo, translator, stylizer, pptx_repo, scaling_repo, tts_repo, voice_cloner_repo, reconstructor }
    }

    pub fn get_job_repo(&self) -> Arc<dyn JobRepository> {
        self.job_repo.clone()
    }

    pub async fn execute(&self, job_id: Uuid) -> Result<()> {
        let res = self.execute_internal(job_id).await;
        
        // Final Scale Down - Always run regardless of success/fail
        tracing::info!("[Job {}] Final cleanup: scaling down all AI services", job_id);
        let _ = self.scaling_repo.scale_down("keryx", "keryx-diffusion-engine").await;
        let _ = self.scaling_repo.scale_down("ollama", "ollama").await;
        let _ = self.scaling_repo.scale_down("openai-whisper-asr-webservice", "openai-whisper-asr-webservice").await;
        let _ = self.scaling_repo.scale_down("keryx", "keryx-pptx-builder").await;
        let _ = self.scaling_repo.scale_down("qwen-tts", "qwen3-tts").await;
        let _ = self.scaling_repo.scale_down("keryx", "voice-cloner").await;
        
        res
    }

    async fn execute_internal(&self, job_id: Uuid) -> Result<()> {
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

        // 3. Analyze
        tracing::info!("[Job {}] Phase 3: Analyzing video for slide transitions...", job_id);
        self.job_repo.update_status(job_id, JobStatus::Analyzing).await?;
        let slides = self.analyzer.detect_slides(&video_path).await?;
        tracing::info!("[Job {}] Analysis complete. Detected {} slides.", job_id, slides.len());

        // 4. Upload frames and clean watermarks
        self.job_repo.update_status(job_id, JobStatus::GeneratingVisuals).await?;
        let mut slide_assets = Vec::new();

        self.scaling_repo.scale_up("keryx", "keryx-diffusion-engine").await?;
        self.scaling_repo.wait_for_service_ping("keryx-diffusion-engine").await?;
        
        let mut cleaned_frames = Vec::new();
        let total_slides = slides.len();

        for (index, timestamp, frame_path) in slides.iter() {
            let frame_remote = format!("jobs/{}/raw/frame_{:04}.jpg", job_id, index);
            let frame_url = self.storage_repo.upload_file(&frame_path, &frame_remote).await?;

            tracing::info!("[Job {}] Cleaning watermark for slide {}/{}...", job_id, index, total_slides);
            let clean_remote = format!("jobs/{}/cleaned/frame_{:04}.jpg", job_id, index);
            
            // Generate a local path for the cleaned frame
            let cleaned_local_path = frame_path.with_file_name(format!("cleaned_{:04}.jpg", index));
            let cleaned_url = self.stylizer.clean_watermark(&frame_url, &cleaned_local_path.to_string_lossy()).await?;

            slide_assets.push(SlideAsset {
                slide_index: *index,
                original_frame: cleaned_url,
                timestamp: *timestamp,
                translations: std::collections::HashMap::new(),
            });
            cleaned_frames.push((cleaned_local_path, *timestamp));
        }

        // Calculate durations for each frame
        let mut frames_with_durations = Vec::new();
        for i in 0..cleaned_frames.len() {
            let duration = if i + 1 < cleaned_frames.len() {
                cleaned_frames[i+1].1 - cleaned_frames[i].1
            } else {
                5.0 // fallback duration for last slide
            };
            frames_with_durations.push((cleaned_frames[i].0.clone(), duration));
        }

        // 5. Build Silent Normalized Video
        tracing::info!("[Job {}] Phase 4: Reconstructing silent branded video...", job_id);
        let silent_video_path = video_path.with_file_name(format!("{}_silent.mp4", job_id));
        self.reconstructor.concat_images(&frames_with_durations, &silent_video_path).await?;

        // 6. Transcribe Original Audio
        tracing::info!("[Job {}] Phase 5: Transcribing and translating...", job_id);
        self.scaling_repo.scale_up("openai-whisper-asr-webservice", "openai-whisper-asr-webservice").await?;
        self.scaling_repo.wait_for_service_ping("openai-whisper-asr-webservice.openai-whisper-asr-webservice.svc.cluster.local:9000").await?;
        let transcription = self.stt_repo.transcribe(&audio_path).await?;
        
        // 7. Translate and Generate Multi-Language Audio
        self.scaling_repo.scale_up("ollama", "ollama").await?;
        self.scaling_repo.wait_for_service_ping("ollama.ollama.svc.cluster.local:11434").await?;
        self.scaling_repo.scale_up("qwen-tts", "qwen3-tts").await?;
        self.scaling_repo.wait_for_service_ping("qwen3-tts.qwen-tts.svc.cluster.local:7860").await?;
        self.scaling_repo.scale_up("keryx", "voice-cloner").await?;
        self.scaling_repo.wait_for_service_ping("voice-cloner.keryx.svc.cluster.local:9880").await?;

        let mut fr_audio_segments = Vec::new();
        let mut jf_audio_segments = Vec::new();

        for (i, segment) in transcription.segments.iter().enumerate() {
            let fr_text = self.translator.translate(&segment.text, "fr").await?;
            
            let fr_seg_path = audio_path.with_file_name(format!("seg_{}_fr.wav", i));
            let jf_seg_path = audio_path.with_file_name(format!("seg_{}_jf.wav", i));

            // Native TTS
            self.tts_repo.generate(&fr_text, "fr", &fr_seg_path).await?;
            // Joseph Voice Clone
            self.voice_cloner_repo.clone(&fr_text, "fr", None, &jf_seg_path).await?;

            fr_audio_segments.push(fr_seg_path);
            jf_audio_segments.push(jf_seg_path);
        }

        // Merge audio segments? For simplicity, we assume we need one single track.
        // In a real scenario we'd use FFmpeg to concat these audios with correct offsets.
        // For this task, let's assume we produce 3 final videos.
        
        // 8. Final Export Phase
        tracing::info!("[Job {}] Phase 6: Final Exports...", job_id);
        
        // Version 1: Original Cleaned Video (Silent Cleaned + Original Audio)
        let video_en_path = video_path.with_file_name(format!("{}_en.mp4", job_id));
        self.reconstructor.reconstruct(&silent_video_path, &audio_path, &video_en_path).await?;
        self.storage_repo.upload_file(&video_en_path, &format!("jobs/{}/exports/video_en.mp4", job_id)).await?;

        // Version 2: French TTS Video (Silent Cleaned + Concatenated FR TTS)
        let full_fr_audio = audio_path.with_file_name(format!("{}_full_fr.wav", job_id));
        self.reconstructor.concat_audio(&fr_audio_segments, &full_fr_audio).await?;
        
        let video_fr_path = video_path.with_file_name(format!("{}_fr_tts.mp4", job_id));
        self.reconstructor.reconstruct(&silent_video_path, &full_fr_audio, &video_fr_path).await?;
        self.storage_repo.upload_file(&video_fr_path, &format!("jobs/{}/exports/video_fr_tts.mp4", job_id)).await?;

        // Version 3: Joseph Voice Version (Silent Cleaned + Concatenated JF Voice)
        let full_jf_audio = audio_path.with_file_name(format!("{}_full_jf.wav", job_id));
        self.reconstructor.concat_audio(&jf_audio_segments, &full_jf_audio).await?;

        let video_jf_path = video_path.with_file_name(format!("{}_jf.mp4", job_id));
        self.reconstructor.reconstruct(&silent_video_path, &full_jf_audio, &video_jf_path).await?;
        self.storage_repo.upload_file(&video_jf_path, &format!("jobs/{}/exports/video_jf.mp4", job_id)).await?;

        tracing::info!("[Job {}] Ingestion complete. All 3 versions uploaded.", job_id);
        Ok(())
    }
}
