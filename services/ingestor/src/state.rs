use std::sync::Arc;
use crate::application::use_cases::ingest_video::IngestVideoUseCase;

#[derive(Clone)]
pub struct AppState {
    pub ingest_video_use_case: Arc<IngestVideoUseCase>,
}
