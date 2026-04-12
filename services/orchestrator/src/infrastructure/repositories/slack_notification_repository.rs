use async_trait::async_trait;
use anyhow::Result;
use keryx_core::domain::ports::notification_repository::NotificationRepository;
use serde_json::json;

pub struct SlackNotificationRepository {
    webhook_url: String,
    client: reqwest::Client,
}

impl SlackNotificationRepository {
    pub fn new(webhook_url: String) -> Self {
        Self {
            webhook_url,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl NotificationRepository for SlackNotificationRepository {
    async fn notify_slack(&self, message: &str) -> Result<()> {
        let payload = json!({
            "text": message
        });

        self.client.post(&self.webhook_url)
            .json(&payload)
            .send()
            .await?;
        
        Ok(())
    }
}
