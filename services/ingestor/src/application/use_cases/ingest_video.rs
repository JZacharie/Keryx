use std::sync::Arc;
use uuid::Uuid;
use anyhow::Result;
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
use crate::domain::ports::notification_repository::NotificationRepository;
use crate::domain::entities::job::JobStatus;

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
    notification_repo: Arc<dyn NotificationRepository>,
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
        notification_repo: Arc<dyn NotificationRepository>,
    ) -> Self {
        Self { job_repo, storage_repo, downloader, analyzer, stt_repo, translator, stylizer, pptx_repo, scaling_repo, tts_repo, voice_cloner_repo, reconstructor, notification_repo }
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
        
        if let Err(e) = &res {
            let error_details = format!("{:?}", e);
            let _ = self.notification_repo.notify_slack(&format!("❌ *Job {} failed !*\n\n*Error:* {}\n\n*Details:* ```{}```", job_id, e, error_details)).await;
        }

        res
    }

    async fn execute_internal(&self, job_id: Uuid) -> Result<()> {
        let job = self.job_repo.find_by_id(job_id).await?
            .ok_or_else(|| anyhow::anyhow!("Job {} not found", job_id))?;

        tracing::info!("[Job {}] Starting ingestion for {}", job_id, job.source_url);

        // Pre-Scale Phase: Fire all scale-ups in parallel to avoid sequential 20s delays
        tracing::info!("[Job {}] Phase 0: Pre-scaling all AI services (Diffusion, Whisper, Ollama, TTS, Voice, PPTX)...", job_id);
        let s0 = self.scaling_repo.scale_up("keryx", "keryx-diffusion-engine");
        let s1 = self.scaling_repo.scale_up("openai-whisper-asr-webservice", "openai-whisper-asr-webservice");
        let s2 = self.scaling_repo.scale_up("ollama", "ollama");
        let s3 = self.scaling_repo.scale_up("qwen-tts", "qwen3-tts");
        let s4 = self.scaling_repo.scale_up("keryx", "voice-cloner");
        let s5 = self.scaling_repo.scale_up("keryx", "keryx-pptx-builder");
        
        let (r0, r1, r2, r3, r4, r5) = tokio::join!(s0, s1, s2, s3, s4, s5);
        r0?; r1?; r2?; r3?; r4?; r5?;
        
        // Wait for all health checks to be green (parallel)
        tracing::info!("[Job {}] Phase 0b: Waiting for service health checks...", job_id);
        let p0 = self.scaling_repo.wait_for_service_ping("keryx-diffusion-engine");
        let p1 = self.scaling_repo.wait_for_service_ping("openai-whisper-asr-webservice.openai-whisper-asr-webservice.svc.cluster.local:9000");
        let p2 = self.scaling_repo.wait_for_service_ping("ollama.ollama.svc.cluster.local:11434");
        let p3 = self.scaling_repo.wait_for_service_ping("qwen3-tts.qwen-tts.svc.cluster.local:7860");
        let p4 = self.scaling_repo.wait_for_service_ping("voice-cloner.keryx.svc.cluster.local:9880");
        let p5 = self.scaling_repo.wait_for_service_ping("keryx-pptx-builder.keryx.svc.cluster.local:8002");
        
        let (pr0, pr1, pr2, pr3, pr4, pr5) = tokio::join!(p0, p1, p2, p3, p4, p5);
        pr0?; pr1?; pr2?; pr3?; pr4?; pr5?;

        // 1. Download
        tracing::info!("[Job {}] Phase 1: Downloading video and audio...", job_id);
        self.job_repo.update_status(job_id, JobStatus::Downloading).await?;
        let (video_path, audio_path, _) = self.downloader.download(&job.source_url).await?;
        self.notification_repo.notify_slack(&format!("📥 [Job {}] *Phase 1 Finished:* Video downloaded successfully.", job_id)).await?;
        

        // 2. Transcribe (Early to allow duration calculations)
        tracing::info!("[Job {}] Phase 2: Transcribing original audio...", job_id);
        let transcription = self.stt_repo.transcribe(&audio_path).await?;
        let total_video_duration = transcription.segments.last().map(|s| s.end).unwrap_or(0.0);
        
        // Upload transcription to S3
        let trans_json = serde_json::to_string_pretty(&transcription)?;
        let trans_path = audio_path.with_extension("json");
        std::fs::write(&trans_path, trans_json)?;
        let trans_url = self.storage_repo.upload_file(&trans_path, &format!("jobs/{}/transcription.json", job_id)).await?;
        
        self.notification_repo.notify_slack(&format!("📝 [Job {}] *Phase 2 Finished:* Transcription completed ({}s). Result: {}", job_id, total_video_duration, trans_url)).await?;

        // 3. Analyze and Clean
        tracing::info!("[Job {}] Phase 3: Analyzing and cleaning slides...", job_id);
        self.job_repo.update_status(job_id, JobStatus::Analyzing).await?;
        let slides = self.analyzer.detect_slides(&video_path).await?;
        
        let mut cleaned_frames = Vec::new();
        let mut pptx_inputs = Vec::new();

        for (index, timestamp, frame_path) in slides.iter() {
            let frame_remote = format!("jobs/{}/raw/frame_{:04}.jpg", job_id, index);
            let frame_url = self.storage_repo.upload_file(&frame_path, &frame_remote).await?;

            tracing::info!("[Job {}] Cleaning slide {}...", job_id, index);
            let cleaned_local_path = frame_path.with_file_name(format!("cleaned_{:04}.jpg", index));
            let cleaned_url = self.stylizer.clean_watermark(&frame_url, &cleaned_local_path.to_string_lossy()).await?;

            cleaned_frames.push((cleaned_local_path, *timestamp));
            pptx_inputs.push(SlideInput { image_url: cleaned_url, text: String::new() });
        }
        self.notification_repo.notify_slack(&format!("✨ [Job {}] *Phase 3 Finished:* {} slides cleaned and stylized.", job_id, slides.len())).await?;

        // Calculate durations
        let mut frames_with_durations = Vec::new();
        for i in 0..cleaned_frames.len() {
            let duration = if i + 1 < cleaned_frames.len() {
                cleaned_frames[i+1].1 - cleaned_frames[i].1
            } else {
                total_video_duration - cleaned_frames[i].1
            };
            frames_with_durations.push((cleaned_frames[i].0.clone(), duration));
        }

        // 4. Reconstruction
        tracing::info!("[Job {}] Phase 4: Video reconstruction...", job_id);
        let silent_video_path = video_path.with_file_name(format!("{}_silent.mp4", job_id));
        self.reconstructor.concat_images(&frames_with_durations, &silent_video_path).await?;

        // 5. Generate Audio Tracks
        tracing::info!("[Job {}] Phase 5: Generating multi-voice tracks...", job_id);
        let mut fr_audio_segments = Vec::new();
        let mut jf_audio_segments = Vec::new();

        for (i, segment) in transcription.segments.iter().enumerate() {
            let fr_text = self.translator.translate(&segment.text, "fr").await?;
            let fr_seg_path = audio_path.with_file_name(format!("seg_{}_fr.wav", i));
            let jf_seg_path = audio_path.with_file_name(format!("seg_{}_jf.wav", i));

            self.tts_repo.generate(&fr_text, "fr", &fr_seg_path).await?;
            self.voice_cloner_repo.voice_clone(&fr_text, "fr", None, &jf_seg_path).await?;

            fr_audio_segments.push(fr_seg_path);
            jf_audio_segments.push(jf_seg_path);
        }
        self.notification_repo.notify_slack(&format!("🎙️ [Job {}] *Phase 5 Finished:* {} high-quality audio tracks generated.", job_id, transcription.segments.len())).await?;

        // 6. Final Exports and PPTX
        tracing::info!("[Job {}] Phase 6: Final Exports and PPTX...", job_id);
        
        let video_en_path = video_path.with_file_name(format!("{}_en.mp4", job_id));
        self.reconstructor.reconstruct(&silent_video_path, &audio_path, &video_en_path).await?;
        let url_en = self.storage_repo.upload_file(&video_en_path, &format!("jobs/{}/exports/video_en.mp4", job_id)).await?;

        let full_fr_audio = audio_path.with_file_name(format!("{}_full_fr.wav", job_id));
        self.reconstructor.concat_audio(&fr_audio_segments, &full_fr_audio).await?;
        let video_fr_path = video_path.with_file_name(format!("{}_fr_tts.mp4", job_id));
        self.reconstructor.reconstruct(&silent_video_path, &full_fr_audio, &video_fr_path).await?;
        let url_fr = self.storage_repo.upload_file(&video_fr_path, &format!("jobs/{}/exports/video_fr_tts.mp4", job_id)).await?;

        let full_jf_audio = audio_path.with_file_name(format!("{}_full_jf.wav", job_id));
        self.reconstructor.concat_audio(&jf_audio_segments, &full_jf_audio).await?;
        let video_jf_path = video_path.with_file_name(format!("{}_jf.mp4", job_id));
        self.reconstructor.reconstruct(&silent_video_path, &full_jf_audio, &video_jf_path).await?;
        let url_jf = self.storage_repo.upload_file(&video_jf_path, &format!("jobs/{}/exports/video_jf.mp4", job_id)).await?;

        let pptx_url = self.pptx_repo.build(&job_id.to_string(), pptx_inputs).await?;

        // 7. Notify Slack
        let slack_msg = format!(
            "📽️ *Job Concluded: {}*\n\n✅ *Original (Cleaned):* {}\n✅ *French TTS:* {}\n✅ *Joseph Voice:* {}\n📊 *Editable PPTX:* {}",
            job_id, url_en, url_fr, url_jf, pptx_url
        );
        self.notification_repo.notify_slack(&slack_msg).await?;

        tracing::info!("[Job {}] Job fully completed and notified.", job_id);
        Ok(())
    }
}
