
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::fs;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read config file: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse TOML: {0}")]
    Toml(#[from] toml::de::Error),
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub ports: Ports,
    pub database: Database,
    pub redis: Redis,
    pub captcha: Captcha,
}

#[derive(Debug, Deserialize)]
pub struct Ports {
    pub min: u16,
    pub max: u16,
    pub default: u16,
    pub default_ttl_secs: u64,
}

#[derive(Debug, Deserialize)]
pub struct Database {
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct Redis {
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct Captcha {
    pub provider: String,
    pub site_key: String,
    pub secret_key: String,
}

static CONFIG: Lazy<Config> = Lazy::new(|| {
    let toml_str = fs::read_to_string("../../Config.toml")
        .expect("unable to read Config.toml");
    toml::from_str(&toml_str)
        .expect("invalid TOML in Config.toml")
});

pub fn get_config() -> &'static Config {
    &CONFIG
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_root_config() {
        let cfg = get_config();
        assert!(cfg.ports.min < cfg.ports.max);
        assert!(!cfg.database.url.is_empty());
        assert!(!cfg.redis.url.is_empty());
        assert_eq!(cfg.ports.default_ttl_secs, 1800);
    }
}
