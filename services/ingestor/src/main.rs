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
    },
};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    // Configuration
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());
    let s3_bucket = std::env::var("S3_BUCKET").unwrap_or_else(|_| "keryx-raw".to_string());
    let s3_region = std::env::var("S3_REGION").unwrap_or_else(|_| "eu-west-1".to_string());
    let temp_dir = PathBuf::from("/tmp/keryx");
    std::fs::create_dir_all(&temp_dir)?;

    // Initialize repositories
    let job_repo = Arc::new(RedisJobRepository::new(&redis_url)?);
    let storage_repo = Arc::new(S3StorageRepository::new(&s3_region, &s3_bucket, None).await);
    let downloader = Arc::new(YtDlpRepository::new(temp_dir.clone()));
    let analyzer = Arc::new(FfmpegAnalyzer::new(temp_dir.clone()));

    // Initialize use cases
    let ingest_video_use_case = Arc::new(IngestVideoUseCase::new(
        job_repo,
        storage_repo,
        downloader,
        analyzer,
    ));

    // Initialize state
    let state = AppState {
        ingest_video_use_case,
    };

    // Build routes
    let app = Router::new()
        .route("/api/jobs", post(create_job_handler))
        .route("/api/jobs/:id", get(get_job_handler))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Start server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    tracing::info!("Server listening on http://{}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}
