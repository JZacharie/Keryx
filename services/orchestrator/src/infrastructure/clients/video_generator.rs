use anyhow::Result;
use serde::{Deserialize, Serialize};
use reqwest::Client;
use super::otel_propagation::inject_trace_context;


#[derive(Debug, Serialize)]
pub struct AnimationRequest {
    pub image_url: String,
    pub job_id: String,
    pub fps: u32,
    pub motion_bucket_id: u32,
    pub noise_aug_strength: f64,
    pub num_frames: u32,
    pub output_key: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AnimationResponse {
    pub status: String,
    pub url: String,
    pub duration: String,
    pub frames: u32,
}

pub struct VideoGeneratorClient {
    client: Client,
    base_url: String,
}

impl VideoGeneratorClient {
    pub fn new(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
        }
    }

    pub async fn animate(&self, job_id: &str, image_url: &str) -> Result<AnimationResponse> {
        let req = AnimationRequest {
            image_url: image_url.to_string(),
            job_id: job_id.to_string(),
            fps: 14,
            motion_bucket_id: 127,
            noise_aug_strength: 0.02,
            num_frames: 25,
            output_key: None,
        };

        let resp = inject_trace_context(
            self.client
                .post(format!("{}/animate", self.base_url))
                .json(&req)
        )
            .send()
            .await?;


        if !resp.status().is_success() {
            let error = resp.text().await?;
            return Err(anyhow::anyhow!("Video Generator error: {}", error));
        }

        Ok(resp.json().await?)
    }
}
