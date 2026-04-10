use async_trait::async_trait;
use anyhow::{Result, anyhow};
use crate::domain::ports::scaling_repository::ScalingRepository;
use kube::{Client, Api, core::DynamicObject, discovery::ApiResource};
use kube::api::{Patch, PatchParams, GroupVersionKind};
use k8s_openapi::api::apps::v1::Deployment;
use k8s_openapi::api::core::v1::Pod;
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
        
        // KEDA Compatibility: If there is an associated HTTPScaledObject, we must set its minReplicas to 1
        // to prevent KEDA from scaling the deployment back to 0 due to lack of traffic.
        if deployment_name == "openai-whisper-asr-webservice" {
            if let Err(e) = self.patch_httpscaledobject_min_replicas(namespace, "openai-whisper-asr-webservice-http", 1).await {
                tracing::warn!("Failed to patch HTTPScaledObject for {}: {}", deployment_name, e);
            }
        } else if deployment_name == "whisperx" {
             if let Err(e) = self.patch_httpscaledobject_min_replicas(namespace, "whisperx-http", 1).await {
                tracing::warn!("Failed to patch HTTPScaledObject for {}: {}", deployment_name, e);
            }
        }
        
        // Wait for ready
        tracing::info!("Waiting for deployment {}/{} to be ready...", namespace, deployment_name);
        let mut attempts = 0;
        let mut preempted = false;

        while attempts < 600 { // 10 minutes timeout
            let d = deployments.get(deployment_name).await?;
            if let Some(status) = d.status {
                if status.ready_replicas.unwrap_or(0) >= 1 {
                    tracing::info!("Deployment {}/{} is ready! Waiting extra 20s for ML service initialization...", namespace, deployment_name);
                    sleep(Duration::from_secs(20)).await;
                    return Ok(());
                }
            }
            
            // Priority Mechanism: If after 30 seconds it's still not ready, 
            // maybe there's not enough VRAM. Let's kill background AI services.
            if attempts > 30 && !preempted {
                tracing::warn!("Deployment {}/{} is slow to start ({}s). Preempting background AI services to free VRAM...", namespace, deployment_name, attempts);
                self.enforce_vram_priority().await?;
                preempted = true;
            }

            attempts += 1;
            sleep(Duration::from_secs(1)).await;
        }

        // If we reach here, it's a timeout.
        // Before returning error, try to capture pod status for debugging
        let label_selector = format!("app={},app.kubernetes.io/name={}", deployment_name, deployment_name);
        if let Ok(pods) = Api::<Pod>::namespaced(self.client.clone(), namespace).list(&kube::api::ListParams::default().labels(&label_selector)).await {
            for p in pods {
                let pod_name = p.metadata.name.clone().unwrap_or_else(|| "unknown".to_string());
                if let Some(status) = p.status {
                    tracing::error!("Pod {} status: Phase={:?}, Reason={:?}, Message={:?}", 
                        pod_name, status.phase, status.reason, status.message);
                    if let Some(container_statuses) = status.container_statuses {
                        for cs in container_statuses {
                            tracing::error!("  Container {} state: {:?}", cs.name, cs.state);
                        }
                    }
                }
            }
        } else {
            tracing::error!("Failed to list pods for debugging timeout of {}/{} with selector {}", namespace, deployment_name, label_selector);
        }
        
        Err(anyhow!("Timeout waiting for deployment {}/{} to be ready after 10m", namespace, deployment_name))
    }

    async fn wait_for_service_ping(&self, service_name: &str) -> Result<()> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()?;
        
        let url = format!("http://{}/health", service_name);
        let mut attempts = 0;
        
        while attempts < 150 { // 5 minutes timeout
            match client.get(&url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    tracing::info!("Service {} responded to health check!", service_name);
                    return Ok(());
                }
                Ok(resp) => {
                    tracing::debug!("Service {} ping: HTTP {} ({}s)", service_name, resp.status(), attempts * 2);
                }
                Err(e) => {
                    tracing::debug!("Service {} ping failed: {} ({}s)", service_name, e, attempts * 2);
                }
            }
            attempts += 1;
            sleep(Duration::from_secs(2)).await;
        }
        
        Err(anyhow!("Service {} failed to respond to health check after 300s", service_name))
    }

    async fn scale_down(&self, namespace: &str, deployment_name: &str) -> Result<()> {
        let deployments: Api<Deployment> = Api::namespaced(self.client.clone(), namespace);
        tracing::info!("Scaling down deployment {}/{} to 0...", namespace, deployment_name);
        let patch = json!({ "spec": { "replicas": 0 } });
        deployments.patch(deployment_name, &PatchParams::default(), &Patch::Merge(&patch)).await?;

        // Reset KEDA minReplicas
        if deployment_name == "openai-whisper-asr-webservice" {
            let _ = self.patch_httpscaledobject_min_replicas(namespace, "openai-whisper-asr-webservice-http", 0).await;
        } else if deployment_name == "whisperx" {
            let _ = self.patch_httpscaledobject_min_replicas(namespace, "whisperx-http", 0).await;
        }
        
        Ok(())
    }
}

impl KubeScalingRepository {
    /// Forces background AI services to scale down to 0 to free up GPU memory.
    async fn enforce_vram_priority(&self) -> Result<()> {
        let background_services = vec![
            ("llama-cpp", "llama-cpp"),
            // ("qwen-tts", "qwen3-tts"), // Do not preempt qwen3-tts as it's used in primary ingestion jobs
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

    async fn patch_httpscaledobject_min_replicas(&self, namespace: &str, resource_name: &str, min_replicas: i32) -> Result<()> {
        let gvk = GroupVersionKind::gvk("http.keda.sh", "v1alpha1", "HTTPScaledObject");
        let ar = ApiResource::from_gvk(&gvk);
        let api: Api<DynamicObject> = Api::namespaced_with(self.client.clone(), namespace, &ar);

        let patch = json!({
            "spec": {
                "replicas": {
                    "min": min_replicas
                }
            }
        });

        tracing::info!("Patching HTTPScaledObject {}/{} minReplicas to {}...", namespace, resource_name, min_replicas);
        api.patch(resource_name, &PatchParams::default(), &Patch::Merge(&patch)).await?;
        Ok(())
    }
}
