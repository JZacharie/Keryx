use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait TranslatorRepository: Send + Sync {
    async fn translate(&self, text: &str, target_lang: &str) -> Result<String>;
}
