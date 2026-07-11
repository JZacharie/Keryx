use bollard::container::{
    Config, CreateContainerOptions, LogOutput, LogsOptions, RemoveContainerOptions,
    StartContainerOptions, StopContainerOptions,
};
use bollard::secret::{HostConfig, Mount, PortBinding};
use bollard::Docker;
use futures::StreamExt;
use std::collections::HashMap;
use std::fmt::Write;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, OwnedMutexGuard};

#[derive(Clone, Debug)]
pub struct WorkerSpec {
    pub image: String,
    pub container_name: String,
    pub env: Vec<String>,
    pub needs_gpu: bool,
    pub port: u16,
    pub volumes: Vec<(String, String)>,
}

pub struct WorkerHandle {
    pub container_name: String,
    pub needs_gpu: bool,
    _gpu_guard: Option<OwnedMutexGuard<()>>,
}

impl WorkerHandle {
    fn new(container_name: String, needs_gpu: bool, guard: Option<OwnedMutexGuard<()>>) -> Self {
        Self {
            container_name,
            needs_gpu,
            _gpu_guard: guard,
        }
    }
}

pub struct ContainerManager {
    docker: Docker,
    gpu_mutex: Arc<Mutex<()>>,
    network: String,
    http_client: reqwest::Client,
}

