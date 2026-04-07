use async_trait::async_trait;
use anyhow::{Result, anyhow};
use crate::domain::ports::scaling_repository::ScalingRepository;
use kube::{Client, Api};
use k8s_openapi::api::apps::v1::Deployment;
use kube::api::{Patch, PatchParams};
use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;

pub struct KubeScalingRepository {
    client: Client,
}

impl KubeScalingRepository {
    pub async fn new() -> Result<Self> {
        let client = Client::try_default().await?;
        Ok(Self { client })
    }
}

#[async_trait]
impl ScalingRepository for KubeScalingRepository {
    async fn scale_up(&self, namespace: &str, deployment_name: &str) -> Result<()> {
        let deployments: Api<Deployment> = Api::namespaced(self.client.clone(), namespace);
        
        tracing::info!("Scaling up deployment {}/{} to 1...", namespace, deployment_name);
        
        let patch = json!({
            "spec": {
                "replicas": 1
            }
        });
        
        deployments.patch(deployment_name, &PatchParams::default(), &Patch::Merge(&patch)).await?;
        
        // Wait for ready
        tracing::info!("Waiting for deployment {}/{} to be ready...", namespace, deployment_name);
        let mut attempts = 0;
        let mut preempted = false;

        while attempts < 300 { // 5 minutes timeout
            let d = deployments.get(deployment_name).await?;
            if let Some(status) = d.status {
                if status.ready_replicas.unwrap_or(0) >= 1 {
                    tracing::info!("Deployment {}/{} is ready!", namespace, deployment_name);
                    sleep(Duration::from_secs(7)).await;
                    return Ok(());
                }
            }
            
            // Priority Mechanism: If after 15 seconds it's still not ready, 
            // maybe there's not enough VRAM. Let's kill background AI services.
            if attempts > 15 && !preempted {
                tracing::warn!("Deployment {}/{} is slow to start. Preempting background AI services to free VRAM...", namespace, deployment_name);
                self.enforce_vram_priority().await?;
                preempted = true;
            }

            attempts += 1;
            sleep(Duration::from_secs(1)).await;
        }
        
        Err(anyhow!("Timeout waiting for deployment {}/{} to be ready", namespace, deployment_name))
    }

    async fn scale_down(&self, namespace: &str, deployment_name: &str) -> Result<()> {
        let deployments: Api<Deployment> = Api::namespaced(self.client.clone(), namespace);
        tracing::info!("Scaling down deployment {}/{} to 0...", namespace, deployment_name);
        let patch = json!({ "spec": { "replicas": 0 } });
        deployments.patch(deployment_name, &PatchParams::default(), &Patch::Merge(&patch)).await?;
        Ok(())
    }
}

impl KubeScalingRepository {
    /// Forces background AI services to scale down to 0 to free up GPU memory.
    async fn enforce_vram_priority(&self) -> Result<()> {
        let background_services = vec![
            ("llama-cpp", "llama-cpp"),
            ("qwen-tts", "qwen3-tts"),
        ];

        for (ns, deploy) in background_services {
            let api: Api<Deployment> = Api::namespaced(self.client.clone(), ns);
            let patch = json!({ "spec": { "replicas": 0 } });
            if let Err(e) = api.patch(deploy, &PatchParams::default(), &Patch::Merge(&patch)).await {
                tracing::warn!("Failed to preempt service {}/{} (it might not exist): {}", ns, deploy, e);
            } else {
                tracing::info!("Successfully preempted service {}/{} to free VRAM.", ns, deploy);
            }
        }
        Ok(())
    }
}
