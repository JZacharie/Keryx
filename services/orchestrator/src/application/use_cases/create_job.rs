use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

use crate::domain::entities::job::{Job, JobStatus};
use crate::domain::ports::job_repository::JobRepository;
use crate::infrastructure::docker::{ContainerManager, WorkerSpec};

#[derive(Debug, Deserialize)]
pub struct CreateJobInput {
    #[serde(alias = "video_url")]
    pub media_url: String,
    #[serde(alias = "target_langs")]
    pub language: serde_json::Value,
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

        let lang_str = match &input.language {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Array(arr) => {
                if let Some(serde_json::Value::String(s)) = arr.first() {
                    s.clone()
                } else {
                    "en".to_string()
                }
            }
            _ => "en".to_string(),
        };

        tracing::info!(
            "=================================================================================="
        );
        tracing::info!(
            "PIPELINE START | job={} | media={} | lang={}",
            job_id,
            input.media_url,
            lang_str
        );
        tracing::info!(
            "=================================================================================="
        );

        let base_env = self.build_env();

        // Phase 1: Extractor (downloads video, no GPU)
        job.status = JobStatus::Downloading;
        self.repository.save(&job).await?;
        let extract_res = self.run_worker(
            "keryx-extractor",
            "Downloading",
            false,
            &base_env,
            "/extract",
            serde_json::json!({
                "url": input.media_url,
                "job_id": job_id
            })
        ).await?;

        let audio_url = extract_res["audio_url"].as_str()
            .ok_or_else(|| anyhow::anyhow!("extractor response missing audio_url"))?.to_string();
        let video_url = extract_res["video_url"].as_str()
            .ok_or_else(|| anyhow::anyhow!("extractor response missing video_url"))?.to_string();
        let video_duration = extract_res["duration"].as_f64().unwrap_or(0.0);

        // Phase 2: Voice Extractor (Whisper STT, GPU)
        job.status = JobStatus::Transcribing;
        self.repository.save(&job).await?;
        let transcribe_res = self.run_worker(
            "keryx-voice-extractor",
            "Transcribing",
            true,
            &base_env,
            "/transcribe",
            serde_json::json!({
                "audio_url": audio_url,
                "job_id": job_id,
                "language": lang_str
            })
        ).await?;

        let segments = transcribe_res["segments"].clone();

        // Phase 3: Video Composer (slide detection, GPU for encoding)
        job.status = JobStatus::Analyzing;
        self.repository.save(&job).await?;
        let detect_slides_res = self.run_worker(
            "keryx-video-composer",
            "Analyzing",
            true,
            &base_env,
            "/detect_slides",
            serde_json::json!({
                "job_id": job_id,
                "video_url": video_url,
                "scene_threshold": 0.3
            })
        ).await?;

        let slides = detect_slides_res["slides"].as_array()
            .ok_or_else(|| anyhow::anyhow!("video-composer response missing slides"))?.clone();

        // Phase 3B: Dewatermark (GPU)
        let dewatermark_res = self.run_worker(
            "keryx-dewatermark",
            "Dewatermark",
            true,
            &base_env,
            "/clean/video",
            serde_json::json!({
                "video_url": video_url,
                "job_id": job_id
            })
        ).await?;

        let cleaned_video_url = dewatermark_res["url"].as_str()
            .ok_or_else(|| anyhow::anyhow!("dewatermark response missing url"))?.to_string();

        // Phase 4: Texts Translation (no GPU)
        job.status = JobStatus::Translating;
        self.repository.save(&job).await?;
        let translate_res = self.run_worker(
            "keryx-texts-translation",
            "Translating",
            false,
            &base_env,
            "/translate",
            serde_json::json!({
                "segments": segments,
                "target_lang": lang_str,
                "job_id": job_id
            })
        ).await?;

        let translated_segments = translate_res["segments"].as_array()
            .ok_or_else(|| anyhow::anyhow!("texts-translation response missing segments"))?.clone();

        // Phase 4B: Voices Cloner (GPU)
        job.status = JobStatus::CloningVoice;
        self.repository.save(&job).await?;
        
        let mut cloned_audios = Vec::new();
        
        let mut cloner_volumes = Vec::new();
        let mut cloner_env = base_env.clone();
        if let Ok(host_shared_dir) = std::env::var("HOST_SHARED_DIR") {
            cloner_volumes.push((host_shared_dir, "/data".to_string()));
            cloner_env.push("SHARED_DATA_DIR=/data".to_string());
        }
        
        let cloner_spec = WorkerSpec {
            image: "keryx-voices-cloner".to_string(),
            container_name: format!("keryx-voices-cloner-{}", uuid::Uuid::new_v4()),
            env: cloner_env,
            needs_gpu: true,
            port: 8000,
            volumes: cloner_volumes,
        };
        
        tracing::info!("[{}] >>> Starting voices cloner worker...", cloner_spec.container_name);
        let cloner_handle = self.container_manager.start_worker(&cloner_spec).await?;
        
