mod error;

use chrono::Utc;
use tokio::time::{sleep, Duration};
use tracing::{info, error};

use config_manager::get_config;
use common::InstanceStatus;
use data_models::Db;
use deploy_service::Deployer;
use crate::error::SchedulerError;

pub async fn run() -> Result<(), SchedulerError> {
    // 1. Bring up your deployer & DB once
    let mut deploy = Deployer::new().await?;
    let db         = Db::new()?;
    let interval   = get_config().scheduler.poll_interval_secs;

    loop {
        let now     = Utc::now();
        let expired = db.list_expired_instances(now)?;

        for inst in expired {
            info!("Instance {} expired → stopping container {}", inst.id, inst.container_id);

            if let Err(e) = deploy.stop(&inst).await {
                error!("Failed to stop {}: {}", inst.id, e);
            }

            if let Err(e) = db.update_instance_status(inst.id, InstanceStatus::Stopped) {
                error!("Failed to mark {} stopped in DB: {}", inst.id, e);
            }
        }

        sleep(Duration::from_secs(interval)).await;
    }
}
