use async_trait::async_trait;
use anyhow::{Result, anyhow};
use std::path::PathBuf;
use keryx_core::domain::ports::tts_repository::TTSRepository;
use reqwest::Client;
use serde_json::json;

pub struct QwenTTSRepository {
    api_url: String,
    client: Client,
}

impl QwenTTSRepository {
    pub fn new(api_url: String) -> Self {
        Self {
            api_url,
            client: Client::new(),
        }
    }
}

#[async_trait]
impl TTSRepository for QwenTTSRepository {
    async fn generate(&self, text: &str, language: &str, target_path: &PathBuf) -> Result<PathBuf> {
        // Qwen3-TTS usually uses Gradio or direct FastAPI.
        // Assuming /generate endpoint or similar. Based on user's pattern:
        let response = self.client.post(format!("{}/generate", self.api_url))
            .json(&json!({
                "text": text,
                "language": language
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let err = response.text().await?;
            return Err(anyhow!("Qwen TTS error: {}", err));
        }

        let content = response.bytes().await?;
        tokio::fs::write(target_path, content).await?;

        Ok(target_path.clone())
    }
}
