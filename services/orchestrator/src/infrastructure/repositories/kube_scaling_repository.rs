use async_trait::async_trait;
use anyhow::{Result, anyhow};
use keryx_core::domain::ports::scaling_repository::ScalingRepository;
use kube::{Client, Api, core::DynamicObject, discovery::ApiResource};
use kube::api::{Patch, PatchParams, GroupVersionKind};
use k8s_openapi::api::apps::v1::Deployment;
use k8s_openapi::api::core::v1::{Pod, Service};
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
        // We attempt a generic patch based on the deployment name with a "-http" suffix.
        let hso_name = format!("{}-http", deployment_name);
        if let Err(e) = self.patch_httpscaledobject_min_replicas(namespace, &hso_name, 1).await {
            // We log as debug because not all services necessarily have an HTTPScaledObject
            tracing::debug!("No HTTPScaledObject found or failed to patch for {}: {}", hso_name, e);
        }
        
        // Specific legacy/external overrides if they don't follow the pattern
        if deployment_name == "whisperx" {
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
                    tracing::info!("Deployment {}/{} is ready at K8s level. Waiting for service response...", namespace, deployment_name);
                    
                    // We try to ping the service to ensure the ML model is actually loaded in VRAM.
                    // We discover the port from the Service object.
                    let service_port = if let Ok(s) = Api::<Service>::namespaced(self.client.clone(), namespace).get(deployment_name).await {
                         s.spec.and_then(|spec| spec.ports.and_then(|p| p.first().map(|p| p.port))).unwrap_or(80) as u16
                    } else {
                        80
                    };

                    if let Err(e) = self.wait_for_service_ping(deployment_name, service_port).await {
                        tracing::warn!("Service {}/{} ready replicas > 0 but health check failed on port {}: {}. Continuing anyway...", namespace, deployment_name, service_port, e);
                    }
                    
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
        // Try to identify component name from deployment name (e.g. keryx-extractor -> extractor)
        let component_name = deployment_name.split('-').last().unwrap_or(deployment_name);
        let label_selector = format!("app.kubernetes.io/component={}", component_name);
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

    async fn wait_for_service_ping(&self, service_name: &str, port: u16) -> Result<()> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()?;
        
        // List of endpoints to try
        let endpoints = vec!["/health", "/docs", "/"];
        let mut attempts = 0;
        
        while attempts < 60 { // 5 minutes (60 * 5s)
            for endpoint in &endpoints {
                let url = format!("http://{}:{}{}", service_name, port, endpoint);
                match client.get(&url).send().await {
                    Ok(resp) if resp.status().is_success() => {
                        tracing::info!("Service {} responded to health check on {}!", service_name, endpoint);
                        return Ok(());
                    }
                    Ok(resp) => {
                        tracing::debug!("Service {} ping {}: HTTP {} ({}s)", service_name, endpoint, resp.status(), attempts * 5);
                    }
                    Err(e) => {
                        tracing::debug!("Service {} ping {} failed: {} ({}s)", service_name, endpoint, e, attempts * 5);
                    }
                }
            }
            attempts += 1;
            sleep(Duration::from_secs(5)).await;
        }
        
        Err(anyhow!("Service {} failed to respond to any health check (tried {:?}) after 300s", service_name, endpoints))
    }

    async fn scale_down(&self, namespace: &str, deployment_name: &str) -> Result<()> {
        let deployments: Api<Deployment> = Api::namespaced(self.client.clone(), namespace);
        tracing::info!("Scaling down deployment {}/{} to 0...", namespace, deployment_name);
        let patch = json!({ "spec": { "replicas": 0 } });
        deployments.patch(deployment_name, &PatchParams::default(), &Patch::Merge(&patch)).await?;

        // Reset KEDA minReplicas generically
        let hso_name = format!("{}-http", deployment_name);
        let _ = self.patch_httpscaledobject_min_replicas(namespace, &hso_name, 0).await;

        // Specific legacy/external overrides
        if deployment_name == "whisperx" {
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
