mod domain;
mod application;
mod infrastructure;
mod interfaces;

use axum::{routing::{get, post}, Router, middleware::from_fn_with_state};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::services::ServeDir;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::infrastructure::auth::verifier::JwtVerifier;
use crate::infrastructure::repositories::s3_job_repository::S3JobRepository;
use crate::domain::ports::job_repository::JobRepository;
use crate::interfaces::http::middleware::auth::auth_middleware;
use crate::interfaces::http::job_handlers::create_job_handler;

#[derive(Clone)]
pub struct AppState {
    pub jwt_verifier: Arc<JwtVerifier>,
    pub job_repository: Arc<dyn JobRepository>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load JWT Public Key
    let public_key_pem = std::env::var("JWT_PUBLIC_KEY")
        .unwrap_or_else(|_| include_str!("../test_public_key.pem").to_string());
    
    let jwt_verifier = Arc::new(JwtVerifier::new(&public_key_pem)?);
    
    // S3 Config
    let s3_bucket = std::env::var("S3_BUCKET").unwrap_or_else(|_| "keryx-jobs".to_string());
    let s3_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let s3_client = aws_sdk_s3::Client::new(&s3_config);
    let job_repository = Arc::new(S3JobRepository::new(s3_client, s3_bucket));

    let state = AppState { 
        jwt_verifier,
        job_repository,
    };

    // Routes
    let app = Router::new()
        .nest_service("/", ServeDir::new("static"))
        .route("/health", get(interfaces::http::health::health_check))
        .nest("/api", 
            Router::new()
                .route("/secure-ping", get(interfaces::http::health::health_check))
                .route("/jobs", post(create_job_handler))
                .layer(from_fn_with_state(state.jwt_verifier.clone(), auth_middleware))
        )
        .with_state(state);

    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()
        .expect("PORT must be a number");

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
