
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::fs;
use thiserror::Error;
use std::{env, path::PathBuf};

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read config file: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse TOML: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("Config.toml not found in any parent directory")]
    NotFound,
}


#[derive(Deserialize)]
pub struct RoutingConfig {
    pub variant: String,           // "port" | "traefik"
    pub domain: String,            // e.g. "localhost"
    pub traefik_domain: String,    // e.g. "ctf.local"
    pub http_entry: String,        // e.g. "web"
    pub tcp_entry: String,         // e.g. "tcp"
}

#[derive(Deserialize)]
pub struct TaskConfig {
    #[serde(default="default_protocol")]
    pub protocol: String,
    #[serde(default="default_cport")]
    pub container_port: u16,
}
fn default_protocol() -> String { "http".into() }
fn default_cport()   -> u16    { 3000 }

#[derive(Deserialize)]
pub struct Config {
    pub routing: RoutingConfig,
    pub tasks: std::collections::HashMap<String, TaskConfig>,
    pub ports: Ports,
    pub database: Database,
    pub redis: Redis,
    pub captcha: Captcha,
    pub scheduler: Scheduler,
    pub sessions: Sessions,
}

#[derive(Debug, Deserialize)]
pub struct Ports {
    pub min: u16,
    pub max: u16,
    pub default: u16,
    pub default_ttl_secs: u64,
    pub extend_time_secs: u64,
}

#[derive(Debug, Deserialize)]
pub struct Scheduler {
    pub poll_interval_secs: u64,
}

#[derive(Debug, Deserialize)]
pub struct Database {
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct Redis {
    pub url: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Sessions {
    pub ttl_hours: i64,
    pub max_instances: u16,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Captcha {
    pub provider: String,
    pub site_key: String,
    pub secret_key: String,
    pub verify_url: String,
}

fn find_config_file() -> Result<PathBuf, ConfigError> {
    let mut dir = env::current_dir()?;
    loop {
        let candidate = dir.join("Config.toml");
        if candidate.is_file() {
            return Ok(candidate);
        }
        if !dir.pop() {
            break;
        }
    }
    Err(ConfigError::NotFound)
}

/// Lazily load & parse the config once
static CONFIG: Lazy<Config> = Lazy::new(|| {
    let path = find_config_file().expect("Config.toml not found");
    let toml_str = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("unable to read {}: {}", path.display(), e));
    toml::from_str(&toml_str)
        .unwrap_or_else(|e| panic!("invalid TOML in {}: {}", path.display(), e))
});

/// Public accessor
pub fn get_config() -> &'static Config {
    &CONFIG
}