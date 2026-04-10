use async_trait::async_trait;
use anyhow::Result;

#[async_trait]
pub trait NotificationRepository: Send + Sync {
    async fn notify_slack(&self, message: &str) -> Result<()>;
}
