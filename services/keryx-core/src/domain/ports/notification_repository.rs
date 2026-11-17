use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait NotificationRepository: Send + Sync {
    async fn notify_slack(&self, message: &str) -> Result<()>;
}
