use std::sync::Arc;
use std::io::Write;
use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use keryx_orchestrator::{
    state::AppState,
    interfaces::http::job_handlers::{create_job_handler, get_job_handler, list_jobs_handler},
    interfaces::http::log_handlers::{get_job_logs_sse_handler, get_job_logs_raw_handler},
    application::use_cases::ingest_video::IngestVideoUseCase,
    infrastructure::{
        auth_middleware::{require_api_key, AuthState},
        repositories::{
            redis_job_repository::RedisJobRepository,
            s3_storage_repository::S3StorageRepository,
            kube_scaling_repository::KubeScalingRepository,
            compose_scaling_repository::ComposeScalingRepository,
            slack_notification_repository::SlackNotificationRepository,
        },
        clients::{
            extractor::ExtractorClient,
            dewatermark::DewatermarkClient,
            voice_extractor::VoiceExtractorClient,
            voice_cloner::VoiceClonerClient,
            video_composer::VideoComposerClient,
            video_generator::VideoGeneratorClient,
        },
    },
};

mod telemetry;


#[tokio::main]
async fn main() {
    // RAW HELLO - No runtime needed for this
    println!(">>> KERYX ORCHESTRATOR: RAW HELLO FROM MAIN!");
    let _ = std::io::stdout().flush();

    println!(">>> KERYX ORCHESTRATOR: Starting process...");
    let _ = std::io::stdout().flush();

    // --- OpenTelemetry : init traces + métriques (optionnel) ---
    let otel_endpoint = std::env::var("OTEL_ENDPOINT").ok();
    let otel_auth_token = std::env::var("OTEL_AUTH_TOKEN").ok();
    let otel_service_name = std::env::var("OTEL_SERVICE_NAME")
        .unwrap_or_else(|_| "keryx-orchestrator".to_string());

    // _otel_guard doit vivre jusqu'à la fin du main (flush au Drop)
    let _otel_guard = telemetry::init_telemetry(
        &otel_service_name,
        otel_endpoint.as_deref(),
        otel_auth_token.as_deref()
    );

    if let Some(ep) = otel_endpoint {
        tracing::info!("OTel initialized: service={} endpoint={}", otel_service_name, ep);
    } else {
        println!(">>> KERYX ORCHESTRATOR: OTel disabled (OTEL_ENDPOINT not set)");
    }

    println!(">>> KERYX ORCHESTRATOR: Initializing runtime...");
    let _ = std::io::stdout().flush();

    if let Err(e) = run().await {
        eprintln!(">>> FATAL ERROR IN RUN: {:?}", e);
        tracing::error!("FATAL ERROR: {:?}", e);
        let _ = std::io::stderr().flush();
        std::process::exit(1);
    }
}


