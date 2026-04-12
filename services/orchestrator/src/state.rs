use std::sync::Arc;
use crate::application::use_cases::ingest_video::IngestVideoUseCase;
use crate::infrastructure::clients::extractor::ExtractorClient;
use crate::infrastructure::clients::dewatermark::DewatermarkClient;
use crate::infrastructure::clients::voice_extractor::VoiceExtractorClient;
use crate::infrastructure::clients::voice_cloner::VoiceClonerClient;
use crate::infrastructure::clients::video_composer::VideoComposerClient;
use crate::infrastructure::clients::video_generator::VideoGeneratorClient;

#[derive(Clone)]
pub struct AppState {
    pub ingest_video_use_case: Arc<IngestVideoUseCase>,
    pub extractor: Arc<ExtractorClient>,
    pub dewatermark: Arc<DewatermarkClient>,
    pub voice_extractor: Arc<VoiceExtractorClient>,
    pub voice_cloner: Arc<VoiceClonerClient>,
    pub video_composer: Arc<VideoComposerClient>,
    pub video_generator: Arc<VideoGeneratorClient>,
}
