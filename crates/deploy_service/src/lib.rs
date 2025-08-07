mod docker;
pub mod error;

use crate::error::DeployError;
use chrono::Utc;
use common::{InstanceStatus, TaskInstance, compute_expiry};
use config_manager::get_config;
use data_models::Db;
use docker::DockerClient;
use port_manager::PortManager;
use uuid::Uuid;
use bollard::query_parameters::CreateContainerOptions;
use bollard::models::{HostConfig, PortBinding};
use std::collections::HashMap;
use bollard::query_parameters::StartContainerOptions;

pub struct Deployer {
    docker: DockerClient,
    ports: PortManager,
    db: Db,
}
pub struct DeployResult {
    pub instance: TaskInstance,
    pub endpoint: String,
}
impl Deployer {
    pub async fn new() -> Result<Self, DeployError> {
        let docker = DockerClient::new();
        let ports = PortManager::new().await?;
        let db = Db::new()?;
        Ok(Self { docker, ports, db })
    }

    pub async fn deploy(&mut self, task_name: &str)
                        -> Result<DeployResult, DeployError>
    {
        let cfg = get_config();
        let task_cfg = cfg.tasks
            .get(task_name)
            .unwrap_or(&cfg.tasks["_default"]);

        // 1. Reserve port
        let port = self.ports.reserve_port(Some(cfg.ports.default_ttl_secs)).await?;

        // 2. Build image (unchanged)
        let tag = format!("ctf-{}-{}", task_name, Uuid::new_v4());
        self.docker.build_image(task_name, &tag).await?;

        // 3. Create & start container
        let unique = Uuid::new_v4().simple().to_string();
        let hostname = format!("{}.{}", unique, cfg.routing.traefik_domain);
        let container_id = match cfg.routing.variant.as_str() {
            "port" => {
                // Port routing: publish host port = port
                let mut hc = HostConfig::default();
                hc.network_mode = Some("bridge".into());
                hc.port_bindings = Some({
                    let mut m = HashMap::new();
                    m.insert(
                        format!("{}/tcp", task_cfg.container_port),
                        Some(vec![PortBinding{ host_ip: Some("0.0.0.0".into()), host_port: Some(port.to_string()) }])
                    );
                    m
                });
                let opts = CreateContainerOptions { name: Some(tag.clone()) , platform: "".to_string()};
                let cfg = bollard::models::ContainerCreateBody{
                    image: Some(tag.clone()), host_config: Some(hc), ..Default::default()
                };
                self.docker.create_container(opts, cfg).await?
            },
            "traefik" => {
                // Traefik routing: no host port. Use labels + network
                let mut labels = HashMap::new();
                labels.insert("traefik.enable".into(), "true".into());
                // HTTP or TCP router
                if task_cfg.protocol=="http" {
                    labels.insert(
                        format!("traefik.http.routers.{}.rule", unique),
                        format!("Host(`{}`)", hostname)
                    );
                    labels.insert(
                        format!("traefik.http.services.{}.loadbalancer.server.port", unique),
                        task_cfg.container_port.to_string()
                    );
                } else {
                    labels.insert(
                        format!("traefik.tcp.routers.{}.entryPoints", unique),
                        cfg.routing.tcp_entry.clone()
                    );
                    labels.insert(
                        format!("traefik.tcp.routers.{}.rule", unique),
                        format!("HostSNI(`{}`)", hostname)
                    );
                    labels.insert(
                        format!("traefik.tcp.services.{}.loadbalancer.server.port", unique),
                        task_cfg.container_port.to_string()
                    );
                }
                let hc = HostConfig {
                    network_mode: Some("ctf-net".into()),
                    annotations: Some(labels.clone()),
                    ..Default::default()
                };
                let opts = CreateContainerOptions { name: Some(tag.clone()) , platform: "".to_string()};
                let cfg = bollard::models::ContainerCreateBody {
                    image: Some(tag.clone()), host_config: Some(hc), ..Default::default()
                };
                self.docker.create_container(opts, cfg).await?
            },
            v => return Err(DeployError::Config(format!("unknown routing {}",v))),
        };

        // 4. Start it
        self.docker
            .start_container(&container_id, None::<StartContainerOptions>)
            .await?;

        // 5. Compute expiry, record in DB
        let expires_at = compute_expiry(cfg.ports.default_ttl_secs);
        let inst = TaskInstance {
            id:       0,
            task_name: task_name.into(),
            container_id: container_id.clone(),
            port,
            created_at: Utc::now(),
            expires_at,
            status: InstanceStatus::Running,
            user_id: 0,  // API layer will overwrite
        };

        // 6. Construct the endpoint string
        let endpoint = match cfg.routing.variant.as_str() {
            "port" => {
                let proto = if task_cfg.protocol=="tcp" { "nc" } else { "http" };
                if task_cfg.protocol=="http" {
                    format!("http://{}:{}", cfg.routing.domain, port)
                } else {
                    format!("nc {} {}", cfg.routing.domain, port)
                }
            },
            "traefik" => {
                if task_cfg.protocol=="http" {
                    format!("http://{}", hostname)
                } else {
                    format!("nc {} {}", hostname, cfg.routing.tcp_entry)
                }
            },
            _ => unreachable!(),
        };

        Ok(DeployResult { instance: inst, endpoint })
    }

    pub async fn stop(&mut self, inst: &TaskInstance) -> Result<(), DeployError> {
        let _ = self.docker.stop_container(&inst.container_id).await;
        self.docker.remove_container(&inst.container_id).await?;
        self.ports.release_port(inst.port).await?;
        self.db
            .update_instance(inst.id, InstanceStatus::Stopped, Utc::now())?;
        Ok(())
    }
    pub async fn restart(&mut self, inst: &TaskInstance) -> Result<(), DeployError> {
        self.docker.restart_container(&inst.container_id).await?;

        let new_expiry = compute_expiry(get_config().ports.default_ttl_secs);
        self.db
            .update_instance(inst.id, InstanceStatus::Running, new_expiry)?;
        Ok(())
    }

    pub async fn extend(
        &mut self,
        inst: &TaskInstance,
        extra_ttl_secs: u64,
    ) -> Result<(), DeployError> {
        self.ports.extend_port(inst.port, extra_ttl_secs).await?;

        let new_expiry = compute_expiry(extra_ttl_secs);
        self.db.update_instance(inst.id, inst.status, new_expiry)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bollard::Docker;
    use bollard::query_parameters::RemoveContainerOptions;
    use tokio::time::{Duration, sleep};

    #[tokio::test]
    async fn deploy_and_stop() {
        let mut d = Deployer::new().await.unwrap();
        let inst = d.deploy("foo_task").await.unwrap().instance;

        sleep(Duration::from_secs(20)).await;

        d.stop(&inst).await.unwrap();

        let docker = Docker::connect_with_local_defaults().unwrap();
        let _ = docker
            .remove_container(
                &inst.container_id,
                Some(RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                }),
            )
            .await;
    }
}
