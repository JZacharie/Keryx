use async_trait::async_trait;
use anyhow::Result;

#[async_trait]
pub trait TranslatorRepository: Send + Sync {
    async fn translate(&self, text: &str, target_lang: &str) -> Result<String>;
}
