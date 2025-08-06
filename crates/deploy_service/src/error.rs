use thiserror::Error;
use bollard::errors::Error as BollardError;
use common::ServiceError;

#[derive(Debug, Error)]
pub enum DeployError {
    #[error("docker error: {0}")]
    Docker(#[from] BollardError),
    #[error("service error: {0}")]
    Service(#[from] ServiceError),
    #[error("build failed: {0}")]
    Build(String),
    #[error("port manager error: {0}")]
    Port(#[from] port_manager::PortError),
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
}
