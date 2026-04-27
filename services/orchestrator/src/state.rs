use std::sync::Arc;
use crate::application::use_cases::ingest_video::IngestVideoUseCase;
use crate::application::use_cases::voices_lab::VoicesLabUseCase;
use crate::infrastructure::clients::extractor::ExtractorClient;
use crate::infrastructure::clients::dewatermark::DewatermarkClient;
use crate::infrastructure::clients::voice_extractor::VoiceExtractorClient;
use crate::infrastructure::clients::texts_translation::TextsTranslationClient;
use crate::infrastructure::clients::voice_cloner::VoiceClonerClient;
use crate::infrastructure::clients::video_composer::VideoComposerClient;
use crate::infrastructure::clients::video_generator::VideoGeneratorClient;
use crate::infrastructure::clients::diffusion_engine::DiffusionEngineClient;
use crate::infrastructure::clients::pptx_builder::PptxBuilderClient;

#[derive(Clone)]
pub struct AppState {
    pub ingest_video_use_case: Arc<IngestVideoUseCase>,
    pub voices_lab_use_case: Arc<VoicesLabUseCase>,
    pub extractor: Arc<ExtractorClient>,
    pub dewatermark: Arc<DewatermarkClient>,
    pub voice_extractor: Arc<VoiceExtractorClient>,
    pub texts_translation: Arc<TextsTranslationClient>,
    pub voice_cloner: Arc<VoiceClonerClient>,
    pub video_composer: Arc<VideoComposerClient>,
    pub voices_composer: Arc<VideoComposerClient>,
    pub video_generator: Arc<VideoGeneratorClient>,
    pub diffusion_engine: Arc<DiffusionEngineClient>,
    pub pptx_builder: Arc<PptxBuilderClient>,
    pub gpu_semaphore: Arc<tokio::sync::Semaphore>,
}
