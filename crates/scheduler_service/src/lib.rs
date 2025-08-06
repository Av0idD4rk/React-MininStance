pub mod error;

use config_manager::get_config;
use port_manager::PortManager;
use deploy_service::Deployer;
use data_models::Db;
use crate::error::SchedulerError;
use tokio::time::{sleep, Duration};
use tracing::{info, error};

pub async fn run() -> Result<(), SchedulerError> {
    let mut ports   = PortManager::new().await?;
    let mut deploy = Deployer::new().await?;
    let db          = Db::new()?;
    let interval    = get_config().scheduler.poll_interval_secs;

    loop {
        let expired = ports.get_expired().await?;
        for port in expired {
            if let Some(inst) = db.find_by_port(port as i32)? {
                info!("Port {} expired → stopping instance {}", port, inst.id);

                if let Err(e) = deploy.stop(&inst).await {
                    error!("Failed to stop {}: {}", inst.id, e);
                }
            }

            let _ = ports.release_port(port).await;
        }

        sleep(Duration::from_secs(interval)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use common::{TaskInstance, InstanceStatus};
    use tokio::time::{sleep, Duration};
    use redis::AsyncCommands;
    use config_manager::get_config;

    const FREE_SET: &str   = "ports:free";
    const IN_USE_ZSET: &str= "ports:in_use";

    async fn flush_port_state() {
        let cfg = get_config();
        let client = redis::Client::open(cfg.redis.url.to_owned()).unwrap();
        // Use the async, multiplexed connection
        let mut conn = client
            .get_multiplexed_tokio_connection()
            .await
            .unwrap();

        let _: () = conn.del(FREE_SET).await.unwrap();
        let _: () = conn.del(IN_USE_ZSET).await.unwrap();
    }

    #[tokio::test]
    async fn scheduler_stops_expired() {
        flush_port_state().await;

        let mut ports   = PortManager::new().await.unwrap();
        let db          = Db::new().unwrap();
        let mut deploy  = Deployer::new().await.unwrap();

        let port = ports.reserve_port(Some(0)).await.unwrap();
        assert_eq!(port, get_config().ports.min, "expected first port");

        let now = Utc::now();
        let fake = TaskInstance {
            id: 0,
            task_name: "foo_task".to_string(),
            container_id: "dummy-id".to_string(),
            port,
            created_at: now,
            expires_at: now,
            status: InstanceStatus::Running,
        };
        let _inst = db.create_instance(&fake).unwrap();

        ports.extend_port(port, 0).await.unwrap();
        sleep(Duration::from_millis(50)).await;

        let expired = ports.get_expired().await.unwrap();
        assert_eq!(expired, vec![port], "port should show as expired");

        for p in expired {
            let i = db.find_by_port(p as i32).unwrap().unwrap();
            deploy.stop(&i).await.unwrap();
        }

        let expired_after = ports.get_expired().await.unwrap();
        assert!(!expired_after.contains(&port));

        let fetched = db.find_by_port(port as i32).unwrap().unwrap();
        assert_eq!(fetched.status, InstanceStatus::Stopped);
    }
}
