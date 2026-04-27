use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::domain::entities::job::Segment;

#[derive(Debug, Serialize)]
pub struct TranslateRequest {
    pub segments: Vec<Segment>,
    pub target_lang: String,
    pub job_id: String,
}

#[derive(Debug, Deserialize)]
pub struct TranslateResponse {
    pub status: String,
    pub segments: Vec<Segment>,
}

#[derive(Debug, Serialize)]
pub struct RefineRequest {
    pub text: String,
    pub job_id: String,
}

#[derive(Debug, Deserialize)]
pub struct RefineResponse {
    pub status: String,
    pub refined_text: String,
}

pub struct TextsTranslationClient {
    url: String,
    client: reqwest::Client,
}

impl TextsTranslationClient {
    pub fn new(url: String) -> Self {
        Self {
            url,
            client: reqwest::Client::new(),
        }
    }

    pub async fn translate(&self, job_id: &str, segments: Vec<Segment>, target_lang: &str) -> anyhow::Result<Vec<Segment>> {
        let url = format!("{}/translate", self.url);
        let req = TranslateRequest {
            segments,
            target_lang: target_lang.to_string(),
            job_id: job_id.to_string(),
        };

        let resp = self.client.post(&url)
            .json(&req)
            .send()
            .await?
            .json::<TranslateResponse>()
            .await?;

        Ok(resp.segments)
    }

    pub async fn refine(&self, job_id: &str, text: &str) -> anyhow::Result<String> {
        let url = format!("{}/refine", self.url);
        let req = RefineRequest {
            text: text.to_string(),
            job_id: job_id.to_string(),
        };

        let resp = self.client.post(&url)
            .json(&req)
            .send()
            .await?
            .json::<RefineResponse>()
            .await?;

        Ok(resp.refined_text)
    }
}
