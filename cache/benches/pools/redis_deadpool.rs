use super::cache_error::*;
use deadpool::managed::{Object, PoolConfig, Timeouts};
use deadpool_redis::{Config, ConnectionWrapper, Pool};
use redis::{AsyncCommands, RedisError};
use std::time::Duration;
use tokio::time::timeout;

// Implementation
#[derive(Clone)]
pub struct RedisDeadpool {
    pool: Pool,
    read_timeout: Duration,
    write_timeout: Duration,
}

impl std::fmt::Debug for RedisDeadpool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedisCacheConnection")
            .field("read_timeout", &self.read_timeout)
            .field("write_timeout", &self.write_timeout)
            .field("pool", &self.pool.status())
            .finish()
    }
}

impl RedisDeadpool {
    pub fn create_connection_pool(
        database_url: &str,
        connection_timeout: u64,
        read_timeout: u64,
        write_timeout: u64,
        max_size: Option<usize>,
    ) -> Result<RedisDeadpool, CacheError> {
        let max_size = max_size.unwrap_or(PoolConfig::default().max_size);
        let pool_config = Config {
            url: Some(database_url.to_string()),
            pool: Some(PoolConfig {
                timeouts: Timeouts {
                    create: Some(Duration::from_millis(connection_timeout)),
                    wait: Some(Duration::from_millis(connection_timeout)),
                    ..Timeouts::default()
                },
                max_size,
            }),
        };

        let pool = pool_config.create_pool()?;

        Ok(RedisDeadpool {
            pool,
            read_timeout: Duration::from_millis(read_timeout),
            write_timeout: Duration::from_millis(write_timeout),
        })
    }

    pub async fn conn(&self) -> Result<Object<ConnectionWrapper, RedisError>, CacheError> {
        let connection = self.pool.get().await?;

        Ok(connection)
    }

    pub async fn get(&mut self, key: &str) -> Result<Option<String>, CacheError> {
        let mut conn = self.conn().await.unwrap();
        Ok(timeout(self.read_timeout, conn.get(key)).await??)
    }
}
