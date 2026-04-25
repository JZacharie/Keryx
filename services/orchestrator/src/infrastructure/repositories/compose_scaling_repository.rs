use async_trait::async_trait;
use anyhow::{Result, anyhow};
use keryx_core::domain::ports::scaling_repository::ScalingRepository;
use bollard::Docker;
use bollard::container::StopContainerOptions;
use std::time::Duration;
use tokio::time::sleep;

pub struct ComposeScalingRepository {
    docker: Docker,
}

impl ComposeScalingRepository {
    pub fn new() -> Result<Self> {
        let docker = Docker::connect_with_local_defaults()?;
        Ok(Self { docker })
    }
}

#[async_trait]
impl ScalingRepository for ComposeScalingRepository {
    async fn scale_up(&self, namespace: &str, deployment_name: &str) -> Result<()> {
        // Ensure mutual exclusion on GPU by preempting other AI services
        tracing::info!("Ensuring exclusive access for {}. Preempting other AI services...", deployment_name);
        let _ = self.preempt_conflicting_services(namespace, deployment_name).await;

        // In Docker Compose, the deployment_name usually maps to the container name or service name
        // We'll try to start the container.
        tracing::info!("Docker Compose: Starting container {}...", deployment_name);
        
        match self.docker.start_container::<String>(deployment_name, None).await {
            Ok(_) => tracing::info!("Successfully sent start command to container {}", deployment_name),
            Err(e) => {
                tracing::warn!("Failed to start container {} (it might be already running): {}", deployment_name, e);
            }
        }
        
        // Discovery port for compose: try to find it or use a default.
        // Actually, we'll try to use a convention or hardcoded mapping if needed, 
        // but for now we follow the same pattern as Kube where the port is passed.
        let port = match deployment_name {
            "keryx-extractor" => 8010,
            "keryx-dewatermark" => 8011,
            "keryx-voice-extractor" => 8012,
            "keryx-video-composer" => 8013,
            "keryx-video-generator" => 8014,
            "keryx-voice-cloner" => 9880,
            "keryx-pptx-builder" => 8002,
            _ => 80,
        };

        // Wait for ready via ping
        self.wait_for_service_ping(namespace, deployment_name, port).await
    }

    async fn wait_for_service_ping(&self, _namespace: &str, service_name: &str, port: u16) -> Result<()> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()?;
        
        // Map service name to actual URL if needed (in compose, service name is usually host name)
        let endpoints = vec!["/health", "/docs", "/"];
        let mut attempts = 0;
        
        tracing::info!("Waiting for service {} to respond...", service_name);
        
        while attempts < 300 { // 10 minutes timeout (300 * 2s)
            for endpoint in &endpoints {
                let url = format!("http://{}:{}{}", service_name, port, endpoint);
                match client.get(&url).send().await {
                    Ok(resp) if resp.status().is_success() => {
                        tracing::info!("Service {} is UP!", service_name);
                        return Ok(());
                    }
                    _ => {}
                }
            }
            attempts += 1;
            sleep(Duration::from_secs(2)).await;
        }
        
        Err(anyhow!("Service {} failed to respond after 1 minute", service_name))
    }

    async fn scale_down(&self, _namespace: &str, deployment_name: &str) -> Result<()> {
        tracing::info!("Docker Compose: Stopping container {}...", deployment_name);
        
        let options = Some(StopContainerOptions { t: 10 });
        match self.docker.stop_container(deployment_name, options).await {
            Ok(_) => tracing::info!("Successfully stopped container {}", deployment_name),
            Err(e) => {
                tracing::warn!("Failed to stop container {}: {}", deployment_name, e);
            }
        }
        
        Ok(())
    }

    /// Preempts all other AI services to ensure the current one has full access to resources.
    async fn preempt_conflicting_services(&self, _namespace: &str, current_container: &str) -> Result<()> {
        let ai_services = vec![
            "keryx-dewatermark",
            "keryx-voice-extractor",
            "keryx-video-generator",
            "keryx-voice-cloner",
            "keryx-voice-cloner-gpt",
            "keryx-diffusion-engine",
            "keryx-video-composer",
        ];

        let options = Some(StopContainerOptions { t: 5 });

        for container in ai_services {
            if container == current_container {
                continue;
            }
            // Stop container
            let _ = self.docker.stop_container(container, options.clone()).await;
        }
        
        // Small delay to allow processes to exit
        sleep(Duration::from_secs(2)).await;
        
        Ok(())
    }
}
