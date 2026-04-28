use axum::{response::IntoResponse, Json};
use serde_json::json;

pub async fn health_check() -> impl IntoResponse {
    Json(json!({ "status": "ok", "service": "keryx-orchestrator" }))
}
