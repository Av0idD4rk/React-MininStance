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
        let expires_at = compute_expiry(get_config().ports.default_ttl_secs);
        let inst = TaskInstance {
            id: 0,
            task_name: task_name.to_string(),
            container_id: container_id.clone(),
            port,
            created_at: Utc::now(),
            expires_at,
            status: InstanceStatus::Running,
        };
        let saved = self.db.create_instance(&inst)?;
        Ok(saved)
    }

    pub async fn stop(&mut self, instance: &TaskInstance) -> Result<(), DeployError> {
        self.docker.stop_container(&instance.container_id).await?;
        self.ports.release_port(instance.port).await?;
        self.db
            .update_instance(instance.id, InstanceStatus::Stopped, Utc::now())?;
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
