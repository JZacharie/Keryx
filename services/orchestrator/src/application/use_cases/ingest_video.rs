use std::sync::Arc;
use std::io::Write;
use uuid::Uuid;
use anyhow::Result;
use keryx_core::domain::ports::job_repository::JobRepository;
use keryx_core::domain::ports::storage_repository::StorageRepository;
use keryx_core::domain::ports::scaling_repository::ScalingRepository;
use keryx_core::domain::ports::notification_repository::NotificationRepository;
use keryx_core::domain::entities::job::JobStatus;

use crate::infrastructure::clients::extractor::ExtractorClient;
use crate::infrastructure::clients::dewatermark::DewatermarkClient;
use crate::infrastructure::clients::voice_extractor::{VoiceExtractorClient, Segment};
use crate::infrastructure::clients::texts_translation::TextsTranslationClient;
use crate::infrastructure::scaling_guard::WorkerGuard;
use crate::infrastructure::clients::voice_cloner::VoiceClonerClient;
use crate::infrastructure::clients::video_composer::{VideoComposerClient, SlideInput as ComposerSlideInput};
use crate::infrastructure::clients::video_generator::VideoGeneratorClient;
use crate::infrastructure::clients::diffusion_engine::DiffusionEngineClient;
use crate::infrastructure::clients::pptx_builder::{PptxBuilderClient, PptxSlide};
use crate::domain::job_tracking::{JobTrackingData, CleanedSlide, StyledSlide};
use sha2::{Sha256, Digest};

pub struct IngestVideoUseCase {
    job_repo: Arc<dyn JobRepository>,
    _storage_repo: Arc<dyn StorageRepository>,
    scaling_repo: Arc<dyn ScalingRepository>,
    notification_repo: Arc<dyn NotificationRepository>,
    
