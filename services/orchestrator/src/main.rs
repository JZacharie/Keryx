mod domain;
mod application;
mod infrastructure;
mod interfaces;

use axum::{routing::get, Router, middleware::from_fn_with_state};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use crate::infrastructure::auth::verifier::JwtVerifier;
use crate::interfaces::http::middleware::auth::auth_middleware;

#[derive(Clone)]
pub struct AppState {
    pub jwt_verifier: Arc<JwtVerifier>,
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
    let state = AppState { jwt_verifier };

    // Routes
    let app = Router::new()
        .route("/health", get(interfaces::http::health::health_check))
        .nest("/api", 
            Router::new()
                .route("/secure-ping", get(interfaces::http::health::health_check))
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
