use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
    http::{StatusCode, header},
    Json,
};
use serde_json::json;
use crate::infrastructure::auth::verifier::JwtVerifier;
use std::sync::Arc;

pub async fn auth_middleware(
    State(verifier): State<Arc<JwtVerifier>>,
    request: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    let auth_header = request.headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    let token = if let Some(auth_header) = auth_header {
        if auth_header.starts_with("Bearer ") {
            &auth_header[7..]
        } else {
            return Err((StatusCode::UNAUTHORIZED, Json(json!({"error": "Missing bearer token"}))));
        }
    } else {
        return Err((StatusCode::UNAUTHORIZED, Json(json!({"error": "Missing authorization header"}))));
    };

    match verifier.verify(token) {
        Ok(claims) => {
            let mut request = request;
            request.extensions_mut().insert(claims);
            Ok(next.run(request).await)
        },
        Err(e) => {
            tracing::error!("JWT verification failed: {}", e);
            Err((StatusCode::UNAUTHORIZED, Json(json!({"error": "Invalid or expired token"}))))
        },
    }
}
