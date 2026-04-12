use std::sync::Arc;
use uuid::Uuid;
use anyhow::Result;
use keryx_core::domain::ports::job_repository::JobRepository;
use keryx_core::domain::ports::storage_repository::StorageRepository;
use keryx_core::domain::ports::scaling_repository::ScalingRepository;
use keryx_core::domain::ports::notification_repository::NotificationRepository;
use keryx_core::domain::entities::job::JobStatus;

use crate::infrastructure::clients::extractor::ExtractorClient;
use crate::infrastructure::clients::dewatermark::DewatermarkClient;
use crate::infrastructure::clients::voice_extractor::VoiceExtractorClient;
use crate::infrastructure::clients::voice_cloner::VoiceClonerClient;
use crate::infrastructure::clients::video_composer::{VideoComposerClient, SlideInput as ComposerSlideInput};
use crate::infrastructure::clients::video_generator::VideoGeneratorClient;

pub struct IngestVideoUseCase {
    job_repo: Arc<dyn JobRepository>,
    _storage_repo: Arc<dyn StorageRepository>,
    scaling_repo: Arc<dyn ScalingRepository>,
    notification_repo: Arc<dyn NotificationRepository>,
    
    // HTTP Clients
    extractor: Arc<ExtractorClient>,
    dewatermark: Arc<DewatermarkClient>,
    voice_extractor: Arc<VoiceExtractorClient>,
    voice_cloner: Arc<VoiceClonerClient>,
    video_composer: Arc<VideoComposerClient>,
    video_generator: Arc<VideoGeneratorClient>,
}

