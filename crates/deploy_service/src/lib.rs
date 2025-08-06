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

pub struct Deployer {
    docker: DockerClient,
    ports: PortManager,
    db: Db,
}

impl Deployer {
    pub async fn new() -> Result<Self, DeployError> {
        let docker = DockerClient::new();
        let ports = PortManager::new().await?;
        let db = Db::new()?;
        Ok(Self { docker, ports, db })
    }

    pub async fn deploy(&mut self, task_name: &str) -> Result<TaskInstance, DeployError> {
        let port = self.ports.reserve_port(None).await?;

        let tag = format!("ctf-{}-{}", task_name, Uuid::new_v4());
        self.docker.build_image(task_name, &tag).await?;

        let cfg = get_config();
        let internal = cfg.ports.default.to_string();
        let container_id = self.docker.start_container(&tag, port, &internal).await?;
        let expires_at = compute_expiry(cfg.ports.default_ttl_secs);
        let inst = TaskInstance {
            id: 0,
            task_name: task_name.to_string(),
            container_id: container_id.clone(),
            port,
            created_at: Utc::now(),
            expires_at,
            status: InstanceStatus::Running,
            user_id: 0,
        };
        Ok(inst)
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
        let inst = d.deploy("foo_task").await.unwrap();

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
