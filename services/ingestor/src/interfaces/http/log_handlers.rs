use axum::{
    extract::{Path, State},
    response::{IntoResponse, Response, Sse},
    http::StatusCode,
    Json,
};
use axum::response::sse::{Event, KeepAlive};
use futures::stream::{self, Stream};
use serde_json::json;
use std::convert::Infallible;
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;
use crate::state::AppState;

/// `GET /api/jobs/:id/logs/raw` — Snapshot JSON de tous les logs actuels
pub async fn get_job_logs_raw_handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.ingest_video_use_case.get_job_repo().get_logs(id).await {
        Ok(logs) => Json(json!({ "job_id": id, "logs": logs })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// `GET /api/jobs/:id/logs` — Stream SSE (Server-Sent Events) des logs en temps réel.
/// Poll Redis toutes les 500ms et envoie les nouvelles lignes.
pub async fn get_job_logs_sse_handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let job_repo = state.ingest_video_use_case.get_job_repo();

    let stream = stream::unfold(
        (job_repo, 0usize, false),
        move |(repo, cursor, done)| async move {
            if done {
                return None;
            }

            sleep(Duration::from_millis(500)).await;

            let logs = match repo.get_logs(id).await {
                Ok(l) => l,
                Err(e) => {
                    let ev = Event::default()
                        .event("error")
                        .data(format!("log fetch error: {}", e));
                    return Some((Ok(ev), (repo, cursor, true)));
                }
            };

            // Envoyer seulement les nouvelles lignes
            let new_lines = &logs[cursor..];
            let new_cursor = logs.len();

            // Vérifier si le job est terminé (Completed ou Failed)
            let is_done = match repo
                .find_by_id(id)
                .await
                .ok()
                .flatten()
                .map(|j| j.status)
            {
                Some(keryx_core::domain::entities::job::JobStatus::Completed) => true,
                Some(keryx_core::domain::entities::job::JobStatus::Failed(_)) => true,
                _ => false,
            };

            if new_lines.is_empty() {
                if is_done {
                    // Envoyer l'événement "done" pour fermer le stream côté client
                    let ev = Event::default().event("done").data("job_finished");
                    return Some((Ok(ev), (repo, new_cursor, true)));
                }
                // Rien de nouveau, envoyer un heartbeat silencieux
                let ev = Event::default().event("heartbeat").data(".");
                return Some((Ok(ev), (repo, new_cursor, false)));
            }

            let batch = new_lines.join("\n");
            let ev = Event::default().event("log").data(batch);

            Some((Ok(ev), (repo, new_cursor, is_done)))
        },
    );

    Sse::new(stream).keep_alive(KeepAlive::default())
}
