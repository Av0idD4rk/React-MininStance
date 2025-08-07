mod docker;
pub mod error;

use crate::error::DeployError;
use bollard::models::{ContainerCreateBody, HostConfig};
use bollard::query_parameters::CreateContainerOptions;
use bollard::query_parameters::StartContainerOptions;
use chrono::Utc;
use common::{InstanceStatus, TaskInstance, compute_expiry};
use config_manager::get_config;
use data_models::Db;
use docker::DockerClient;
use std::collections::HashMap;
use uuid::Uuid;

pub struct Deployer {
    docker: DockerClient,
    db: Db,
}
pub struct DeployResult {
    pub instance: TaskInstance,
}
impl Deployer {
    pub async fn new() -> Result<Self, DeployError> {
        let docker = DockerClient::new();
        let db = Db::new()?;
        Ok(Self { docker, db })
    }

    pub async fn deploy(&mut self, task_name: &str) -> Result<DeployResult, DeployError> {
        let cfg = get_config();
        let task_cfg = cfg.tasks.get(task_name).unwrap_or(&cfg.tasks["_default"]);


        // 2. Build the image
        let tag = format!("ctf-{}-{}", task_name, Uuid::new_v4());
        self.docker.build_image(task_name, &tag).await?;

        // 3. Generate a unique token & hostname (for Traefik mode)
        let unique = Uuid::new_v4().simple().to_string();
        let hostname = format!("{}.{}", unique, cfg.routing.traefik_domain);

        // 4. Create the container
        let container_id = {
                // no published ports, just labels + custom network
                let mut labels = HashMap::new();
                labels.insert("traefik.enable".into(), "true".into());
                labels.insert(format!("traefik.docker.network"), "ctf-net".into());

                if task_cfg.protocol == "http" {
                    // HTTP router
                    labels.insert(
                        format!("traefik.http.routers.{}.rule", unique),
                        format!("Host(`{}`)", hostname),
                    );
                    labels.insert(
                        format!("traefik.http.routers.{}.entrypoints", unique),
                        cfg.routing.http_entry.clone(),
                    );
                    labels.insert(
                        format!("traefik.http.services.{}.loadbalancer.server.port", unique),
                        task_cfg.container_port.to_string(),
                    );
                } else {
                    // TCP router
                    labels.insert(
                        format!("traefik.tcp.routers.{}.rule", unique),
                        format!("HostSNI(`{}`)", hostname),
                    );
                    labels.insert(
                        format!("traefik.tcp.routers.{}.entrypoints", unique),
                        cfg.routing.tcp_entry.clone(),
                    );
                    labels.insert(
                        format!("traefik.tcp.services.{}.loadbalancer.server.port", unique),
                        task_cfg.container_port.to_string(),
                    );
                }
                let cont_cfg = &get_config().containers;

                let cpu_period = 100_000;
                let cpu_quota = (cont_cfg.cpu_quota * cpu_period as f64) as i64;

                let mut hc = HostConfig {
                    memory: Some(cont_cfg.memory_limit),
                    memory_swap: Some(cont_cfg.swap_limit),
                    cpu_period: Some(cpu_period),
                    cpu_quota: Some(cpu_quota),
                    pids_limit: Some(cont_cfg.pids_limit as i64),

                    readonly_rootfs: Some(cont_cfg.read_only_rootfs),
                    cap_drop: if cont_cfg.drop_all_capabilities {
                        Some(vec!["ALL".into()])
                    } else {
                        None
                    },
                    cap_add: if cont_cfg.add_capabilities.is_empty() {
                        None
                    } else {
                        Some(cont_cfg.add_capabilities.clone())
                    },
                    security_opt: Some(vec![if cont_cfg.enable_no_new_privileges {
                        "no-new-privileges".to_string()
                    } else {
                        "no-new-privileges=false".to_string()
                    }]),
                    tmpfs: if cont_cfg.enable_tmpfs {
                        let mut m = HashMap::new();
                        m.insert(
                            "/tmp".to_string(),
                            format!("rw,size={}", cont_cfg.tmpfs_size),
                        );
                        Some(m)
                    } else {
                        None
                    },
                    ..Default::default()
                };
                let opts = CreateContainerOptions {
                    name: Some(tag.clone()),
                    platform: "".to_string(),
                };
                let body = ContainerCreateBody {
                    image: Some(tag.clone()),
                    labels: Some(labels),
                    host_config: Some(hc),
                    ..Default::default()
                };
                self.docker.create_container(opts, body).await?
            };


        self.docker
            .start_container(&container_id, None::<StartContainerOptions>)
            .await?;

        let endpoint = {
                if task_cfg.protocol == "http" {
                    format!("http://{}", hostname)
                } else {
                    format!("nc {} {}", hostname, 9000)
                }
            };


        let inst = TaskInstance {
            id: 0,
            task_name: task_name.to_string(),
            container_id: container_id.clone(),
            created_at: Utc::now(),
            expires_at: compute_expiry(cfg.ports.default_ttl_secs),
            status: InstanceStatus::Running,
            endpoint: endpoint.clone(),
            user_id: 0, // will be set by create_instance_for_user
        };

        Ok(DeployResult { instance: inst })
    }

    pub async fn stop(&mut self, inst: &TaskInstance) -> Result<(), DeployError> {
        let _ = self.docker.stop_container(&inst.container_id).await;
        self.docker.remove_container(&inst.container_id).await?;
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