        for (i, seg) in translated_segments.iter().enumerate() {
            let text = seg["translated"].as_str().unwrap_or("");
            if text.trim().is_empty() {
                continue;
            }
            tracing::info!("[{}] Cloning segment {}/{}...", cloner_spec.container_name, i + 1, translated_segments.len());
            let clone_res = self.container_manager.call_worker(
                &cloner_spec.container_name,
                8000,
                "/clone",
                serde_json::json!({
                    "text": text,
                    "language": "en",
                    "reference_url": audio_url,
                    "job_id": job_id,
                    "output_key": format!("{}/voices-cloner/seg_{}.wav", job_id, i)
                })
            ).await?;
            if let Some(url) = clone_res["url"].as_str() {
                cloned_audios.push(url.to_string());
            }
        }
        self.container_manager.stop_worker(&cloner_handle).await?;
        drop(cloner_handle);

        // Phase 5: Final Video Composer (GPU)
        job.status = JobStatus::Composing;
        self.repository.save(&job).await?;
        
        let mut composer_volumes = Vec::new();
        let mut composer_env = base_env.clone();
        if let Ok(host_shared_dir) = std::env::var("HOST_SHARED_DIR") {
            composer_volumes.push((host_shared_dir, "/data".to_string()));
            composer_env.push("SHARED_DATA_DIR=/data".to_string());
        }
        
        let composer_spec = WorkerSpec {
            image: "keryx-video-composer".to_string(),
            container_name: format!("keryx-video-composer-{}", uuid::Uuid::new_v4()),
            env: composer_env,
            needs_gpu: true,
            port: 8000,
            volumes: composer_volumes,
        };
        let composer_handle = self.container_manager.start_worker(&composer_spec).await?;

        tracing::info!("[{}] Concatenating cloned audios...", composer_spec.container_name);
        let concat_res = self.container_manager.call_worker(
            &composer_spec.container_name,
            8000,
            "/concat_audio",
            serde_json::json!({
                "job_id": job_id,
                "segments": cloned_audios,
                "output_key": format!("{}/video-composer/audio/merged.wav", job_id)
            })
        ).await?;
        let merged_audio_url = concat_res["url"].as_str()
            .ok_or_else(|| anyhow::anyhow!("concat_audio response missing url"))?.to_string();

        let mut slide_inputs = Vec::new();
        for (i, slide) in slides.iter().enumerate() {
            let image_url = slide["image_url"].as_str().unwrap_or("").to_string();
            let timestamp = slide["timestamp"].as_f64().unwrap_or(0.0);
            
            let duration = if i + 1 < slides.len() {
                let next_timestamp = slides[i + 1]["timestamp"].as_f64().unwrap_or(video_duration);
                next_timestamp - timestamp
            } else {
                video_duration - timestamp
            };
            
            slide_inputs.push(serde_json::json!({
                "image_url": image_url,
                "duration": duration
            }));
        }

        tracing::info!("[{}] Composing final video...", composer_spec.container_name);
        let compose_res = self.container_manager.call_worker(
            &composer_spec.container_name,
            8000,
            "/compose",
            serde_json::json!({
                "job_id": job_id,
                "slides": slide_inputs,
                "audio_url": merged_audio_url,
                "output_key": format!("{}/video-composer/output.mp4", job_id)
            })
        ).await?;
        
        self.container_manager.stop_worker(&composer_handle).await?;

        let final_video_url = compose_res["url"].as_str()
            .ok_or_else(|| anyhow::anyhow!("compose response missing url"))?.to_string();

        job.status = JobStatus::Completed;
        self.repository.save(&job).await?;

        let elapsed = pipeline_start.elapsed();
        tracing::info!(
            "=================================================================================="
        );
        tracing::info!(
            "PIPELINE COMPLETE | job={} | duration={:.1}s | final_video={}",
            job_id,
            elapsed.as_secs_f64(),
            final_video_url
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
        path: &str,
        body: serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let container_name = format!("{}-{}", image, uuid::Uuid::new_v4());
        let worker_start = Instant::now();

        let mut env = base_env.to_vec();
        let mut volumes = Vec::new();
        if let Ok(host_shared_dir) = std::env::var("HOST_SHARED_DIR") {
            volumes.push((host_shared_dir, "/data".to_string()));
            env.push("SHARED_DATA_DIR=/data".to_string());
        }

        let spec = WorkerSpec {
            image: image.to_string(),
            container_name: container_name.clone(),
            env,
            needs_gpu,
            port: 8000,
            volumes,
        };

        tracing::info!("[{}] >>> Starting phase: {} (image={}, gpu={})", container_name, phase, image, needs_gpu);
        match self.container_manager.start_worker(&spec).await {
            Ok(handle) => {
                let start_elapsed = worker_start.elapsed();
                tracing::info!("[{}] <<< {} ready in {:.1}s", container_name, phase, start_elapsed.as_secs_f64());

                tracing::info!("[{}] Calling endpoint {}...", container_name, path);
                let response = self.container_manager.call_worker(&container_name, 8000, path, body).await;

                self.container_manager.stop_worker(&handle).await?;

                let total_elapsed = worker_start.elapsed();
                tracing::info!("[{}] === Phase {} completed in {:.1}s ===", container_name, phase, total_elapsed.as_secs_f64());
                response
            }
            Err(e) => {
                tracing::error!("[{}] Phase {} FAILED: {}", container_name, phase, e);
                Err(e)
            }
        }
    }
}
