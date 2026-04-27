use std::sync::Arc;
use uuid::Uuid;
use anyhow::Result;
use keryx_core::domain::ports::job_repository::JobRepository;
use keryx_core::domain::ports::scaling_repository::ScalingRepository;

use crate::infrastructure::clients::voice_extractor::{VoiceExtractorClient, Segment};
use crate::infrastructure::clients::texts_translation::TextsTranslationClient;
use crate::infrastructure::clients::voice_cloner::VoiceClonerClient;
use crate::infrastructure::clients::video_composer::VideoComposerClient;

pub struct VoicesLabUseCase {
    job_repo: Arc<dyn JobRepository>,
    scaling_repo: Arc<dyn ScalingRepository>,
    voice_extractor: Arc<VoiceExtractorClient>,
    texts_translation: Arc<TextsTranslationClient>,
    voice_cloner: Arc<VoiceClonerClient>,
    voices_composer: Arc<VideoComposerClient>,
}

impl VoicesLabUseCase {
    pub fn new(
        job_repo: Arc<dyn JobRepository>,
        scaling_repo: Arc<dyn ScalingRepository>,
        voice_extractor: Arc<VoiceExtractorClient>,
        texts_translation: Arc<TextsTranslationClient>,
        voice_cloner: Arc<VoiceClonerClient>,
        voices_composer: Arc<VideoComposerClient>,
    ) -> Self {
        Self {
            job_repo,
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
        self.scaling_repo.scale_up("keryx", "keryx-voice-extractor").await?;
        let trans_res = self.voice_extractor.perform_transcription(audio_url, &job_id_str, None).await?;
        self.scaling_repo.scale_down("keryx", "keryx-voice-extractor").await?;

        // 2. Refinement
        self.scaling_repo.scale_up("keryx", "keryx-texts-translation").await?;
        let full_text: String = trans_res.segments.iter().map(|s| s.text.clone()).collect::<Vec<_>>().join(" ");
        let refined_text = self.texts_translation.refine(&job_id_str, &full_text).await?;

        // 3. Translation
        let dummy_seg = Segment {
            start: 0.0,
            end: trans_res.duration,
            text: refined_text,
            translated: None,
        };
        let translated_segs = self.texts_translation.translate(&job_id_str, vec![dummy_seg], target_lang).await?;
        let translated_text = translated_segs.first().and_then(|s| s.translated.clone()).unwrap_or_default();
        self.scaling_repo.scale_down("keryx", "keryx-texts-translation").await?;

        // 4. Cloning
        self.scaling_repo.scale_up("keryx", "keryx-voices-cloner").await?;
        let clone_res = self.voice_cloner.perform_cloning(&translated_text, target_lang, audio_url, &job_id_str).await?;
        self.scaling_repo.scale_down("keryx", "keryx-voices-cloner").await?;

        // 5. Concat (even if only one, it handles S3 upload etc)
        self.scaling_repo.scale_up("keryx", "voices-composer").await?;
        let concat_res = self.voices_composer.concat_audio(&job_id_str, vec![clone_res.url]).await?;
        self.scaling_repo.scale_down("keryx", "voices-composer").await?;

        Ok(concat_res.url)
    }
}
