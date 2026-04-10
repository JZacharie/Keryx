use async_trait::async_trait;
use anyhow::{Result, anyhow};
use serde_json::json;
use keryx_core::domain::ports::stylizer_repository::StylizerRepository;

pub struct DiffusionStylizerRepository {
    api_url: String,
    client: reqwest::Client,
}

impl DiffusionStylizerRepository {
    pub fn new(api_url: String) -> Self {
        Self {
            api_url,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl StylizerRepository for DiffusionStylizerRepository {
    async fn style_image(&self, image_url: &str, prompt: &str) -> Result<String> {
        let response = self.client.post(format!("{}{}", self.api_url, "/style"))
            .json(&json!({
                "image_url": image_url,
                "prompt": prompt,
                "strength": 0.5,
                "num_inference_steps": 2
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("Diffusion styling failed: {}", error_text));
        }

        let body: serde_json::Value = response.json().await?;
        let styled_url = body["url"].as_str()
            .ok_or_else(|| anyhow!("Failed to parse styled URL from response"))?
            .to_string();

        Ok(styled_url)
    }

    async fn clean_watermark(&self, image_url: &str, target_path: &str) -> Result<String> {
        let response = self.client.post(format!("{}{}", self.api_url, "/clean_watermark"))
            .json(&json!({
                "image_url": image_url,
                "target_path": target_path
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("Watermark cleaning failed: {}", error_text));
        }

        let body: serde_json::Value = response.json().await?;
        let cleaned_url = body["url"].as_str()
            .ok_or_else(|| anyhow!("Failed to parse cleaned URL from response"))?
            .to_string();

        Ok(cleaned_url)
    }
}
