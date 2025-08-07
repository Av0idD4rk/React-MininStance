use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{subscriber::set_global_default, Level};
use tracing_subscriber::FmtSubscriber;

use config_manager::ConfigError;
use port_manager::PortError;
use redis::RedisError;
use r2d2::Error as R2d2Error;

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("database error: {0}")]
    Db(#[from] diesel::result::Error),

    #[error("redis error: {0}")]
    Redis(#[from] RedisError),

    #[error("port error: {0}")]
    Port(#[from] PortError),

    #[error("config error: {0}")]
    Config(#[from] ConfigError),

    #[error("other: {0}")]
    Other(String),
    #[error("pool error: {0}")]
    Pool(#[from] R2d2Error),
}

/// Represents a running task instance
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TaskInstance {
    pub id: i32,
    pub task_name: String,
    pub container_id: String,
    pub port: u16,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub status: InstanceStatus,
    pub user_id: i32,
    pub endpoint: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum InstanceStatus {
    Running,
    Stopped,
    Expired,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserSession {
    pub session_id: String,
    pub user_id: i32,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub created_at: DateTime<Utc>,
}

pub fn init_logging() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    set_global_default(subscriber)
        .expect("setting default tracing subscriber failed");
}

pub fn compute_expiry(ttl_secs: u64) -> DateTime<Utc> {
    Utc::now() + chrono::Duration::seconds(ttl_secs as i64)
}

pub fn ttl_secs_until(expiry: DateTime<Utc>) -> u64 {
    let now = Utc::now();
    if expiry > now {
        (expiry - now).num_seconds() as u64
    } else {
        0
    }
}

impl InstanceStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            InstanceStatus::Running => "Running",
            InstanceStatus::Stopped => "Stopped",
            InstanceStatus::Expired => "Expired",
        }
    }
}
