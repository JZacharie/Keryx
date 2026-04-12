use axum::{
    extract::{State, Path},
    response::IntoResponse,
    Json,
};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use keryx_core::domain::entities::job::{Job, JobStatus, StyleConfig};

#[derive(Deserialize)]
pub struct CreateJobRequest {
    pub video_url: String,
    pub target_langs: Vec<String>,
    pub prompt: Option<String>,
    pub lora: Option<String>,
}

#[derive(Serialize)]
pub struct CreateJobResponse {
    pub job_id: Uuid,
}

pub async fn create_job_handler(
    State(state): State<AppState>,
    Json(payload): Json<CreateJobRequest>,
) -> impl IntoResponse {
    let job_id = Uuid::new_v4();
    let job = Job {
        id: job_id,
        source_url: payload.video_url,
        target_langs: payload.target_langs,
        status: JobStatus::Pending,
        style_config: StyleConfig {
            prompt: payload.prompt.unwrap_or_else(|| "Modern professional SaaS presentation, clean corporate layout, high fidelity, sharp focus".to_string()),
            lora: payload.lora,
        },
        assets_map: Vec::new(),
    };

    // Save job
    if let Err(e) = state.ingest_video_use_case.get_job_repo().save(&job).await {
        tracing::error!("Failed to save job to Redis: {}", e);
        return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response();
    }

    // Spawn background task
    let use_case = state.ingest_video_use_case.clone();
    tokio::spawn(async move {
        if let Err(e) = use_case.execute(job_id).await {
            tracing::error!("Job {} failed: {:?}", job_id, e);
            let _ = use_case.get_job_repo().update_status(job_id, JobStatus::Failed(e.to_string())).await;
        }
    });

    (axum::http::StatusCode::ACCEPTED, Json(CreateJobResponse { job_id })).into_response()
}

pub async fn get_job_handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.ingest_video_use_case.get_job_repo().find_by_id(id).await {
        Ok(Some(job)) => Json(job).into_response(),
        Ok(None) => (axum::http::StatusCode::NOT_FOUND, Json(json!({"error": "Job not found"}))).into_response(),
        Err(e) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
    }
}

use serde_json::json;
