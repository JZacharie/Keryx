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
    let use_case = CreateJobUseCase::new(state.job_repository.clone());

    match use_case.execute(input).await {
        Ok(output) => (StatusCode::CREATED, Json(output)).into_response(),
        Err(e) => {
            tracing::error!("Failed to create job: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Internal server error"}))).into_response()
        }
    }
}
