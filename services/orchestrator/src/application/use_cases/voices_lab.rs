use std::sync::Arc;
use uuid::Uuid;
use anyhow::Result;
use keryx_core::domain::ports::scaling_repository::ScalingRepository;

use crate::infrastructure::clients::voice_extractor::{VoiceExtractorClient, Segment};
use crate::infrastructure::clients::texts_translation::TextsTranslationClient;
use crate::infrastructure::clients::voice_cloner::VoiceClonerClient;
use crate::infrastructure::clients::video_composer::VideoComposerClient;
use crate::infrastructure::scaling_guard::WorkerGuard;

pub struct VoicesLabUseCase {
    scaling_repo: Arc<dyn ScalingRepository>,
    voice_extractor: Arc<VoiceExtractorClient>,
    texts_translation: Arc<TextsTranslationClient>,
    voice_cloner: Arc<VoiceClonerClient>,
    voices_composer: Arc<VideoComposerClient>,
}

impl VoicesLabUseCase {
    pub fn new(
        scaling_repo: Arc<dyn ScalingRepository>,
        voice_extractor: Arc<VoiceExtractorClient>,
        texts_translation: Arc<TextsTranslationClient>,
        voice_cloner: Arc<VoiceClonerClient>,
        voices_composer: Arc<VideoComposerClient>,
    ) -> Self {
        Self {
            scaling_repo,
            voice_extractor,
            texts_translation,
            voice_cloner,
            voices_composer,
        }
    }

    pub async fn execute_test(&self, audio_url: &str, target_lang: &str) -> Result<String> {
        let test_id = Uuid::new_v4();
        let job_id_str = test_id.to_string();
        
        tracing::info!("[VoicesLab] Starting test for audio: {}", audio_url);

        // 1. Transcription
        let trans_res = {
            let _guard = WorkerGuard::new(self.scaling_repo.clone(), "keryx", "keryx-voice-extractor").await?;
            self.voice_extractor.perform_transcription(audio_url, &job_id_str, None).await?
        };

        // 2 & 3. Refinement & Translation
        let translated_text = {
            let _guard = WorkerGuard::new(self.scaling_repo.clone(), "keryx", "keryx-texts-translation").await?;
            
            let full_text: String = trans_res.segments.iter().map(|s| s.text.clone()).collect::<Vec<_>>().join(" ");
            let refined_text = self.texts_translation.refine(&job_id_str, &full_text).await?;

            let dummy_seg = Segment {
                start: 0.0,
                end: trans_res.duration,
                text: refined_text,
                translated: None,
            };
            let translated_segs = self.texts_translation.translate(&job_id_str, vec![dummy_seg], target_lang).await?;
            translated_segs.first().and_then(|s| s.translated.clone()).unwrap_or_default()
        };

        // 4. Cloning
        let clone_res = {
            let _guard = WorkerGuard::new(self.scaling_repo.clone(), "keryx", "keryx-voices-cloner").await?;
            self.voice_cloner.perform_cloning(&translated_text, target_lang, audio_url, &job_id_str).await?
        };

        // 5. Concat
        let concat_res = {
            let _guard = WorkerGuard::new(self.scaling_repo.clone(), "keryx", "voices-composer").await?;
            self.voices_composer.concat_audio(&job_id_str, vec![clone_res.url]).await?
        };

        Ok(concat_res.url)
    }
}
