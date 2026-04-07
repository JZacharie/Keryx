use async_trait::async_trait;
use anyhow::Result;

#[async_trait]
pub trait ScalingRepository: Send + Sync {
    /// Scales up a deployment to 1 replica and waits for it to be ready.
    async fn scale_up(&self, namespace: &str, deployment: &str) -> Result<()>;
    
    /// Waits for a service to respond on /health
    async fn wait_for_service_ping(&self, service_name: &str) -> Result<()>;
    
    /// Scales down a deployment to 0 replicas.
    async fn scale_down(&self, namespace: &str, deployment: &str) -> Result<()>;
}
