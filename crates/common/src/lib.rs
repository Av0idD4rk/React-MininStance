use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{subscriber::set_global_default, Level};
use tracing_subscriber::FmtSubscriber;

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("database error: {0}")]
    Db(#[from] diesel::result::Error),
    #[error("redis error: {0}")]
    Redis(#[from] redis::RedisError),
    #[error("port error: {0}")]
    Port(#[from] port_manager::PortError),
    #[error("config error: {0}")]
    Config(#[from] config_manager::ConfigError),
    #[error("other: {0}")]
    Other(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TaskInstance {
    pub id: i32,
    pub task_name: String,
    pub container_id: String,
    pub port: u16,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub status: InstanceStatus,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
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
