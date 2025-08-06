use thiserror::Error;
use common::ServiceError;
use deploy_service::error::DeployError;
use port_manager::PortError;

#[derive(Debug, Error)]
pub enum SchedulerError {
    #[error("service error: {0}")]
    Service(#[from] ServiceError),

    #[error("deploy error: {0}")]
    Deploy(#[from] DeployError),

    #[error("port manager error: {0}")]
    Port(#[from] PortError),
}
