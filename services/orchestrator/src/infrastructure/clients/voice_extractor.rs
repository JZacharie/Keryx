use anyhow::Result;
use serde::{Deserialize, Serialize};
use reqwest::Client;
use super::otel_propagation::inject_trace_context;


#[derive(Debug, Serialize)]
pub struct TranscribeRequest {
    pub audio_url: String,
    pub job_id: String,
    pub language: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Segment {
    pub start: f64,
    pub end: f64,
    pub text: String,
    pub translated: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TranscribeResponse {
    pub status: String,
    pub segments: Vec<Segment>,
    pub duration: f64,
    pub language: String,
}

#[derive(Debug, Serialize)]
pub struct TranslateRequest {
    pub segments: Vec<Segment>,
    pub target_lang: String,
    pub job_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TranslateResponse {
    pub status: String,
    pub segments: Vec<Segment>,
}

#[derive(Debug, Serialize)]
pub struct RefineRequest {
    pub text: String,
    pub job_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RefineResponse {
    pub status: String,
    pub refined_text: String,
}

pub struct VoiceExtractorClient {
    client: Client,
    base_url: String,
}

impl VoiceExtractorClient {
    pub fn new(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
        }
    }

    pub async fn perform_transcription(&self, audio_url: &str, job_id: &str, language: Option<String>) -> Result<TranscribeResponse> {
        let req = TranscribeRequest {
            audio_url: audio_url.to_string(),
            job_id: job_id.to_string(),
            language,
        };

        let resp = inject_trace_context(
            self.client
                .post(format!("{}/transcribe", self.base_url))
                .json(&req)
        )
            .send()
            .await?;


        if !resp.status().is_success() {
            let error = resp.text().await?;
            return Err(anyhow::anyhow!("Voice Extractor error: {}", error));
        }

        Ok(resp.json().await?)
    }

    pub async fn translate(&self, segments: Vec<Segment>, target_lang: &str, job_id: &str) -> Result<TranslateResponse> {
        let req = TranslateRequest {
            segments,
            target_lang: target_lang.to_string(),
            job_id: job_id.to_string(),
        };

        let resp = inject_trace_context(
            self.client
                .post(format!("{}/translate", self.base_url))
                .json(&req)
        )
            .send()
            .await?;


        if !resp.status().is_success() {
            let error = resp.text().await?;
            return Err(anyhow::anyhow!("Voice Extractor error: {}", error));
        }

        Ok(resp.json().await?)
    }

    pub async fn refine(&self, text: &str, job_id: &str) -> Result<RefineResponse> {
        let req = RefineRequest {
            text: text.to_string(),
            job_id: job_id.to_string(),
        };

        let resp = inject_trace_context(
            self.client
                .post(format!("{}/refine", self.base_url))
                .json(&req)
        )
            .send()
            .await?;

        if !resp.status().is_success() {
            let error = resp.text().await?;
            return Err(anyhow::anyhow!("Voice Extractor error (refine): {}", error));
        }

        Ok(resp.json().await?)
    }
}
