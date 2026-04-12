use std::sync::Arc;
use uuid::Uuid;
use anyhow::Result;
use std::path::PathBuf;
use keryx_core::domain::ports::job_repository::JobRepository;
use keryx_core::domain::ports::storage_repository::StorageRepository;
use keryx_core::domain::ports::video_repository::{VideoDownloader, VideoAnalyzer, VideoReconstructor};
use keryx_core::domain::ports::stt_repository::STTRepository;
use keryx_core::domain::ports::translator_repository::TranslatorRepository;
use keryx_core::domain::ports::stylizer_repository::StylizerRepository;
use keryx_core::domain::ports::pptx_repository::{PptxRepository, SlideInput};
use keryx_core::domain::ports::scaling_repository::ScalingRepository;
use keryx_core::domain::ports::tts_repository::TTSRepository;
use keryx_core::domain::ports::voice_cloner_repository::VoiceClonerRepository;
use keryx_core::domain::ports::notification_repository::NotificationRepository;
use keryx_core::domain::entities::job::JobStatus;

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

    /// Log une ligne dans Redis ET dans tracing simultanement.
    async fn log(&self, job_id: Uuid, msg: &str) {
        tracing::info!("[Job {}] {}", job_id, msg);
        let _ = self.job_repo.append_log(job_id, msg).await;
    }

    pub async fn execute(&self, job_id: Uuid) -> Result<()> {
        let res = self.execute_internal(job_id).await;

        // Final Scale Down — toujours exécuté peu importe succès ou échec
        self.log(job_id, "Phase finale : scale down de tous les services AI...").await;
        let _ = self.scaling_repo.scale_down("keryx", "keryx-diffusion-engine").await;
        let _ = self.scaling_repo.scale_down("ollama", "ollama").await;
        let _ = self.scaling_repo.scale_down("openai-whisper-asr-webservice", "openai-whisper-asr-webservice").await;
        let _ = self.scaling_repo.scale_down("keryx", "keryx-pptx-builder").await;
        let _ = self.scaling_repo.scale_down("qwen-tts", "qwen3-tts").await;
        let _ = self.scaling_repo.scale_down("keryx", "voice-cloner").await;

        if let Err(e) = &res {
            let error_details = format!("{:?}", e);
            let msg = format!("ERREUR FATALE: {}", e);
            let _ = self.job_repo.append_log(job_id, &msg).await;
            let _ = self.notification_repo.notify_slack(&format!(
                "❌ *Job {} failed !*\n\n*Error:* {}\n\n*Details:* ```{}```",
                job_id, e, error_details
            )).await;
        }

        res
    }

    async fn execute_internal(&self, job_id: Uuid) -> Result<()> {
        let job = self.job_repo.find_by_id(job_id).await?
            .ok_or_else(|| anyhow::anyhow!("Job {} not found", job_id))?;

        self.log(job_id, &format!("Démarrage de l'ingestion pour {}", job.source_url)).await;

        // Phase 0 : Pre-scale de tous les services en parallèle
        self.log(job_id, "Phase 0 : Pre-scaling de tous les services AI (Diffusion, Whisper, Ollama, TTS, Voice, PPTX)...").await;
        let s0 = self.scaling_repo.scale_up("keryx", "keryx-diffusion-engine");
        let s1 = self.scaling_repo.scale_up("openai-whisper-asr-webservice", "openai-whisper-asr-webservice");
        let s2 = self.scaling_repo.scale_up("ollama", "ollama");
        let s3 = self.scaling_repo.scale_up("qwen-tts", "qwen3-tts");
        let s4 = self.scaling_repo.scale_up("keryx", "voice-cloner");
        let s5 = self.scaling_repo.scale_up("keryx", "keryx-pptx-builder");

        let (r0, r1, r2, r3, r4, r5) = tokio::join!(s0, s1, s2, s3, s4, s5);
        r0?; r1?; r2?; r3?; r4?; r5?;

        // Phase 0b : Health checks en parallèle
        self.log(job_id, "Phase 0b : Attente des health checks de tous les services...").await;
        let p0 = self.scaling_repo.wait_for_service_ping("keryx-diffusion-engine");
        let p1 = self.scaling_repo.wait_for_service_ping("openai-whisper-asr-webservice.openai-whisper-asr-webservice.svc.cluster.local:9000");
        let p2 = self.scaling_repo.wait_for_service_ping("ollama.ollama.svc.cluster.local:11434");
        let p3 = self.scaling_repo.wait_for_service_ping("qwen3-tts.qwen-tts.svc.cluster.local:7860");
        let p4 = self.scaling_repo.wait_for_service_ping("voice-cloner.keryx.svc.cluster.local:9880");
        let p5 = self.scaling_repo.wait_for_service_ping("keryx-pptx-builder.keryx.svc.cluster.local:80");

        let (pr0, pr1, pr2, pr3, pr4, pr5) = tokio::join!(p0, p1, p2, p3, p4, p5);
        pr0?; pr1?; pr2?; pr3?; pr4?; pr5?;
        self.log(job_id, "Phase 0b : Tous les services sont opérationnels.").await;

        // Phase 1 : Téléchargement
        self.log(job_id, "Phase 1 : Téléchargement de la vidéo et de l'audio...").await;
        self.job_repo.update_status(job_id, JobStatus::Downloading).await?;
        let (video_path, audio_path, _) = self.downloader.download(&job.source_url).await?;
        self.log(job_id, "Phase 1 : Téléchargement terminé.").await;
        self.notification_repo.notify_slack(&format!("📥 [Job {}] *Phase 1 Finished:* Video downloaded successfully.", job_id)).await?;

        // Phase 2 : Transcription STT
        self.log(job_id, "Phase 2 : Transcription de l'audio original (STT)...").await;
        self.job_repo.update_status(job_id, JobStatus::Transcribing).await?;
        let transcription = self.stt_repo.transcribe(&audio_path).await?;
        let total_video_duration = transcription.segments.last().map(|s| s.end).unwrap_or(0.0);
        self.log(job_id, &format!("Phase 2 : Transcription terminée — {} segments, durée totale {:.1}s.", transcription.segments.len(), total_video_duration)).await;

        // Upload transcription vers S3
        let trans_json = serde_json::to_string_pretty(&transcription)?;
        let trans_path = audio_path.with_extension("json");
        std::fs::write(&trans_path, trans_json)?;
        let trans_url = self.storage_repo.upload_file(&trans_path, &format!("jobs/{}/transcription.json", job_id)).await?;
        self.log(job_id, &format!("Transcription uploadée : {}", trans_url)).await;
        self.notification_repo.notify_slack(&format!("📝 [Job {}] *Phase 2 Finished:* Transcription completed ({}s). Result: {}", job_id, total_video_duration, trans_url)).await?;

        // Phase 3 : Analyse et nettoyage des slides
        self.log(job_id, "Phase 3 : Détection et nettoyage des slides...").await;
        self.job_repo.update_status(job_id, JobStatus::Analyzing).await?;
        let slides = self.analyzer.detect_slides(&video_path).await?;
        self.log(job_id, &format!("Phase 3 : {} slides détectées.", slides.len())).await;

        let mut cleaned_frames = Vec::new();
        let mut pptx_inputs = Vec::new();

        for (index, timestamp, frame_path) in slides.iter() {
            let frame_remote = format!("jobs/{}/raw/frame_{:04}.jpg", job_id, index);
            let frame_url = self.storage_repo.upload_file(&frame_path, &frame_remote).await?;

            self.log(job_id, &format!("Nettoyage du watermark de la slide {}...", index)).await;
            let cleaned_local_path = frame_path.with_file_name(format!("cleaned_{:04}.jpg", index));
            let cleaned_url = self.stylizer.clean_watermark(&frame_url, &cleaned_local_path.to_string_lossy()).await?;

            cleaned_frames.push((cleaned_local_path, *timestamp));
            pptx_inputs.push(SlideInput { image_url: cleaned_url, text: String::new() });
        }
        self.log(job_id, &format!("Phase 3 : {} slides nettoyées et stylisées.", slides.len())).await;
        self.notification_repo.notify_slack(&format!("✨ [Job {}] *Phase 3 Finished:* {} slides cleaned and stylized.", job_id, slides.len())).await?;

        // Calcul des durées par slide
        let mut frames_with_durations = Vec::new();
        for i in 0..cleaned_frames.len() {
            let duration = if i + 1 < cleaned_frames.len() {
                cleaned_frames[i+1].1 - cleaned_frames[i].1
            } else {
                total_video_duration - cleaned_frames[i].1
            };
            frames_with_durations.push((cleaned_frames[i].0.clone(), duration));
        }

        // Phase 4 : Reconstruction vidéo silencieuse
        self.log(job_id, "Phase 4 : Reconstruction vidéo (assemblage des slides)...").await;
        self.job_repo.update_status(job_id, JobStatus::Composing).await?;
        let silent_video_path = video_path.with_file_name(format!("{}_silent.mp4", job_id));
        self.reconstructor.concat_images(&frames_with_durations, &silent_video_path).await?;
        self.log(job_id, "Phase 4 : Vidéo silencieuse assemblée.").await;

        // Phase 5 : Génération des pistes audio (traduction + TTS + clonage vocal)
        self.log(job_id, &format!("Phase 5 : Génération de {} pistes audio multi-voix...", transcription.segments.len())).await;
        self.job_repo.update_status(job_id, JobStatus::Translating).await?;
        let mut fr_audio_segments = Vec::new();
        let mut jf_audio_segments = Vec::new();

        for (i, segment) in transcription.segments.iter().enumerate() {
            self.log(job_id, &format!("Segment {}/{} : traduction + TTS + clonage vocal...", i + 1, transcription.segments.len())).await;
            let fr_text = self.translator.translate(&segment.text, "fr").await?;
            let fr_seg_path = audio_path.with_file_name(format!("seg_{}_fr.wav", i));
            let jf_seg_path = audio_path.with_file_name(format!("seg_{}_jf.wav", i));

            self.job_repo.update_status(job_id, JobStatus::CloningVoice).await?;
            self.tts_repo.generate(&fr_text, "fr", &fr_seg_path).await?;
            self.voice_cloner_repo.voice_clone(&fr_text, "fr", None, &jf_seg_path).await?;

            fr_audio_segments.push(fr_seg_path);
            jf_audio_segments.push(jf_seg_path);
        }
        self.log(job_id, &format!("Phase 5 : {} pistes audio générées.", transcription.segments.len())).await;
        self.notification_repo.notify_slack(&format!("🎙️ [Job {}] *Phase 5 Finished:* {} high-quality audio tracks generated.", job_id, transcription.segments.len())).await?;

        // Phase 6 : Export final (EN, FR TTS, JF) et PPTX
        self.log(job_id, "Phase 6 : Export final (EN, FR TTS, JF Voice) et génération PPTX...").await;

        let intro_path = PathBuf::from("begin.mp4");

        // Version EN (voix originale)
        self.log(job_id, "Export EN (voix originale)...").await;
        let video_en_recon = video_path.with_file_name(format!("{}_en_recon.mp4", job_id));
        self.reconstructor.reconstruct(&silent_video_path, &audio_path, &video_en_recon).await?;
        let video_en_path = video_path.with_file_name(format!("{}_en.mp4", job_id));
        self.reconstructor.concat_with_transition(&intro_path, &video_en_recon, &video_en_path).await?;
        let url_en = self.storage_repo.upload_file(&video_en_path, &format!("jobs/{}/exports/video_en.mp4", job_id)).await?;
        self.log(job_id, &format!("Version EN uploadée : {}", url_en)).await;

        // Version FR TTS (voix Qwen)
        self.log(job_id, "Export FR (voix TTS Qwen)...").await;
        let full_fr_audio = audio_path.with_file_name(format!("{}_full_fr.wav", job_id));
        self.reconstructor.concat_audio(&fr_audio_segments, &full_fr_audio).await?;
        let video_fr_recon = video_path.with_file_name(format!("{}_fr_tts_recon.mp4", job_id));
        self.reconstructor.reconstruct(&silent_video_path, &full_fr_audio, &video_fr_recon).await?;
        let video_fr_path = video_path.with_file_name(format!("{}_fr_tts.mp4", job_id));
        self.reconstructor.concat_with_transition(&intro_path, &video_fr_recon, &video_fr_path).await?;
        let url_fr = self.storage_repo.upload_file(&video_fr_path, &format!("jobs/{}/exports/video_fr_tts.mp4", job_id)).await?;
        self.log(job_id, &format!("Version FR uploadée : {}", url_fr)).await;

        // Version JF (voix clonée Joseph)
        self.log(job_id, "Export JF (voix clonée Joseph)...").await;
        let full_jf_audio = audio_path.with_file_name(format!("{}_full_jf.wav", job_id));
        self.reconstructor.concat_audio(&jf_audio_segments, &full_jf_audio).await?;
        let video_jf_recon = video_path.with_file_name(format!("{}_jf_recon.mp4", job_id));
        self.reconstructor.reconstruct(&silent_video_path, &full_jf_audio, &video_jf_recon).await?;
        let video_jf_path = video_path.with_file_name(format!("{}_jf.mp4", job_id));
        self.reconstructor.concat_with_transition(&intro_path, &video_jf_recon, &video_jf_path).await?;
        let url_jf = self.storage_repo.upload_file(&video_jf_path, &format!("jobs/{}/exports/video_jf.mp4", job_id)).await?;
        self.log(job_id, &format!("Version JF uploadée : {}", url_jf)).await;

        // PPTX
        self.log(job_id, "Génération du fichier PPTX...").await;
        let pptx_url = self.pptx_repo.build(&job_id.to_string(), pptx_inputs).await?;
        self.log(job_id, &format!("PPTX généré : {}", pptx_url)).await;

        // Phase 7 : Notification Slack finale
        let slack_msg = format!(
            "📽️ *Job Concluded: {}*\n\n✅ *Original (Cleaned):* {}\n✅ *French TTS:* {}\n✅ *Joseph Voice:* {}\n📊 *Editable PPTX:* {}",
            job_id, url_en, url_fr, url_jf, pptx_url
        );
        self.notification_repo.notify_slack(&slack_msg).await?;

        self.job_repo.update_status(job_id, JobStatus::Completed).await?;
        self.log(job_id, "✅ Job complètement terminé et notifié. Tous les exports sont disponibles.").await;

        Ok(())
    }
}
