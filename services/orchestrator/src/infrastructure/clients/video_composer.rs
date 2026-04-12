use anyhow::Result;
use serde::{Deserialize, Serialize};
use reqwest::Client;

#[derive(Debug, Serialize)]
pub struct SlideInput {
    pub image_url: String,
    pub duration: f64,
}

#[derive(Debug, Serialize)]
pub struct ComposeRequest {
    pub job_id: String,
    pub slides: Vec<SlideInput>,
    pub audio_url: Option<String>,
    pub intro_url: Option<String>,
    pub output_key: Option<String>,
    pub fps: u32,
}

#[derive(Debug, Serialize)]
pub struct ConcatAudioRequest {
    pub job_id: String,
    pub segments: Vec<String>,
    pub output_key: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DetectSlidesRequest {
    pub job_id: String,
    pub video_url: String,
    pub scene_threshold: f64,
    pub output_prefix: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SlideFrame {
    pub index: u32,
    pub timestamp: f64,
    pub image_url: String,
}

#[derive(Debug, Deserialize)]
pub struct DetectSlidesResponse {
    pub status: String,
    pub slides: Vec<SlideFrame>,
}

#[derive(Debug, Deserialize)]
pub struct ComposeResponse {
    pub status: String,
    pub url: String,
    pub duration: String,
}

pub struct VideoComposerClient {
    client: Client,
    base_url: String,
}

impl VideoComposerClient {
    pub fn new(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
        }
    }

    pub async fn compose(&self, job_id: &str, slides: Vec<SlideInput>, audio_url: Option<String>) -> Result<ComposeResponse> {
        let req = ComposeRequest {
            job_id: job_id.to_string(),
            slides,
            audio_url,
            intro_url: None,
            output_key: None,
            fps: 24,
        };

        let resp = self.client
            .post(format!("{}/compose", self.base_url))
            .json(&req)
            .send()
            .await?;

        if !resp.status().is_success() {
            let error = resp.text().await?;
            return Err(anyhow::anyhow!("Video Composer error: {}", error));
        }

        Ok(resp.json().await?)
    }

    pub async fn concat_audio(&self, job_id: &str, segments: Vec<String>) -> Result<ComposeResponse> {
        let req = ConcatAudioRequest {
            job_id: job_id.to_string(),
            segments,
            output_key: None,
        };

        let resp = self.client
            .post(format!("{}/concat_audio", self.base_url))
            .json(&req)
            .send()
            .await?;

        if !resp.status().is_success() {
            let error = resp.text().await?;
            return Err(anyhow::anyhow!("Video Composer error: {}", error));
        }

        Ok(resp.json().await?)
    }

    pub async fn detect_slides(&self, job_id: &str, video_url: &str) -> Result<DetectSlidesResponse> {
        let req = DetectSlidesRequest {
            job_id: job_id.to_string(),
            video_url: video_url.to_string(),
            scene_threshold: 0.3,
            output_prefix: None,
        };

        let resp = self.client
            .post(format!("{}/detect_slides", self.base_url))
            .json(&req)
            .send()
            .await?;

        if !resp.status().is_success() {
            let error = resp.text().await?;
            return Err(anyhow::anyhow!("Video Composer error: {}", error));
        }

        Ok(resp.json().await?)
    }
}
