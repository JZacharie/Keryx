use async_trait::async_trait;
use anyhow::Result;

#[async_trait]
pub trait StylizerRepository: Send + Sync {
    async fn style_image(&self, image_url: &str, prompt: &str) -> Result<String>;
}
