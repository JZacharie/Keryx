use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

/// State data for the auth middleware — just the expected API key.
#[derive(Clone)]
pub struct AuthState {
    pub api_key: String,
}

/// Axum middleware that checks `Authorization: Bearer <key>` header.
/// Returns 401 if the header is missing or malformed, 403 if the key is wrong.
pub async fn require_api_key(
    State(auth): State<AuthState>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let auth_header = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    match auth_header {
        None => (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Missing Authorization header. Expected: Bearer <API_KEY>"})),
        )
            .into_response(),
        Some(header) => {
            let token = header.strip_prefix("Bearer ").unwrap_or("").trim();
            if token == auth.api_key {
                next.run(req).await
            } else {
                (
                    StatusCode::FORBIDDEN,
                    Json(json!({"error": "Invalid API key"})),
                )
                    .into_response()
            }
        }
    }
}
