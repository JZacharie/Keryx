use async_trait::async_trait;
use anyhow::Result;

pub struct SlideInput {
    pub image_url: String,
    pub text: String,
}

#[async_trait]
pub trait PptxRepository: Send + Sync {
    async fn build(&self, job_id: &str, slides: Vec<SlideInput>) -> Result<String>;
}
