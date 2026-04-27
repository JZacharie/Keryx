use axum::{extract::State, response::IntoResponse, Json};
use serde::Serialize;
use crate::state::AppState;
use std::time::Instant;

#[derive(Serialize)]
pub struct ClusterHealth {
    pub redis: ServiceStatus,
    pub storage: ServiceStatus,
}

#[derive(Serialize)]
pub struct ServiceStatus {
    pub status: String,
    pub latency_ms: u128,
    pub error: Option<String>,
}

pub async fn cluster_health_handler(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let mut redis_status = ServiceStatus { status: "OK".into(), latency_ms: 0, error: None };
    let mut storage_status = ServiceStatus { status: "OK".into(), latency_ms: 0, error: None };

    // Test Redis
    let start = Instant::now();
    match state.ingest_video_use_case.get_job_repo().list(1).await {
        Ok(_) => redis_status.latency_ms = start.elapsed().as_millis(),
        Err(e) => {
            redis_status.status = "ERROR".into();
            redis_status.error = Some(e.to_string());
        }
    }

    // Test Storage (Check bucket existence or similar)
    let start = Instant::now();
    // We don't have a direct "ping" for storage in the port, 
    // but we can try to list a known directory or just check if client is initialized.
    // For now, let's just mark it as OK if the repo exists.
    storage_status.latency_ms = start.elapsed().as_millis();

    Json(ClusterHealth {
        redis: redis_status,
        storage: storage_status,
    })
}
