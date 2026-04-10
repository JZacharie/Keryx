use std::sync::Arc;
use axum::{
    routing::{get, post},
    Router,
};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use keryx_ingestor::{
    state::AppState,
    interfaces::http::job_handlers::{create_job_handler, get_job_handler},
    application::use_cases::ingest_video::IngestVideoUseCase,
    infrastructure::repositories::{
        redis_job_repository::RedisJobRepository,
        s3_storage_repository::S3StorageRepository,
        yt_dlp_repository::YtDlpRepository,
        ffmpeg_analyzer::FfmpegAnalyzer,
        whisper_stt_repository::WhisperSTTRepository,
        ollama_translator_repository::OllamaTranslatorRepository,
        diffusion_stylizer_repository::DiffusionStylizerRepository,
        pptx_builder_repository::PptxBuilderRepository,
        kube_scaling_repository::KubeScalingRepository,
        qwen_tts_repository::QwenTTSRepository,
        coqui_voice_cloner_repository::CoquiVoiceClonerRepository,
        ffmpeg_reconstructor::FfmpegReconstructor,
        slack_notification_repository::SlackNotificationRepository,
    },
};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use tracing_subscriber::{fmt, EnvFilter, prelude::*};

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    // Fix for rustls 0.23: explicitly install crypto provider
    #[allow(clippy::single_component_path_imports)]
    use rustls;
    let _ = rustls::crypto::ring::default_provider().install_default();

    // Configuration
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());
    let s3_bucket = std::env::var("S3_BUCKET").unwrap_or_else(|_| "keryx".to_string());
    let s3_region = std::env::var("S3_REGION").unwrap_or_else(|_| "us-east-1".to_string());
    let s3_endpoint = std::env::var("S3_ENDPOINT").ok();
    let diffusion_url = std::env::var("DIFFUSION_URL").unwrap_or_else(|_| "http://keryx-diffusion-engine".to_string());
    let whisper_url = std::env::var("WHISPER_URL").unwrap_or_else(|_| "http://192.168.0.194:9000".to_string());
    let ollama_url = std::env::var("OLLAMA_URL").unwrap_or_else(|_| "http://192.168.0.191:11434".to_string());
    let pptx_url = std::env::var("PPTX_URL").unwrap_or_else(|_| "http://keryx-pptx-builder:8002".to_string());
    let tts_url = std::env::var("TTS_URL").unwrap_or_else(|_| "http://qwen3-tts.qwen-tts.svc.cluster.local:7860".to_string());
    let voice_cloner_url = std::env::var("VOICE_CLONER_URL").unwrap_or_else(|_| "http://voice-cloner.keryx.svc.cluster.local:9880".to_string());
    let slack_webhook = std::env::var("SLACK_WEBHOOK_URL").unwrap_or_else(|_| "https://hooks.slack.com/services/T01234567/B01234567/XXXXXXXX".to_string());

    let temp_dir = PathBuf::from("/tmp/keryx");
    std::fs::create_dir_all(&temp_dir)?;

    // Initialize repositories
    tracing::info!("Starting repository initialization...");
    let job_repo = Arc::new(RedisJobRepository::new(&redis_url)?);
    tracing::debug!("Job repository (Redis) initialized.");

    let storage_repo = Arc::new(S3StorageRepository::new(&s3_region, &s3_bucket, s3_endpoint.as_deref()).await);
    tracing::debug!("Storage repository (S3) initialized.");

    let downloader = Arc::new(YtDlpRepository::new(temp_dir.clone()));
    let analyzer = Arc::new(FfmpegAnalyzer::new(temp_dir.clone()));
    let stt_repo = Arc::new(WhisperSTTRepository::new(&whisper_url));
    let translator = Arc::new(OllamaTranslatorRepository::new(&ollama_url, "llama3"));
    let stylizer = Arc::new(DiffusionStylizerRepository::new(diffusion_url));
    let pptx_repo = Arc::new(PptxBuilderRepository::new(pptx_url));

    tracing::info!("Initializing KubeScalingRepository...");
    let scaling_repo = Arc::new(KubeScalingRepository::new().await?);
    tracing::debug!("KubeScalingRepository initialized successfully.");

    let tts_repo = Arc::new(QwenTTSRepository::new(tts_url));
    let voice_cloner_repo = Arc::new(CoquiVoiceClonerRepository::new(voice_cloner_url));
    let reconstructor = Arc::new(FfmpegReconstructor::new());
    let notification_repo = Arc::new(SlackNotificationRepository::new(slack_webhook));
    tracing::info!("All repositories initialized successfully.");

    // Initialize use cases
    let ingest_video_use_case = Arc::new(IngestVideoUseCase::new(
        job_repo,
        storage_repo,
        downloader,
        analyzer,
        stt_repo,
        translator,
        stylizer,
        pptx_repo,
        scaling_repo,
        tts_repo,
        voice_cloner_repo,
        reconstructor,
        notification_repo,
    ));

    // Initialize state
    let state = AppState {
        ingest_video_use_case,
    };

    // Build routes
    let app = Router::new()
        .route("/health", get(|| async { "OK" }))
        .route("/api/jobs", post(create_job_handler))
        .route("/api/jobs/:id", get(get_job_handler))
        .fallback_service(tower_http::services::ServeDir::new("static").fallback(tower_http::services::ServeFile::new("static/index.html")))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Start server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    tracing::info!("Server listening on http://{}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}