impl ContainerManager {
    pub fn new(network: impl Into<String>) -> anyhow::Result<Self> {
        let docker = Docker::connect_with_local_defaults()?;
        Ok(Self {
            docker,
            gpu_mutex: Arc::new(Mutex::new(())),
            network: network.into(),
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(300))
                .build()?,
        })
    }

    pub async fn start_worker(&self, spec: &WorkerSpec) -> anyhow::Result<WorkerHandle> {
        tracing::info!(
            "[{}] Acquiring worker slot (gpu={})",
            spec.container_name, spec.needs_gpu
        );
        let gpu_guard = if spec.needs_gpu {
            tracing::info!("[{}] Waiting for GPU mutex...", spec.container_name);
            let guard = self.gpu_mutex.clone().lock_owned().await;
            tracing::info!("[{}] GPU mutex acquired", spec.container_name);
            Some(guard)
        } else {
            None
        };

        let container_name = self.start_container_inner(spec).await?;

        Ok(WorkerHandle::new(container_name, spec.needs_gpu, gpu_guard))
    }

    async fn start_container_inner(&self, spec: &WorkerSpec) -> anyhow::Result<String> {
        tracing::info!(
            "[{}] Creating container from image '{}'...",
            spec.container_name, spec.image
        );

        let mut port_bindings = HashMap::new();
        port_bindings.insert(
            format!("{}/tcp", spec.port),
            Some(vec![PortBinding {
                host_port: None,
                host_ip: None,
            }]),
        );

        let mut exposed_ports = HashMap::new();
        exposed_ports.insert(format!("{}/tcp", spec.port), HashMap::new());

        let mut mounts = Vec::new();
        for (host_path, container_path) in &spec.volumes {
            mounts.push(Mount {
                target: Some(container_path.clone()),
                source: Some(host_path.clone()),
                typ: Some(bollard::secret::MountTypeEnum::BIND),
                read_only: Some(false),
                ..Default::default()
            });
        }

        let mut device_requests = Vec::new();
        if spec.needs_gpu {
            device_requests.push(bollard::secret::DeviceRequest {
                driver: Some("nvidia".to_string()),
                device_ids: None,
                count: Some(-1),
                capabilities: Some(vec![vec!["gpu".to_string()]]),
                options: None,
            });
        }

        let host_config = HostConfig {
            network_mode: Some(self.network.clone()),
            port_bindings: Some(port_bindings),
            mounts: Some(mounts),
            init: Some(true),
            auto_remove: Some(true),
            device_requests: if device_requests.is_empty() {
                None
            } else {
                Some(device_requests)
            },
            ..Default::default()
        };

        let config = Config {
            image: Some(spec.image.clone()),
            env: Some(spec.env.clone()),
            exposed_ports: Some(exposed_ports),
            host_config: Some(host_config),
            ..Default::default()
        };

        let options = CreateContainerOptions {
            name: spec.container_name.as_str(),
            ..Default::default()
        };

        self.docker.create_container(Some(options), config).await?;
        tracing::info!("[{}] Container created, starting...", spec.container_name);

        self.docker
            .start_container(&spec.container_name, None::<StartContainerOptions<String>>)
            .await?;
        tracing::info!("[{}] Container started, waiting for health check...", spec.container_name);

        let health_url = format!(
            "http://{}:{}/health",
            spec.container_name, spec.port
        );

        self.wait_for_health(&health_url, spec.container_name.as_str(), 60).await?;

        tracing::info!("[{}] Health check OK, container is ready", spec.container_name);

        Ok(spec.container_name.clone())
    }

    pub async fn stop_worker(&self, handle: &WorkerHandle) -> anyhow::Result<()> {
        let name = &handle.container_name;

        tracing::info!("[{}] Capturing container logs before shutdown...", name);
        let container_logs = self.get_container_logs(name).await;
        if !container_logs.is_empty() {
            for line in container_logs.lines() {
                if !line.is_empty() {
                    tracing::info!("[{}:log] {}", name, line);
                }
            }
        } else {
            tracing::info!("[{}] (no container logs captured)", name);
        }

        tracing::info!("[{}] Stopping container...", name);
        let stop_opts = StopContainerOptions { t: 10 };
        let _ = self.docker.stop_container(name, Some(stop_opts)).await;

        tracing::info!("[{}] Removing container...", name);
        let remove_opts = RemoveContainerOptions {
            force: true,
            ..Default::default()
        };
        let _ = self.docker.remove_container(name, Some(remove_opts)).await;

        tracing::info!("[{}] Container cleaned up", name);

        // GPU guard is dropped here, releasing the GPU mutex
        Ok(())
    }

    async fn get_container_logs(&self, container_name: &str) -> String {
        let options = LogsOptions::<String> {
            follow: false,
            stdout: true,
            stderr: true,
            since: 0,
            until: 0,
            timestamps: false,
            tail: "all".to_string(),
        };

        let mut logs = String::new();
        let mut stream = self.docker.logs(container_name, Some(options));
        while let Some(item) = stream.next().await {
            match item {
                Ok(log) => {
                    let msg = String::from_utf8_lossy(match &log {
                        LogOutput::StdOut { message } => message,
                        LogOutput::StdErr { message } => message,
                        LogOutput::StdIn { message } => message,
                        LogOutput::Console { message } => message,
                    });
                    let _ = write!(logs, "{}", msg);
                }
                Err(_) => break,
            }
        }
        logs
    }

    async fn wait_for_health(&self, url: &str, container_name: &str, max_retries: u32) -> anyhow::Result<()> {
        for attempt in 1..=max_retries {
            match self.http_client.get(url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    tracing::info!("[{}] Health check succeeded on attempt {}/{}", container_name, attempt, max_retries);
                    return Ok(());
                }
                Ok(resp) => {
                    let status = resp.status();
                    if attempt % 10 == 0 || attempt == 1 {
                        tracing::info!("[{}] Health check attempt {}/{} returned {}", container_name, attempt, max_retries, status);
                    }
                }
                Err(e) => {
                    if attempt % 10 == 0 || attempt == 1 {
                        tracing::info!("[{}] Health check attempt {}/{} failed: {}", container_name, attempt, max_retries, e);
                    }
                }
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
        Err(anyhow::anyhow!(
            "Container {} not healthy after {} attempts",
            container_name,
            max_retries
        ))
    }

    pub async fn call_worker(
        &self,
        container_name: &str,
        port: u16,
        path: &str,
        body: serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let url = format!("http://{}:{}{}", container_name, port, path);
        let resp = self
            .http_client
            .post(&url)
            .json(&body)
            .send()
            .await?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Worker returned {}: {}", status, text));
        }
        Ok(resp.json().await?)
    }
}
