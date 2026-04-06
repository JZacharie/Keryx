use async_trait::async_trait;
use anyhow::{Result, anyhow};
use serde_json::json;
use crate::domain::ports::pptx_repository::{PptxRepository, SlideInput};

pub struct PptxBuilderRepository {
    api_url: String,
    client: reqwest::Client,
}

impl PptxBuilderRepository {
    pub fn new(api_url: String) -> Self {
        Self { api_url, client: reqwest::Client::new() }
    }
}

#[async_trait]
impl PptxRepository for PptxBuilderRepository {
    async fn build(&self, job_id: &str, slides: Vec<SlideInput>) -> Result<String> {
        let slides_json: Vec<_> = slides.iter().map(|s| json!({
            "image_url": s.image_url,
            "text": s.text,
        })).collect();

        let response = self.client
            .post(format!("{}/build", self.api_url))
            .json(&json!({ "job_id": job_id, "slides": slides_json }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("PPTX build failed: {}", response.text().await?));
        }

        let body: serde_json::Value = response.json().await?;
        body["url"].as_str()
            .ok_or_else(|| anyhow!("Missing url in pptx response"))
            .map(|s| s.to_string())
    }
}
