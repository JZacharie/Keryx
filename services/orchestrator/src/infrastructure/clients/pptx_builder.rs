use anyhow::Result;
use serde::{Deserialize, Serialize};
use reqwest::Client;
use super::otel_propagation::inject_trace_context;

#[derive(Debug, Serialize)]
pub struct PptxSlide {
    pub image_url: String,
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct PptxRequest {
    pub job_id: String,
    pub slides: Vec<PptxSlide>,
    pub target_path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PptxResponse {
    pub status: String,
    pub url: String,
}

pub struct PptxBuilderClient {
    client: Client,
    base_url: String,
}

impl PptxBuilderClient {
    pub fn new(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
        }
    }

    pub async fn build_pptx(&self, job_id: &str, slides: Vec<PptxSlide>, target_path: Option<String>) -> Result<PptxResponse> {
        let req = PptxRequest {
            job_id: job_id.to_string(),
            slides,
            target_path,
        };

        let resp = inject_trace_context(
            self.client
                .post(format!("{}/build", self.base_url))
                .json(&req)
        )
            .send()
            .await?;

        if !resp.status().is_success() {
            let error = resp.text().await?;
            return Err(anyhow::anyhow!("PPTX Builder error: {}", error));
        }

        Ok(resp.json().await?)
    }
}