async fn run() -> anyhow::Result<()> {
    // Check for minimalist test mode
    if std::env::var("ORCHESTRATOR_TEST").unwrap_or_default() == "true" {
        println!(">>> KERYX ORCHESTRATOR: MINIMALIST TEST MODE ACTIVE! Sleeping 1 hour...");
        let _ = std::io::stdout().flush();
        tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
        return Ok(());
    }

    // Fix for rustls 0.23: explicitly install crypto provider
    let _ = rustls::crypto::ring::default_provider().install_default();

    // Configuration
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());
    let s3_bucket = std::env::var("S3_BUCKET").unwrap_or_else(|_| "keryx".to_string());
    let s3_region = std::env::var("S3_REGION").unwrap_or_else(|_| "us-east-1".to_string());
    let s3_endpoint = std::env::var("S3_ENDPOINT").ok();

    // Service URLs
    let extractor_url = std::env::var("EXTRACTOR_URL").unwrap_or_else(|_| "http://keryx-extractor:8000".to_string());
    let dewatermark_url = std::env::var("DEWATERMARK_URL").unwrap_or_else(|_| "http://keryx-dewatermark:8000".to_string());
    let voice_extractor_url = std::env::var("VOICE_EXTRACTOR_URL").unwrap_or_else(|_| "http://keryx-voice-extractor:8000".to_string());
    let video_composer_url = std::env::var("VIDEO_COMPOSER_URL").unwrap_or_else(|_| "http://keryx-video-composer:8000".to_string());
    let video_generator_url = std::env::var("VIDEO_GENERATOR_URL").unwrap_or_else(|_| "http://keryx-wan2gp:8000".to_string());
    let voice_cloner_url = std::env::var("VOICE_CLONER_URL").unwrap_or_else(|_| "http://keryx-voice-cloner:8000".to_string());

    let slack_webhook = std::env::var("SLACK_WEBHOOK_URL").unwrap_or_else(|_| "https://hooks.slack.com/services/T01234567/B01234567/XXXXXXXX".to_string());

    let api_key = std::env::var("API_KEY").unwrap_or_else(|_| "changeme".to_string());

    // Initialize core repositories
    let job_repo = Arc::new(RedisJobRepository::new(&redis_url)?);
    let storage_repo = Arc::new(S3StorageRepository::new(&s3_region, &s3_bucket, s3_endpoint.as_deref()).await);

    let scaling_mode = std::env::var("SCALING_MODE").unwrap_or_else(|_| "kube".to_string());
    let scaling_repo: Arc<dyn keryx_core::domain::ports::scaling_repository::ScalingRepository> = if scaling_mode == "compose" {
        tracing::info!("Using Docker Compose scaling mode");
        Arc::new(ComposeScalingRepository::new()?)
    } else {
        tracing::info!("Using Kubernetes scaling mode");
        Arc::new(KubeScalingRepository::new().await?)
    };

    let notification_repo = Arc::new(SlackNotificationRepository::new(slack_webhook));

    // Initialize HTTP Clients
    let extractor = Arc::new(ExtractorClient::new(extractor_url));
    let dewatermark = Arc::new(DewatermarkClient::new(dewatermark_url));
    let voice_extractor = Arc::new(VoiceExtractorClient::new(voice_extractor_url));
    let voice_cloner = Arc::new(VoiceClonerClient::new(voice_cloner_url));
    let video_composer = Arc::new(VideoComposerClient::new(video_composer_url));
    let video_generator = Arc::new(VideoGeneratorClient::new(video_generator_url));

    // Initialize use case
    let ingest_video_use_case = Arc::new(IngestVideoUseCase::new(
        job_repo,
        storage_repo,
        scaling_repo,
        notification_repo,
        extractor.clone(),
        dewatermark.clone(),
        voice_extractor.clone(),
        voice_cloner.clone(),
        video_composer.clone(),
        video_generator.clone(),
    ));

    // Initialize state
    let state = AppState {
        ingest_video_use_case,
        extractor,
        dewatermark,
        voice_extractor,
        voice_cloner,
        video_composer,
        video_generator,
    };

    let auth_state = AuthState { api_key };

    let public_routes = Router::new()
        .route("/health", get(|| async { "OK" }))
        .route("/api/jobs", get(list_jobs_handler))
        .route("/api/jobs/:id", get(get_job_handler))
        .route("/api/jobs/:id/logs", get(get_job_logs_sse_handler))
        .route("/api/jobs/:id/logs/raw", get(get_job_logs_raw_handler));

    let protected_routes = Router::new()
        .route("/api/jobs", post(create_job_handler))
        .layer(middleware::from_fn_with_state(auth_state, require_api_key));

    let app = Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .fallback_service(tower_http::services::ServeDir::new("static").fallback(tower_http::services::ServeFile::new("static/index.html")))
        .layer(
            CorsLayer::permissive()
                .allow_methods([
                    axum::http::Method::GET,
                    axum::http::Method::POST,
                    axum::http::Method::OPTIONS,
                ])
                .allow_headers([
                    axum::http::header::AUTHORIZATION,
                    axum::http::header::CONTENT_TYPE,
                ])
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    println!(">>> KERYX ORCHESTRATOR: Server listening on http://{}", listener.local_addr()?);
    tracing::info!("orchestrator listening on http://{}", listener.local_addr()?);
    let _ = std::io::stdout().flush();

    // Start server
    let server_handle = axum::serve(listener, app);
    println!(">>> KERYX ORCHESTRATOR: Server handle created, awaiting...");
    let _ = std::io::stdout().flush();

    match server_handle.await {
        Ok(_) => {
            println!(">>> KERYX ORCHESTRATOR: axum::serve finished with OK.");
            tracing::warn!("Server shutdown normally.");
        },
        Err(e) => {
            println!(">>> KERYX ORCHESTRATOR: axum::serve finished with ERROR: {:?}", e);
            tracing::error!("Server error: {:?}", e);
        }
    }

    println!(">>> KERYX ORCHESTRATOR: run() is exiting. This is unexpected for a long-running service.");
    let _ = std::io::stdout().flush();

    Ok(())
}
