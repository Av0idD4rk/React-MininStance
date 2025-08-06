use config_manager::get_config;
use redis::{aio::MultiplexedConnection, AsyncCommands, RedisError};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

const FREE_SET: &str = "ports:free";
const IN_USE_ZSET: &str = "ports:in_use";

#[derive(Debug, Error)]
pub enum PortError {
    #[error("redis error: {0}")]
    Redis(#[from] RedisError),
    #[error("no free ports available")]
    OutOfPorts,
    #[error("invalid port data")]
    InvalidPort,
}

pub struct PortManager {
    conn: MultiplexedConnection,
    pub min: u16,
    pub max: u16,
    default_ttl: u64,
}

impl PortManager {
    pub async fn new() -> Result<Self, PortError> {
        let cfg = get_config();
        let client = redis::Client::open(cfg.redis.url.as_str())?;
        let conn = client.get_multiplexed_tokio_connection().await?;

        let mut mgr = PortManager {
            conn,
            min: cfg.ports.min,
            max: cfg.ports.max,
            default_ttl: cfg.ports.default_ttl_secs,
        };
        mgr.initialize_free().await?;
        Ok(mgr)
    }

    async fn initialize_free(&mut self) -> Result<(), PortError> {
        let count: isize = self.conn.zcard(FREE_SET).await?;
        if count == 0 {
            for port in self.min..=self.max {
                let _added: usize = self.conn.zadd(FREE_SET, port, port as f64).await?;
            }
        }
        Ok(())
    }

    pub async fn reserve_port(&mut self, ttl_secs: Option<u64>) -> Result<u16, PortError> {
        let ttl = ttl_secs.unwrap_or(self.default_ttl);

        let pop: Vec<(u16, f64)> = self.conn.zpopmin(FREE_SET, 1).await?;
        if pop.is_empty() {
            return Err(PortError::OutOfPorts);
        }
        let (port, _) = pop[0];

        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let expiry = (now + ttl) as f64;

        let _added: usize = self.conn.zadd(IN_USE_ZSET, port, expiry).await?;
        Ok(port)
    }

    pub async fn release_port(&mut self, port: u16) -> Result<(), PortError> {
        let _: isize = self.conn.zrem(IN_USE_ZSET, port).await?;
        let _added: usize = self.conn.zadd(FREE_SET, port, port as f64).await?;
        Ok(())
    }

    pub async fn extend_port(&mut self, port: u16, extra_secs: u64) -> Result<(), PortError> {
        let current: Option<f64> = self.conn.zscore(IN_USE_ZSET, port).await?;
        let curr_ts = current.ok_or(PortError::InvalidPort)?;
        let new_ts = curr_ts + (extra_secs as f64);
        let _added: usize = self.conn.zadd(IN_USE_ZSET, port, new_ts).await?;
        Ok(())
    }

    pub async fn get_expired(&mut self) -> Result<Vec<u16>, PortError> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as f64;
        let expired: Vec<u16> = self
            .conn
            .zrangebyscore(IN_USE_ZSET, "-inf", now)
            .await?;
        Ok(expired)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    async fn flush_keys(conn: &mut MultiplexedConnection) {
        let _: () = conn.del(FREE_SET).await.unwrap();
        let _: () = conn.del(IN_USE_ZSET).await.unwrap();
    }

    #[tokio::test]
    async fn basic_reserve_and_release() {
        let cfg = get_config();
        let client = redis::Client::open(cfg.redis.url.as_str()).unwrap();
        let mut raw_conn = client.get_multiplexed_tokio_connection().await.unwrap();
        flush_keys(&mut raw_conn).await;

        let mut pm = PortManager::new().await.unwrap();
        let port = pm.reserve_port(Some(1)).await.unwrap();
        assert!(
            port >= pm.min && port <= pm.max,
            "Reserved port {} out of range", port
        );

        let expired_immediate = pm.get_expired().await.unwrap();
        assert!(
            !expired_immediate.contains(&port),
            "Port should not be expired immediately"
        );

        sleep(Duration::from_secs(2)).await;
        let expired_after = pm.get_expired().await.unwrap();
        assert!(
            expired_after.contains(&port),
            "Port should be expired after TTL"
        );

        pm.release_port(port).await.unwrap();
        let expired_post_release = pm.get_expired().await.unwrap();
        assert!(
            !expired_post_release.contains(&port),
            "Released port should no longer be expired"
        );
    }

    #[tokio::test]
    async fn extend_port_ttl() {
        let cfg = get_config();
        let client = redis::Client::open(cfg.redis.url.as_str()).unwrap();
        let mut raw_conn = client.get_multiplexed_tokio_connection().await.unwrap();
        flush_keys(&mut raw_conn).await;

        let mut pm = PortManager::new().await.unwrap();
        let port = pm.reserve_port(Some(5)).await.unwrap();

        let initial: f64 = pm.conn.zscore(IN_USE_ZSET, port).await.unwrap();
        pm.extend_port(port, 5).await.unwrap();
        let extended: f64 = pm.conn.zscore(IN_USE_ZSET, port).await.unwrap();

        assert!(
            extended > initial,
            "Extended expiry ({}) is not greater than initial ({})",
            extended,
            initial
        );
    }
}
