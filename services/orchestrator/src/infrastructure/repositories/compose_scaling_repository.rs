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
        tracing::info!("[DOCKER] scale_up requested for container: {}", deployment_name);
        
        // Ensure mutual exclusion on GPU by preempting other AI services
        tracing::debug!("[DOCKER] Preempting other AI services...");
        let _ = self.preempt_conflicting_services(namespace, deployment_name).await;
 
        tracing::info!("[DOCKER] Starting container {}...", deployment_name);
        
        match self.docker.start_container::<String>(deployment_name, None).await {
            Ok(_) => tracing::info!("[DOCKER] SUCCESS: Sent start command to container {}", deployment_name),
            Err(e) => {
                tracing::warn!("[DOCKER] WARNING: Failed to start container {} (might be already running or missing): {}", deployment_name, e);
            }
        }
        
        let port = 8000;
 
        tracing::info!("[DOCKER] Waiting for service {} on port {}...", deployment_name, port);
        self.wait_for_service_ping(namespace, deployment_name, port).await
    }
 
    async fn wait_for_service_ping(&self, _namespace: &str, service_name: &str, port: u16) -> Result<()> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()?;
        
        let endpoints = vec!["/health", "/docs", "/"];
        let mut attempts = 0;
        
        while attempts < 30 { // Reduced to 30 attempts for faster debugging
            for endpoint in &endpoints {
                let url = format!("http://{}:{}{}", service_name, port, endpoint);
                tracing::debug!("[DOCKER] Pinging {} (Attempt {}/30)...", url, attempts + 1);
                match client.get(&url).send().await {
                    Ok(resp) if resp.status().is_success() => {
                        tracing::info!("[DOCKER] Service {} is UP and RESPONDING!", service_name);
                        return Ok(());
                    }
                    Ok(resp) => {
                        tracing::debug!("[DOCKER] Service {} responded with status: {}", service_name, resp.status());
                    }
                    Err(e) => {
                        tracing::trace!("[DOCKER] Ping error for {}: {}", service_name, e);
                    }
                }
            }
            attempts += 1;
            sleep(Duration::from_secs(2)).await;
        }
        
        tracing::error!("[DOCKER] TIMEOUT: Service {} failed to respond after 1 minute", service_name);
        Err(anyhow!("Service {} failed to respond after 1 minute", service_name))
    }
 
    async fn scale_down(&self, _namespace: &str, deployment_name: &str) -> Result<()> {
        tracing::info!("[DOCKER] scale_down requested for container: {}", deployment_name);
        
        let options = Some(StopContainerOptions { t: 5 });
        match self.docker.stop_container(deployment_name, options).await {
            Ok(_) => tracing::info!("[DOCKER] SUCCESS: Stopped container {}", deployment_name),
            Err(e) => {
                tracing::warn!("[DOCKER] WARNING: Failed to stop container {}: {}", deployment_name, e);
            }
        }
        
        Ok(())
    }
 
    /// Preempts all other AI services to ensure the current one has full access to resources.
    async fn preempt_conflicting_services(&self, _namespace: &str, current_container: &str) -> Result<()> {
        let ai_services = vec![
            "keryx-extractor",
            "keryx-dewatermark",
            "keryx-voice-extractor",
            "keryx-video-generator",
            "keryx-voice-cloner",
            "keryx-voice-cloner-gpt",
            "keryx-diffusion-engine",
            "keryx-video-composer",
            "keryx-pptx-builder",
        ];
 
        let options = Some(StopContainerOptions { t: 5 });
 
        for container in ai_services {
            if container == current_container {
                continue;
            }
            tracing::debug!("[DOCKER] Preemptively stopping {}...", container);
            let _ = self.docker.stop_container(container, options.clone()).await;
        }
        
        sleep(Duration::from_secs(1)).await;
        Ok(())
    }
}
