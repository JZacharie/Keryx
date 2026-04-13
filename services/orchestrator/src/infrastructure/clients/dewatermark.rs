use anyhow::Result;
use serde::{Deserialize, Serialize};
use reqwest::Client;
use super::otel_propagation::inject_trace_context;


#[derive(Debug, Serialize)]
pub struct ImageCleanRequest {
    pub image_url: String,
    pub job_id: String,
    pub use_sdxl: bool,
    pub output_key: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct VideoCleanRequest {
    pub video_url: String,
    pub job_id: String,
    pub fps_override: Option<f64>,
    pub output_key: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CleanResponse {
    pub status: String,
    pub url: String,
    pub duration: String,
    pub frames_processed: Option<u32>,
}

pub struct DewatermarkClient {
    client: Client,
    base_url: String,
}

impl DewatermarkClient {
    pub fn new(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
        }
    }

    pub async fn clean_image(&self, image_url: &str, job_id: &str, use_sdxl: bool) -> Result<CleanResponse> {
        let req = ImageCleanRequest {
            image_url: image_url.to_string(),
            job_id: job_id.to_string(),
            use_sdxl,
            output_key: None,
        };

        let resp = inject_trace_context(
            self.client
                .post(format!("{}/clean/image", self.base_url))
                .json(&req)
        )
            .send()
            .await?;


        if !resp.status().is_success() {
            let error = resp.text().await?;
            return Err(anyhow::anyhow!("Dewatermark error: {}", error));
        }

        Ok(resp.json().await?)
    }

    pub async fn clean_video(&self, video_url: &str, job_id: &str) -> Result<CleanResponse> {
        let req = VideoCleanRequest {
            video_url: video_url.to_string(),
            job_id: job_id.to_string(),
            fps_override: None,
            output_key: None,
        };

        let resp = inject_trace_context(
            self.client
                .post(format!("{}/clean/video", self.base_url))
                .json(&req)
        )
            .send()
            .await?;


        if !resp.status().is_success() {
            let error = resp.text().await?;
            return Err(anyhow::anyhow!("Dewatermark error: {}", error));
        }

        Ok(resp.json().await?)
    }
}
