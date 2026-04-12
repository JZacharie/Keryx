use anyhow::Result;
use serde::{Deserialize, Serialize};
use reqwest::Client;

#[derive(Debug, Serialize)]
pub struct ExtractRequest {
    pub url: String,
    pub job_id: String,
}

#[derive(Debug, Deserialize)]
pub struct ExtractResponse {
    pub status: String,
    pub video_url: String,
    pub audio_url: String,
    pub duration: f64,
    pub title: String,
}

pub struct ExtractorClient {
    client: Client,
    base_url: String,
}

impl ExtractorClient {
    pub fn new(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
        }
    }

    pub async fn extract(&self, url: &str, job_id: &str) -> Result<ExtractResponse> {
        let req = ExtractRequest {
            url: url.to_string(),
            job_id: job_id.to_string(),
        };

        let resp = self.client
            .post(format!("{}/extract", self.base_url))
            .json(&req)
            .send()
            .await?;

        if !resp.status().is_success() {
            let error: String = resp.text().await?;
            return Err(anyhow::anyhow!("Extractor error: {}", error));
        }

        Ok(resp.json().await?)
    }
}