    // HTTP Clients
    extractor: Arc<ExtractorClient>,
    voice_extractor: Arc<VoiceExtractorClient>,
    texts_translation: Arc<TextsTranslationClient>,
    voice_cloner: Arc<VoiceClonerClient>,
    video_composer: Arc<VideoComposerClient>,
    voices_composer: Arc<VideoComposerClient>,
    video_generator: Arc<VideoGeneratorClient>,
    _dewatermark: Arc<DewatermarkClient>,
    _diffusion_engine: Arc<DiffusionEngineClient>,
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
        texts_translation: Arc<TextsTranslationClient>,
        voice_cloner: Arc<VoiceClonerClient>,
        video_composer: Arc<VideoComposerClient>,
        voices_composer: Arc<VideoComposerClient>,
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
            voice_extractor,
            texts_translation,
            voice_cloner,
            video_composer,
            voices_composer,
            video_generator,
            _dewatermark: dewatermark,
            _diffusion_engine: diffusion_engine,
            pptx_builder,
            gpu_semaphore,
        }
    }

    pub fn get_job_repo(&self) -> Arc<dyn JobRepository> {
        self.job_repo.clone()
    }

    /// Log une ligne dans Redis ET dans tracing simultanement.
    async fn log(&self, job_id: Uuid, msg: &str) {
        println!("[Job {}] {}", job_id, msg);
        let _ = std::io::stdout().flush();
        tracing::info!("[Job {}] {}", job_id, msg);
        let _ = self.job_repo.append_log(job_id, msg).await;
    }

    fn calculate_hash(&self, url: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(url.as_bytes());
        hex::encode(hasher.finalize())
    }

    async fn load_tracking_data(&self, hash: &str) -> Option<JobTrackingData> {
        let path = format!("{}/orchestrator/tracking.json", hash);
        match self._storage_repo.get_file_content(&path).await {
            Ok(content) => serde_json::from_slice(&content).ok(),
            Err(_) => None,
        }
    }

    async fn save_tracking_data(&self, data: &JobTrackingData) -> Result<()> {
        let path = format!("{}/orchestrator/tracking.json", data.url_hash);
        let json = serde_json::to_vec(data)?;
        self._storage_repo.upload_buffer(json, &path, "application/json").await?;
        Ok(())
    }

    pub async fn get_tracking_data(&self, job_id: Uuid) -> Result<Option<JobTrackingData>> {
        let job = self.job_repo.find_by_id(job_id).await?
            .ok_or_else(|| anyhow::anyhow!("Job {} not found", job_id))?;
        let url_hash = self.calculate_hash(&job.source_url);
        Ok(self.load_tracking_data(&url_hash).await)
    }

    pub async fn restart_from_step(&self, job_id: Uuid, step: &str) -> Result<()> {
        let job = self.job_repo.find_by_id(job_id).await?
            .ok_or_else(|| anyhow::anyhow!("Job {} not found", job_id))?;
        
        let url_hash = self.calculate_hash(&job.source_url);
        let mut tracking = self.load_tracking_data(&url_hash).await
            .ok_or_else(|| anyhow::anyhow!("Tracking data not found for job {}", job_id))?;

        self.log(job_id, &format!("Réinitialisation du job à partir de l'étape : {}", step)).await;

        match step {
            "extraction" => {
                tracking.extraction = None;
                tracking.transcription = None;
                tracking.slide_detection = None;
                tracking.cleaned_slides = Vec::new();
                tracking.styled_slides = Vec::new();
                tracking.translation_segments = None;
                tracking.cloned_audio_urls = Vec::new();
                tracking.final_audio_url = None;
                tracking.final_video_url = None;
                tracking.pptx_url = None;
            }
            "transcription" => {
                tracking.transcription = None;
                tracking.translation_segments = None;
                tracking.cloned_audio_urls = Vec::new();
                tracking.final_audio_url = None;
                tracking.final_video_url = None;
            }
            "slide_detection" => {
                tracking.slide_detection = None;
                tracking.cleaned_slides = Vec::new();
                tracking.styled_slides = Vec::new();
                tracking.final_video_url = None;
                tracking.pptx_url = None;
            }
            "cleaning" => {
                tracking.cleaned_slides = Vec::new();
                tracking.styled_slides = Vec::new();
                tracking.refined_texts = Vec::new();
                tracking.translations.clear();
                tracking.cloned_audios.clear();
                tracking.cloned_durations.clear();
                tracking.final_audios.clear();
                tracking.final_videos.clear();
                tracking.final_video_url = None;
                tracking.pptx_url = None;
            }
            "styling" => {
                tracking.styled_slides = Vec::new();
                tracking.final_video_url = None;
                tracking.pptx_url = None;
            }
            "translation" => {
                tracking.translation_segments = None;
                tracking.cloned_audio_urls = Vec::new();
                tracking.final_audio_url = None;
                tracking.final_video_url = None;
            }
            "cloning" => {
                tracking.cloned_audio_urls = Vec::new();
                tracking.final_audio_url = None;
                tracking.final_video_url = None;
            }
            "composition" => {
                tracking.final_video_url = None;
            }
            _ => return Err(anyhow::anyhow!("Étape invalide : {}", step)),
        }

        self.save_tracking_data(&tracking).await?;
        
        // Reset job status to Pending or appropriate status to trigger restart
        self.job_repo.update_status(job_id, JobStatus::Pending).await?;
        
        Ok(())
    }

    pub async fn execute(&self, job_id: Uuid) -> Result<()> {
        let res = self.execute_internal(job_id).await;

        // Final Scale Down - Toujours scaler à 0 à la fin pour libérer les ressources
        let keep_workers_env = std::env::var("KERYX_DEBUG_KEEP_WORKERS").unwrap_or_default() == "true";
        
        if !keep_workers_env {
            self.log(job_id, "Phase finale : scale down de tous les services AI...").await;
            let services = vec![
                "keryx-extractor",
                "keryx-dewatermark",
                "keryx-voice-extractor",
                "keryx-texts-translation",
                "keryx-voices-cloner",
                "keryx-voice-cloner-gpt",
                "keryx-video-composer",
                "voices-composer",
                "keryx-video-generator",
                "keryx-diffusion-engine",
                "keryx-pptx-builder",
            ];
            for svc in services {
                let _ = self.scaling_repo.scale_down("keryx", svc).await;
            }
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

    async fn execute_dry_run(&self, job_id: Uuid) -> Result<()> {
        self.log(job_id, "🚀 Démarrage du mode DRY-RUN (Test d'orchestration séquentielle)").await;
        
        let services = vec![
            "keryx-extractor",
            "keryx-voice-extractor",
            "keryx-dewatermark",
            "keryx-diffusion-engine",
            "keryx-voice-cloner",
            "keryx-voice-cloner-gpt",
            "keryx-video-composer",
            "keryx-video-generator",
            "keryx-pptx-builder",
        ];

        for svc in services {
            self.log(job_id, &format!("--- Test Service : {} ---", svc)).await;
            
            self.log(job_id, &format!("Scaling UP {}...", svc)).await;
            self.scaling_repo.scale_up("keryx", svc).await?;
            
            self.log(job_id, &format!("Service {} est prêt. Attente de 3 secondes...", svc)).await;
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
            
            self.log(job_id, &format!("Scaling DOWN {}...", svc)).await;
            self.scaling_repo.scale_down("keryx", svc).await?;
            
            self.log(job_id, &format!("Service {} arrêté.", svc)).await;
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }

        self.log(job_id, "✅ Fin du mode DRY-RUN. Tous les services ont été testés avec succès.").await;
        self.job_repo.update_status(job_id, JobStatus::Completed).await?;
        
        Ok(())
    }

    async fn execute_internal(&self, job_id: Uuid) -> Result<()> {
        let dry_run = std::env::var("KERYX_DRY_RUN").unwrap_or_default() == "true";
        if dry_run {
            return self.execute_dry_run(job_id).await;
        }

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
            self.job_repo.update_status(job_id, JobStatus::Downloading).await?;
            let res = {
                let _guard = WorkerGuard::new(self.scaling_repo.clone(), "keryx", "keryx-extractor").await?;
                self.extractor.extract(&job.source_url, &job_id.to_string()).await?
            };
            self.log(job_id, "Extraction terminée.").await;
            
            tracking.extraction = Some(res.clone());
            self.save_tracking_data(&tracking).await?;
            res
        };
        let _ = self.job_repo.update_progress(job_id, 10.0).await;
        self.log(job_id, &format!("Phase 1 terminée. Titre: {}", ext_res.title)).await;

        // Phase 2 & 3 in Parallel: Transcription and Slide Detection
        self.log(job_id, "Phase 2 & 3 : Démarrage en parallèle (Transcription + Détection slides)...").await;
        
        // Phase 2 & 3 in Parallel: Transcription and Slide Detection
        self.log(job_id, "Phase 2 & 3 : Démarrage en parallèle (Transcription + Détection slides)...").await;
        
        let (trans_res, slide_res) = tokio::try_join!(
            self.ensure_transcription(job_id, &ext_res, &tracking),
            self.ensure_slide_detection(job_id, &ext_res, &tracking)
        )?;

        // Update tracking after parallel tasks
        let mut tracking_updated = false;
        if tracking.transcription.is_none() {
            tracking.transcription = Some(trans_res.clone());
            tracking_updated = true;
        }
        if tracking.slide_detection.is_none() {
            tracking.slide_detection = Some(slide_res.clone());
            tracking_updated = true;
        }
        if tracking_updated {
            self.save_tracking_data(&tracking).await?;
        }

        let _ = self.job_repo.update_progress(job_id, 30.0).await;
        self.log(job_id, &format!("Phase 2 terminée ({} segments). Phase 3 terminée ({} slides).", 
            trans_res.segments.len(), slide_res.slides.len())).await;

        self.log(job_id, &format!("Phase 2 terminée ({} segments). Phase 3 terminée ({} slides).", 
            trans_res.segments.len(), slide_res.slides.len())).await;

        /* 
        if tracking.cleaned_slides.len() < slide_res.slides.len() {
            let _perm = self.gpu_semaphore.acquire().await?;
            self.log(job_id, "Phase 3 : Nettoyage des slides via Dewatermark...").await;
            self.scaling_repo.scale_up("keryx", "keryx-dewatermark").await?;
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
        */
        // Bypass cleaning: use original images as "cleaned"
        if tracking.cleaned_slides.len() < slide_res.slides.len() {
            for slide in slide_res.slides.iter().skip(tracking.cleaned_slides.len()) {
                tracking.cleaned_slides.push(CleanedSlide {
                    index: slide.index,
                    original_url: slide.image_url.clone(),
                    cleaned_url: slide.image_url.clone(),
                    timestamp: slide.timestamp,
                });
            }
            self.save_tracking_data(&tracking).await?;
        }
        
        // FALLBACK: If no slides detected, create at least one virtual slide for the whole duration
        if tracking.cleaned_slides.is_empty() {
            self.log(job_id, "⚠️ Aucune slide détectée. Création d'une slide virtuelle par défaut.").await;
            tracking.cleaned_slides.push(CleanedSlide {
                index: 0,
                original_url: "https://raw.githubusercontent.com/JZacharie/Keryx/main/services/orchestrator/begin.mp4".to_string(), // Placeholder or first frame
                cleaned_url: "https://raw.githubusercontent.com/JZacharie/Keryx/main/services/orchestrator/begin.mp4".to_string(),
                timestamp: 0.0,
            });
            self.save_tracking_data(&tracking).await?;
        }
 
        /* 
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
        */
        // Bypass styling: use cleaned (original) as "styled"
        if tracking.styled_slides.len() < tracking.cleaned_slides.len() {
            for slide in tracking.cleaned_slides.iter().skip(tracking.styled_slides.len()) {
                tracking.styled_slides.push(StyledSlide {
                    index: slide.index,
                    original_url: slide.cleaned_url.clone(),
                    styled_url: slide.cleaned_url.clone(),
                    timestamp: slide.timestamp,
                });
            }
            self.save_tracking_data(&tracking).await?;
        }

        let mut slides_input = Vec::new();
        for i in 0..tracking.cleaned_slides.len() {
            let _duration = if i + 1 < tracking.cleaned_slides.len() {
                tracking.cleaned_slides[i+1].timestamp - tracking.cleaned_slides[i].timestamp
            } else {
                trans_res.duration - tracking.cleaned_slides[i].timestamp
            };
            slides_input.push(LocalSlideInput {
                image_url: tracking.cleaned_slides[i].cleaned_url.clone(),
            });
        }

        // --- Pipeline Multi-langue ---
        
        // 1. Fluidification (Une seule fois pour toutes les langues)
        if tracking.refined_texts.is_empty() && !tracking.cleaned_slides.is_empty() {
            let _perm = self.gpu_semaphore.acquire().await?;
            self.log(job_id, "Phase 4 : Fluidification du texte (Original Lang)...").await;
            
            let mut refined_list = Vec::new();
            {
                let _guard = WorkerGuard::new(self.scaling_repo.clone(), "keryx", "keryx-texts-translation").await?;
                for i in 0..tracking.cleaned_slides.len() {
                    let slide_start = tracking.cleaned_slides[i].timestamp;
                    let slide_end = if i + 1 < tracking.cleaned_slides.len() {
                        tracking.cleaned_slides[i+1].timestamp
                    } else {
                        trans_res.duration
                    };

                    let slide_text = trans_res.segments.iter()
                        .filter(|s| s.start >= slide_start && s.start < slide_end)
                        .map(|s| s.text.clone())
                        .collect::<Vec<_>>()
                        .join(" ");

                    if slide_text.trim().is_empty() {
                        refined_list.push("".to_string());
                        continue;
                    }

                    self.log(job_id, &format!("Fluidification slide {}/{}...", i+1, tracking.cleaned_slides.len())).await;
                    let refined_res = self.texts_translation.refine(&job_id.to_string(), &slide_text).await?;
                    refined_list.push(refined_res);
                }
            }
            tracking.refined_texts = refined_list;
            self.save_tracking_data(&tracking).await?;
        }
        let _ = self.job_repo.update_progress(job_id, 40.0).await;

        // 2. Traitement par Langue
        for lang in &job.target_langs {
            self.log(job_id, &format!("--- Traitement Langue : {} ---", lang)).await;

            // Phase 4 : Traduction
            if !tracking.translations.contains_key(lang) {
                let _perm = self.gpu_semaphore.acquire().await?;
                self.log(job_id, &format!("Phase 4 : Traduction en {}...", lang)).await;
                
                let mut lang_segments = Vec::new();
                {
                    let _guard = WorkerGuard::new(self.scaling_repo.clone(), "keryx", "keryx-texts-translation").await?;
                    for (i, text) in tracking.refined_texts.iter().enumerate() {
                        if text.is_empty() {
                            lang_segments.push(Segment { 
                                start: tracking.cleaned_slides[i].timestamp, 
                                end: tracking.cleaned_slides[i].timestamp, 
                                text: "".to_string(), 
                                translated: Some("".to_string()) 
                            });
                            continue;
                        }
                        let dummy = Segment { 
                            start: tracking.cleaned_slides[i].timestamp, 
                            end: 0.0, // sera ajusté par le service
                            text: text.clone(), 
                            translated: None 
                        };
                        let res = self.texts_translation.translate(&job_id.to_string(), vec![dummy], lang).await?;
                        if let Some(s) = res.first() {
                            lang_segments.push(s.clone());
                        }
                    }
                }
                tracking.translations.insert(lang.clone(), lang_segments);
                self.save_tracking_data(&tracking).await?;
            }
            let _ = self.job_repo.update_progress(job_id, 60.0).await;

            // Phase 5 : Clonage Vocal
            if !tracking.cloned_audios.contains_key(lang) {
                let _perm = self.gpu_semaphore.acquire().await?;
                self.log(job_id, &format!("Phase 5 : Clonage vocal en {}...", lang)).await;
                
                let mut lang_audios = Vec::new();
                let mut lang_durations = Vec::new();
                let segments = tracking.translations.get(lang).unwrap();
                {
                    let _guard = WorkerGuard::new(self.scaling_repo.clone(), "keryx", "keryx-voices-cloner").await?;
                    for (i, seg) in segments.iter().enumerate() {
                        let text = seg.translated.clone().unwrap_or_else(|| seg.text.clone());
                        if text.trim().is_empty() {
                            lang_audios.push("".to_string());
                            lang_durations.push(1.0); // Fallback 1s pour les slides vides
                            continue;
                        }
                        self.log(job_id, &format!("Génération audio {} slide {}/{}...", lang, i+1, segments.len())).await;
                        let res = self.voice_cloner.perform_cloning(&text, lang, &ext_res.audio_url, &job_id.to_string()).await?;
                        lang_audios.push(res.url);
                        let dur: f64 = res.duration.parse().unwrap_or(2.0);
                        lang_durations.push(dur);
                    }
                }
                tracking.cloned_audios.insert(lang.clone(), lang_audios);
                tracking.cloned_durations.insert(lang.clone(), lang_durations);
                self.save_tracking_data(&tracking).await?;
            }
            let _ = self.job_repo.update_progress(job_id, 80.0).await;

            // Phase 6 : Composition Finale (Audio + Vidéo)
            if !tracking.final_videos.contains_key(lang) {
                let _perm = self.gpu_semaphore.acquire().await?;
                self.log(job_id, &format!("Phase 6 : Composition finale en {}...", lang)).await;
                
                // Audio
                let audios = tracking.cloned_audios.get(lang).unwrap();
                let concat_res = {
                    let _guard = WorkerGuard::new(self.scaling_repo.clone(), "keryx", "voices-composer").await?;
                    self.voices_composer.concat_audio(&job_id.to_string(), audios.clone()).await?
                };
                tracking.final_audios.insert(lang.clone(), concat_res.url.clone());

                // Vidéo
                let durations = tracking.cloned_durations.get(lang).expect("Missing cloned_durations");
                let mut composer_slides = Vec::new();
                for (i, slide) in tracking.styled_slides.iter().enumerate() {
                    composer_slides.push(ComposerSlideInput {
                        image_url: slide.styled_url.clone(),
                        duration: durations[i], // Utilise la durée du clone audio pour cette langue
                    });
                }

                let res = {
                    let _guard = WorkerGuard::new(self.scaling_repo.clone(), "keryx", "keryx-video-composer").await?;
                    self.video_composer.compose(
                        &job_id.to_string(), 
                        composer_slides, 
                        Some(concat_res.url)
                    ).await?
                };
                
                tracking.final_videos.insert(lang.clone(), res.url);
                self.save_tracking_data(&tracking).await?;
            }
            let _ = self.job_repo.update_progress(job_id, 95.0).await;
        }

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
            
            let pptx_slides = tracking.styled_slides.iter().map(|s| {
                PptxSlide {
                    image_url: s.styled_url.clone(),
                    text: "".to_string(), // On pourrait ajouter les notes ici
                }
            }).collect();
            
            let build_res = {
                let _guard = WorkerGuard::new(self.scaling_repo.clone(), "keryx", "keryx-pptx-builder").await?;
                self.pptx_builder.build_pptx(&job_id.to_string(), pptx_slides, None).await
            };

            match build_res {
                Ok(pptx_res) => {
                    tracking.pptx_url = Some(pptx_res.url);
                    self.save_tracking_data(&tracking).await?;
                    self.log(job_id, "Génération PPTX terminée.").await;
                },
                Err(e) => {
                    self.log(job_id, &format!("⚠️ Erreur lors de la génération PPTX (non-fatale): {}", e)).await;
                }
            }
        }

        // Phase 7 : Notification Slack finale
        let mut audio_links = String::new();
        for (lang, url) in &tracking.final_audios {
            audio_links.push_str(&format!("\n• *{}*: <{}|Listen Audio 🔊>", lang, url));
        }

        let slack_msg = format!(
            "📽️ *Job Concluded: {}*\n\n✅ *Status:* COMPLETED\n🌍 *Languages:* {}{}",
            job_id, job.target_langs.join(", "), audio_links
        );
        let _ = self.notification_repo.notify_slack(&slack_msg).await;

        let _ = self.job_repo.update_progress(job_id, 100.0).await;
        self.job_repo.update_status(job_id, JobStatus::Completed).await?;
        self.log(job_id, "✅ Job distribué terminé avec succès.").await;

        // Cleanup final : S'assurer que tous les services sont à 0 pour libérer les ressources
        let services = vec![
            "keryx-extractor",
            "keryx-voice-extractor",
            "keryx-dewatermark",
            "keryx-diffusion-engine",
            "keryx-voice-cloner",
            "keryx-voice-cloner-gpt",
            "keryx-video-composer",
            "keryx-video-generator",
            "keryx-pptx-builder",
        ];
        for svc in services {
            let _ = self.scaling_repo.scale_down("keryx", svc).await;
        }

        Ok(())
    }

    async fn ensure_transcription(
        &self, 
        job_id: Uuid, 
        ext_res: &crate::infrastructure::clients::extractor::ExtractResponse,
        tracking: &JobTrackingData
    ) -> Result<crate::infrastructure::clients::voice_extractor::TranscribeResponse> {
        if let Some(existing) = tracking.transcription.clone() {
            return Ok(existing);
        }

        let _perm = self.gpu_semaphore.acquire().await?;
        self.log(job_id, "Transcription (Whisper) en cours...").await;
        self.job_repo.update_status(job_id, JobStatus::Transcribing).await?;

        let res = {
            let _guard = WorkerGuard::new(self.scaling_repo.clone(), "keryx", "keryx-voice-extractor").await?;
            self.voice_extractor.perform_transcription(&ext_res.audio_url, &job_id.to_string(), None).await?
        };

        Ok(res)
    }

    async fn ensure_slide_detection(
        &self, 
        job_id: Uuid, 
        ext_res: &crate::infrastructure::clients::extractor::ExtractResponse,
        tracking: &JobTrackingData
    ) -> Result<crate::infrastructure::clients::video_composer::DetectSlidesResponse> {
        if let Some(existing) = tracking.slide_detection.clone() {
            return Ok(existing);
        }

        self.log(job_id, "Détection des slides (FFmpeg) en cours...").await;
        self.job_repo.update_status(job_id, JobStatus::Analyzing).await?;

        let res = {
            let _guard = WorkerGuard::new(self.scaling_repo.clone(), "keryx", "keryx-video-composer").await?;
            self.video_composer.detect_slides(&job_id.to_string(), &ext_res.video_url).await?
        };

        Ok(res)
    }
}

// Helper struct temporarily here to store timestamp while calculating duration
struct LocalSlideInput {
    image_url: String,
}
