use async_trait::async_trait;
use anyhow::Result;
use crate::domain::ports::translator_repository::TranslatorRepository;
use serde_json::json;

pub struct OllamaTranslatorRepository {
    client: reqwest::Client,
    base_url: String,
    model: String,
}

impl OllamaTranslatorRepository {
    pub fn new(base_url: &str, model: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.to_string(),
            model: model.to_string(),
        }
    }
}

#[async_trait]
impl TranslatorRepository for OllamaTranslatorRepository {
    async fn translate(&self, text: &str, target_lang: &str) -> Result<String> {
        let prompt = format!(
            "Translate the following text into {}. Maintain technical accuracy and return ONLY the translated text.\n\nText: {}",
            target_lang, text
        );

        let body = json!({
            "model": self.model,
            "prompt": prompt,
            "stream": false
        });

        let response = self.client.post(format!("{}/api/generate", self.base_url))
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Ollama API failed with status: {}", response.status()));
        }

        let result: OllamaResponse = response.json().await?;
        Ok(result.response)
    }
}

#[derive(serde::Deserialize)]
struct OllamaResponse {
    response: String,
}
