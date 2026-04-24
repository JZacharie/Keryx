use anyhow::Result;
use serde::{Deserialize, Serialize};
use reqwest::Client;
use super::otel_propagation::inject_trace_context;

#[derive(Debug, Serialize)]
pub struct StyleRequest {
    pub image_url: String,
    pub prompt: String,
    pub strength: f32,
    pub guidance_scale: f32,
    pub num_inference_steps: u32,
    pub target_path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct StyleResponse {
    pub status: String,
    pub url: String,
    pub prompt: String,
}

pub struct DiffusionEngineClient {
    client: Client,
    base_url: String,
}

impl DiffusionEngineClient {
    pub fn new(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
        }
    }

    pub async fn style_image(
        &self, 
        image_url: &str, 
        prompt: &str, 
        strength: f32,
        guidance_scale: f32,
        steps: u32,
        target_path: Option<String>
    ) -> Result<StyleResponse> {
        let req = StyleRequest {
            image_url: image_url.to_string(),
            prompt: prompt.to_string(),
            strength,
            guidance_scale,
            num_inference_steps: steps,
            target_path,
        };

        let resp = inject_trace_context(
            self.client
                .post(format!("{}/style", self.base_url))
                .json(&req)
        )
            .send()
            .await?;

        if !resp.status().is_success() {
            let error = resp.text().await?;
            return Err(anyhow::anyhow!("Diffusion Engine error: {}", error));
        }

        Ok(resp.json().await?)
    }
}
