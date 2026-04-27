use std::sync::Arc;
use keryx_core::domain::ports::scaling_repository::ScalingRepository;
use tracing::{info, error};

/// Guard that automatically scales down a worker service when dropped.
pub struct WorkerGuard {
    scaling_repo: Arc<dyn ScalingRepository>,
    namespace: String,
    service_name: String,
    active: bool,
}

impl WorkerGuard {
    /// Creates a new guard and immediately scales up the service.
    pub async fn new(
        scaling_repo: Arc<dyn ScalingRepository>,
        namespace: &str,
        service_name: &str,
    ) -> anyhow::Result<Self> {
        info!("[ScalingGuard] Scaling UP {} in namespace {}...", service_name, namespace);
        scaling_repo.scale_up(namespace, service_name).await?;
        
        Ok(Self {
            scaling_repo,
            namespace: namespace.to_string(),
            service_name: service_name.to_string(),
            active: true,
        })
    }

    /// Disables the guard so it won't scale down when dropped.
    pub fn keep_alive(&mut self) {
        self.active = false;
    }
}

impl Drop for WorkerGuard {
    fn drop(&mut self) {
        if self.active {
            let repo = self.scaling_repo.clone();
            let ns = self.namespace.clone();
            let svc = self.service_name.clone();
            
            // We need to spawn a task because drop is synchronous
            tokio::spawn(async move {
                info!("[ScalingGuard] Auto-scaling DOWN {} in namespace {}...", svc, ns);
                if let Err(e) = repo.scale_down(&ns, &svc).await {
                    error!("[ScalingGuard] Failed to auto-scale down {}: {:?}", svc, e);
                }
            });
        }
    }
}
