use async_trait::async_trait;
use anyhow::{Result, anyhow};
use std::path::PathBuf;
use keryx_core::domain::ports::voice_cloner_repository::VoiceClonerRepository;
use reqwest::Client;

pub struct CoquiVoiceClonerRepository {
    api_url: String,
    client: Client,
}

impl CoquiVoiceClonerRepository {
    pub fn new(api_url: String) -> Self {
        Self {
            api_url,
            client: Client::new(),
        }
    }
}

#[async_trait]
impl VoiceClonerRepository for CoquiVoiceClonerRepository {
    async fn voice_clone(&self, text: &str, language: &str, speaker_wav: Option<&str>, target_path: &PathBuf) -> Result<PathBuf> {
        let mut params = vec![
            ("text", text),
            ("language", language),
        ];
        
        if let Some(wav) = speaker_wav {
            params.push(("speaker_wav", wav));
        }

        let response = self.client.get(&self.api_url)
            .query(&params)
            .send()
            .await?;
        if !response.status().is_success() {
            let err = response.text().await?;
            return Err(anyhow!("Voice cloner error: {}", err));
        }

        let content = response.bytes().await?;
        tokio::fs::write(target_path, content).await?;

        Ok(target_path.clone())
    }
}
