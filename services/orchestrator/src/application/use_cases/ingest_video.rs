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
use crate::infrastructure::clients::diffusion_engine::DiffusionEngineClient;
use crate::infrastructure::clients::pptx_builder::{PptxBuilderClient, PptxSlide};
use crate::domain::job_tracking::{JobTrackingData, CleanedSlide};
use sha2::{Sha256, Digest};

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
    diffusion_engine: Arc<DiffusionEngineClient>,
    pptx_builder: Arc<PptxBuilderClient>,
    gpu_semaphore: Arc<tokio::sync::Semaphore>,
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
        diffusion_engine: Arc<DiffusionEngineClient>,
        pptx_builder: Arc<PptxBuilderClient>,
        gpu_semaphore: Arc<tokio::sync::Semaphore>,
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
            diffusion_engine,
            pptx_builder,
            gpu_semaphore,
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

    fn calculate_hash(&self, url: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(url.as_bytes());
        hex::encode(hasher.finalize())
    }

    async fn load_tracking_data(&self, hash: &str) -> Option<JobTrackingData> {
        let path = format!("jobs/{}/tracking.json", hash);
        match self._storage_repo.get_file_content(&path).await {
            Ok(content) => serde_json::from_slice(&content).ok(),
            Err(_) => None,
        }
    }

    async fn save_tracking_data(&self, data: &JobTrackingData) -> Result<()> {
        let path = format!("jobs/{}/tracking.json", data.url_hash);
        let json = serde_json::to_vec(data)?;
        self._storage_repo.upload_buffer(json, &path, "application/json").await?;
        Ok(())
    }

    pub async fn execute(&self, job_id: Uuid) -> Result<()> {
        let res = self.execute_internal(job_id).await;

        // Final Scale Down - skipped if error or debug mode enabled to allow log inspection
        let keep_workers_env = std::env::var("KERYX_DEBUG_KEEP_WORKERS").unwrap_or_default() == "true";
        
        if res.is_ok() && !keep_workers_env {
            self.log(job_id, "Phase finale : scale down de tous les services AI...").await;
            let _ = self.scaling_repo.scale_down("keryx", "keryx-extractor").await;
            let _ = self.scaling_repo.scale_down("keryx", "keryx-dewatermark").await;
            let _ = self.scaling_repo.scale_down("keryx", "keryx-voice-extractor").await;
            let _ = self.scaling_repo.scale_down("keryx", "keryx-voice-cloner").await;
            let _ = self.scaling_repo.scale_down("keryx", "keryx-video-composer").await;
            let _ = self.scaling_repo.scale_down("keryx", "keryx-video-generator").await;
            let _ = self.scaling_repo.scale_down("keryx", "keryx-diffusion-engine").await;
            let _ = self.scaling_repo.scale_down("keryx", "keryx-pptx-builder").await;
        } else if res.is_err() {
            self.log(job_id, "⚠️ Erreur détectée : Les workers sont maintenus actifs pour inspection des logs.").await;
        } else {
            self.log(job_id, "ℹ️ Mode DEBUG actif : Les workers sont maintenus actifs.").await;
        }

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

        let url_hash = self.calculate_hash(&job.source_url);
        self.log(job_id, &format!("Hash calculé pour l'URL : {}", url_hash)).await;

        let mut tracking = self.load_tracking_data(&url_hash).await.unwrap_or_else(|| {
            JobTrackingData {
                url_hash: url_hash.clone(),
                source_url: job.source_url.clone(),
                ..Default::default()
            }
        });

        self.log(job_id, &format!("Démarrage de l'orchestration distribuée pour {}", job.source_url)).await;

        // Phase 1 : Extraction (yt-dlp + ffmpeg distant)
        let ext_res = if let Some(existing) = tracking.extraction.clone() {
            self.log(job_id, "Phase 1 : Déjà réalisée. Chargement depuis le cache S3...").await;
            existing
        } else {
            self.log(job_id, "Phase 1 : Extraction audio/vidéo via microservice...").await;
            self.scaling_repo.scale_up("keryx", "keryx-extractor").await?;
            self.job_repo.update_status(job_id, JobStatus::Downloading).await?;
            let res = self.extractor.extract(&job.source_url, &job_id.to_string()).await?;
            self.log(job_id, "Extraction terminée. Libération du worker extractor...").await;
            let _ = self.scaling_repo.scale_down("keryx", "keryx-extractor").await;
            
            tracking.extraction = Some(res.clone());
            self.save_tracking_data(&tracking).await?;
            res
        };
        self.log(job_id, &format!("Phase 1 terminée. Titre: {}", ext_res.title)).await;

        // Phase 2 : Transcription STT
        let trans_res = if let Some(existing) = tracking.transcription.clone() {
            self.log(job_id, "Phase 2 : Déjà réalisée. Chargement depuis le cache S3...").await;
            existing
        } else {
            self.log(job_id, "Phase 2 : Transcription via microservice (Whisper)...").await;
            self.scaling_repo.scale_up("keryx", "keryx-voice-extractor").await?;
            self.job_repo.update_status(job_id, JobStatus::Transcribing).await?;
            let res = self.voice_extractor.perform_transcription(&ext_res.audio_url, &job_id.to_string(), None).await?;
            self.log(job_id, "Transcription terminée. Libération du worker voice-extractor...").await;
            let _ = self.scaling_repo.scale_down("keryx", "keryx-voice-extractor").await;
            
            tracking.transcription = Some(res.clone());
            self.save_tracking_data(&tracking).await?;
            res
        };
        self.log(job_id, &format!("Phase 2 : Transcription terminée — {} segments.", trans_res.segments.len())).await;

        // Phase 3 : Analyse et nettoyage des slides
        let slide_res = if let Some(existing) = tracking.slide_detection.clone() {
            self.log(job_id, "Phase 3 (Detection) : Déjà réalisée. Chargement depuis le cache S3...").await;
            existing
        } else {
            self.log(job_id, "Phase 3 : Détection des slides via microservice...").await;
            self.scaling_repo.scale_up("keryx", "keryx-video-composer").await?;
            self.job_repo.update_status(job_id, JobStatus::Analyzing).await?;
            let res = self.video_composer.detect_slides(&job_id.to_string(), &ext_res.video_url).await?;
            let _ = self.scaling_repo.scale_down("keryx", "keryx-video-composer").await;
            
            tracking.slide_detection = Some(res.clone());
            self.save_tracking_data(&tracking).await?;
            res
        };

        if tracking.cleaned_slides.len() < slide_res.slides.len() {
            let _perm = self.gpu_semaphore.acquire().await?;
            for slide in slide_res.slides.iter().skip(tracking.cleaned_slides.len()) {
                self.log(job_id, &format!("Nettoyage slide {}...", slide.index)).await;
                let clean_res = self.dewatermark.clean_image(&slide.image_url, &job_id.to_string(), false).await?;
                tracking.cleaned_slides.push(CleanedSlide {
                    index: slide.index,
                    original_url: slide.image_url.clone(),
                    cleaned_url: clean_res.url,
                    timestamp: slide.timestamp,
                });
                self.save_tracking_data(&tracking).await?;
            }
            self.log(job_id, "Nettoyage slides terminé. Libération du worker dewatermark...").await;
            let _ = self.scaling_repo.scale_down("keryx", "keryx-dewatermark").await;
        } else {
            self.log(job_id, "Phase 3 (Cleaning) : Toutes les slides sont déjà nettoyées.").await;
        }
 
        // Phase 3C : Stylisation des slides (Bonus / Optionnel)
        if tracking.styled_slides.len() < tracking.cleaned_slides.len() {
            let _perm = self.gpu_semaphore.acquire().await?;
            self.log(job_id, "Phase 3C : Stylisation des slides via Diffusion Engine...").await;
            self.scaling_repo.scale_up("keryx", "keryx-diffusion-engine").await?;
            
            for slide in tracking.cleaned_slides.iter().skip(tracking.styled_slides.len()) {
                self.log(job_id, &format!("Stylisation slide {}...", slide.index)).await;
                // Prompt par défaut ou celui du job (à étendre)
                let prompt = "SaaS professional presentation, clean UI, tech aesthetic";
                let style_res = self.diffusion_engine.style_image(&slide.cleaned_url, prompt, 0.5, 0.0, 2, None).await?;
                
                tracking.styled_slides.push(StyledSlide {
                    index: slide.index,
                    original_url: slide.cleaned_url.clone(),
                    styled_url: style_res.url,
                    timestamp: slide.timestamp,
                });
                self.save_tracking_data(&tracking).await?;
            }
            self.log(job_id, "Stylisation slides terminée. Libération du worker diffusion-engine...").await;
            let _ = self.scaling_repo.scale_down("keryx", "keryx-diffusion-engine").await;
        }

        let mut slides_input = Vec::new();
        for i in 0..tracking.cleaned_slides.len() {
            let duration = if i + 1 < tracking.cleaned_slides.len() {
                tracking.cleaned_slides[i+1].timestamp - tracking.cleaned_slides[i].timestamp
            } else {
                trans_res.duration - tracking.cleaned_slides[i].timestamp
            };
            slides_input.push(LocalSlideInput {
                image_url: tracking.cleaned_slides[i].cleaned_url.clone(),
                duration,
                timestamp: tracking.cleaned_slides[i].timestamp,
            });
        }

        // Phase 4 : Traduction et Clonage
        let trans_segments = if let Some(existing) = tracking.translation_segments.clone() {
            self.log(job_id, "Phase 4 (Traduction) : Déjà réalisée. Chargement depuis le cache S3...").await;
            existing
        } else {
            self.log(job_id, "Phase 4 : Traduction via microservice...").await;
            self.scaling_repo.scale_up("keryx", "keryx-voice-extractor").await?;
            self.job_repo.update_status(job_id, JobStatus::Translating).await?;
            let res = self.voice_extractor.translate(trans_res.segments.clone(), "fr", &job_id.to_string()).await?;
            let _ = self.scaling_repo.scale_down("keryx", "keryx-voice-extractor").await;
            
            tracking.translation_segments = Some(res.segments.clone());
            self.save_tracking_data(&tracking).await?;
            res.segments
        };

        if tracking.cloned_audio_urls.len() < trans_segments.len() {
            let _perm = self.gpu_semaphore.acquire().await?;
            for i in tracking.cloned_audio_urls.len()..trans_segments.len() {
                let seg = &trans_segments[i];
                let text = seg.translated.clone().unwrap_or_else(|| seg.text.clone());
                self.log(job_id, &format!("Génération clonage vocal segment {}/{}...", i+1, trans_segments.len())).await;
                let clone_res = self.voice_cloner.perform_cloning(&text, "fr", &ext_res.audio_url, &job_id.to_string()).await?;
                tracking.cloned_audio_urls.push(clone_res.url);
                self.save_tracking_data(&tracking).await?;
            }
            self.log(job_id, "Clonage vocal terminé. Libération du worker voice-cloner...").await;
            let _ = self.scaling_repo.scale_down("keryx", "keryx-voice-cloner").await;
        } else {
            self.log(job_id, "Phase 4 (Clonage) : Tous les segments audio sont déjà générés.").await;
        }

        let final_audio_url = if let Some(existing) = tracking.final_audio_url.clone() {
            self.log(job_id, "Phase 4 (Assemblage) : Déjà réalisée. Chargement depuis le cache S3...").await;
            existing
        } else {
            self.log(job_id, "Phase 4 : Assemblage de la piste audio finale...").await;
            self.scaling_repo.scale_up("keryx", "keryx-video-composer").await?;
            let res = self.video_composer.concat_audio(&job_id.to_string(), tracking.cloned_audio_urls.clone()).await?;
            tracking.final_audio_url = Some(res.url.clone());
            self.save_tracking_data(&tracking).await?;
            res.url
        };

        // Phase 5 : Composition Vidéo
        let final_video_url = if let Some(existing) = tracking.final_video_url.clone() {
            self.log(job_id, "Phase 5 : Déjà réalisée. Chargement depuis le cache S3...").await;
        } else {
            let _perm = self.gpu_semaphore.acquire().await?;
            let composer_slides: Vec<ComposerSlideInput> = slides_input.iter().map(|s| {
                ComposerSlideInput {
                    image_url: s.image_url.clone(),
                    duration: s.duration,
                }
            }).collect();

            let res = self.video_composer.compose(
                &job_id.to_string(), 
                composer_slides, 
                Some(final_audio_url.clone())
            ).await?;
            self.log(job_id, "Composition vidéo terminée. Libération du worker video-composer...").await;
            let _ = self.scaling_repo.scale_down("keryx", "keryx-video-composer").await;
            
            tracking.final_video_url = Some(res.url.clone());
            self.save_tracking_data(&tracking).await?;
            res.url
        };

        // Phase 6 : Animations Bonus (SVD) sur la première slide
        if let Some(first_slide) = slides_input.first() {
            let _perm = self.gpu_semaphore.acquire().await?;
            self.log(job_id, "Phase 6 : Génération d'une animation bonus (SVD) pour l'intro...").await;
            self.scaling_repo.scale_up("keryx", "keryx-video-generator").await?;
            let _ = self.video_generator.animate(&job_id.to_string(), &first_slide.image_url).await;
            let _ = self.scaling_repo.scale_down("keryx", "keryx-video-generator").await;
        }
 
        // Phase 7 : Génération PPTX
        if tracking.pptx_url.is_none() {
            self.log(job_id, "Phase 7 : Génération du support de présentation PPTX...").await;
            self.scaling_repo.scale_up("keryx", "keryx-pptx-builder").await?;
            
            let pptx_slides = tracking.styled_slides.iter().map(|s| {
                PptxSlide {
                    image_url: s.styled_url.clone(),
                    text: "".to_string(), // On pourrait ajouter les notes ici
                }
            }).collect();
            
            match self.pptx_builder.build_pptx(&job_id.to_string(), pptx_slides, None).await {
                Ok(pptx_res) => {
                    tracking.pptx_url = Some(pptx_res.url);
                    self.save_tracking_data(&tracking).await?;
                    self.log(job_id, "Génération PPTX terminée.").await;
                },
                Err(e) => {
                    self.log(job_id, &format!("⚠️ Erreur lors de la génération PPTX (non-fatale): {}", e)).await;
                }
            }
            let _ = self.scaling_repo.scale_down("keryx", "keryx-pptx-builder").await;
        }

        // Phase 7 : Notification Slack finale
        let slack_msg = format!(
            "📽️ *Job Concluded: {}*\n\n✅ *Final Video:* {}\n🎙️ *Final Audio:* {}\n📊 *Job Status:* COMPLETED",
            job_id, final_video_url, final_audio_url
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