impl IngestVideoUseCase {
    pub fn new(
        job_repo: Arc<dyn JobRepository>,
        storage_repo: Arc<dyn StorageRepository>,
        scaling_repo: Arc<dyn ScalingRepository>,
        notification_repo: Arc<dyn NotificationRepository>,
        extractor: Arc<ExtractorClient>,
        dewatermark: Arc<DewatermarkClient>,
        voice_extractor: Arc<VoiceExtractorClient>,
        voice_cloner: Arc<VoiceClonerClient>,
        video_composer: Arc<VideoComposerClient>,
        video_generator: Arc<VideoGeneratorClient>,
    ) -> Self {
        Self { 
            job_repo, 
            _storage_repo: storage_repo, 
            scaling_repo, 
            notification_repo,
            extractor,
            dewatermark,
            voice_extractor,
            voice_cloner,
            video_composer,
            video_generator,
        }
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
        // Le scaling pourra être refactorisé plus tard pour cibler les nouveaux microservices
        let _ = self.scaling_repo.scale_down("keryx", "keryx-extractor").await;
        let _ = self.scaling_repo.scale_down("keryx", "keryx-dewatermark").await;
        let _ = self.scaling_repo.scale_down("keryx", "keryx-voice-extractor").await;
        let _ = self.scaling_repo.scale_down("keryx", "keryx-voice-cloner").await;
        let _ = self.scaling_repo.scale_down("keryx", "keryx-video-composer").await;
        let _ = self.scaling_repo.scale_down("keryx", "keryx-video-generator").await;

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

        self.log(job_id, &format!("Démarrage de l'orchestration distribuée pour {}", job.source_url)).await;

        // Phase 1 : Extraction (yt-dlp + ffmpeg distant)
        self.log(job_id, "Phase 1 : Extraction audio/vidéo via microservice...").await;
        self.scaling_repo.scale_up("keryx", "keryx-extractor").await?;
        self.job_repo.update_status(job_id, JobStatus::Downloading).await?;
        let ext_res = self.extractor.extract(&job.source_url, &job_id.to_string()).await?;
        self.log(job_id, &format!("Phase 1 terminée. Titre: {}", ext_res.title)).await;

        // Phase 2 : Transcription STT
        self.log(job_id, "Phase 2 : Transcription via microservice (Whisper)...").await;
        self.scaling_repo.scale_up("keryx", "keryx-voice-extractor").await?;
        self.job_repo.update_status(job_id, JobStatus::Transcribing).await?;
        let trans_res = self.voice_extractor.perform_transcription(&ext_res.audio_url, &job_id.to_string(), None).await?;
        self.log(job_id, &format!("Phase 2 : Transcription terminée — {} segments.", trans_res.segments.len())).await;

        // Phase 3 : Analyse et nettoyage des slides
        self.log(job_id, "Phase 3 : Détection des slides et nettoyage watermark...").await;
        self.scaling_repo.scale_up("keryx", "keryx-video-composer").await?;
        self.job_repo.update_status(job_id, JobStatus::Analyzing).await?;
        
        let slide_res = self.video_composer.detect_slides(&job_id.to_string(), &ext_res.video_url).await?;
        self.log(job_id, &format!("Phase 3 : {} slides détectées. Nettoyage en cours...", slide_res.slides.len())).await;

        self.scaling_repo.scale_up("keryx", "keryx-dewatermark").await?;

        let mut slides_input = Vec::new();
        for slide in slide_res.slides {
            self.log(job_id, &format!("Nettoyage slide {}...", slide.index)).await;
            let clean_res = self.dewatermark.clean_image(&slide.image_url, &job_id.to_string(), false).await?;
            slides_input.push(LocalSlideInput {
                image_url: clean_res.url,
                duration: 0.0, // Sera calculé après
                timestamp: slide.timestamp,
            });
        }

        // Calcul des durées par slide
        for i in 0..slides_input.len() {
            let duration = if i + 1 < slides_input.len() {
                slides_input[i+1].timestamp - slides_input[i].timestamp
            } else {
                trans_res.duration - slides_input[i].timestamp
            };
            slides_input[i].duration = duration;
        }

        // Phase 4 : Traduction et Génération Audio (FR + Clonage)
        self.log(job_id, "Phase 4 : Traduction et génération audio haute qualité...").await;
        self.scaling_repo.scale_up("keryx", "keryx-voice-extractor").await?;
        self.scaling_repo.scale_up("keryx", "keryx-voice-cloner").await?;
        self.job_repo.update_status(job_id, JobStatus::Translating).await?;
        
        let trans_lang_res = self.voice_extractor.translate(trans_res.segments.clone(), "fr", &job_id.to_string()).await?;
        
        let mut fr_audio_urls = Vec::new();
        for (i, seg) in trans_lang_res.segments.iter().enumerate() {
            let text = seg.translated.clone().unwrap_or_else(|| seg.text.clone());
            self.log(job_id, &format!("Génération clonage vocal segment {}/{}...", i+1, trans_lang_res.segments.len())).await;
            let clone_res = self.voice_cloner.perform_cloning(&text, "fr", &ext_res.audio_url, &job_id.to_string()).await?;
            fr_audio_urls.push(clone_res.url);
        }

        // Concaténation audio finale
        self.log(job_id, "Phase 4 : Assemblage de la piste audio finale...").await;
        let final_audio_res = self.video_composer.concat_audio(&job_id.to_string(), fr_audio_urls).await?;

        // Phase 5 : Composition Vidéo
        self.log(job_id, "Phase 5 : Montage final de la vidéo...").await;
        self.scaling_repo.scale_up("keryx", "keryx-video-composer").await?;
        self.job_repo.update_status(job_id, JobStatus::Composing).await?;
        
        let composer_slides: Vec<ComposerSlideInput> = slides_input.iter().map(|s| {
            ComposerSlideInput {
                image_url: s.image_url.clone(),
                duration: s.duration,
            }
        }).collect();

        let final_video_res = self.video_composer.compose(
            &job_id.to_string(), 
            composer_slides, 
            Some(final_audio_res.url.clone())
        ).await?;

        // Phase 6 : Animations Bonus (SVD) sur la première slide
        if let Some(first_slide) = slides_input.first() {
            self.log(job_id, "Phase 6 : Génération d'une animation bonus (SVD) pour l'intro...").await;
            self.scaling_repo.scale_up("keryx", "keryx-video-generator").await?;
            let _ = self.video_generator.animate(&job_id.to_string(), &first_slide.image_url).await;
        }

        // Phase 7 : Notification Slack finale
        let slack_msg = format!(
            "📽️ *Job Concluded: {}*\n\n✅ *Final Video:* {}\n🎙️ *Final Audio:* {}\n📊 *Job Status:* COMPLETED",
            job_id, final_video_res.url, final_audio_res.url
        );
        let _ = self.notification_repo.notify_slack(&slack_msg).await;

        self.job_repo.update_status(job_id, JobStatus::Completed).await?;
        self.log(job_id, "✅ Job distribué terminé avec succès.").await;

        Ok(())
    }
}

// Helper struct temporarily here to store timestamp while calculating duration
struct LocalSlideInput {
    image_url: String,
    duration: f64,
    timestamp: f64,
}
