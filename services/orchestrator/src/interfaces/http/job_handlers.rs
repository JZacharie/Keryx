use axum::{
    extract::{State, Json},
    response::IntoResponse,
    http::StatusCode,
};
use crate::AppState;
use crate::application::use_cases::create_job::{CreateJobInput, CreateJobUseCase};

pub async fn create_job_handler(
    State(state): State<AppState>,
    Json(input): Json<CreateJobInput>,
) -> impl IntoResponse {
    let use_case = CreateJobUseCase::new(
        state.job_repository.clone(),
        state.container_manager.clone(),
    );

    match use_case.execute(input).await {
        Ok(output) => (StatusCode::CREATED, Json(output)).into_response(),
        Err(e) => {
            tracing::error!("Failed to create job: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Internal server error"}))).into_response()
        }
    }
}

pub async fn get_job_handler(
    State(state): State<AppState>,
    axum::extract::Path(job_id): axum::extract::Path<String>,
) -> impl IntoResponse {
    match state.job_repository.get_by_id(&job_id).await {
        Ok(Some(job)) => (StatusCode::OK, Json(job)).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Job not found"}))).into_response(),
        Err(e) => {
            tracing::error!("Failed to get job: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Internal server error"}))).into_response()
        }
    }
}
