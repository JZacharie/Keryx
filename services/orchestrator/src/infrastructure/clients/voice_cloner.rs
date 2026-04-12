use anyhow::Result;
use serde::{Deserialize, Serialize};
use reqwest::Client;

#[derive(Debug, Serialize)]
pub struct CloneRequest {
    pub text: String,
    pub language: String,
    pub reference_url: String,
    pub job_id: String,
    pub output_key: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CloneResponse {
    pub status: String,
    pub url: String,
    pub duration: String,
}

pub struct VoiceClonerClient {
    client: Client,
    base_url: String,
}

impl VoiceClonerClient {
    pub fn new(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
        }
    }

    pub async fn perform_cloning(&self, text: &str, language: &str, reference_url: &str, job_id: &str) -> Result<CloneResponse> {
        let req = CloneRequest {
            text: text.to_string(),
            language: language.to_string(),
            reference_url: reference_url.to_string(),
            job_id: job_id.to_string(),
            output_key: None,
        };

        let resp = self.client
            .post(format!("{}/clone", self.base_url))
            .json(&req)
            .send()
            .await?;

        if !resp.status().is_success() {
            let error = resp.text().await?;
            return Err(anyhow::anyhow!("Voice Cloner error: {}", error));
        }

        Ok(resp.json().await?)
    }
}
