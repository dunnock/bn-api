use crate::cache_error::*;
use deadpool::managed::{Object, PoolConfig, Timeouts};
use deadpool_redis::{Config, ConnectionWrapper, Pool};
use redis::{AsyncCommands, RedisError};
use std::time::Duration;
use tokio::time::timeout;

type Milliseconds = usize;

// Implementation
#[derive(Clone)]
pub struct RedisCacheConnection {
    pool: Pool,
    read_timeout: Duration,
    write_timeout: Duration,
}

impl std::fmt::Debug for RedisCacheConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedisCacheConnection")
            .field("read_timeout", &self.read_timeout)
            .field("write_timeout", &self.write_timeout)
            .field("pool", &self.pool.status())
            .finish()
    }
}

impl RedisCacheConnection {
    pub fn create_connection_pool(
        database_url: &str,
        connection_timeout: u64,
        read_timeout: u64,
        write_timeout: u64,
        max_size: Option<usize>,
    ) -> Result<RedisCacheConnection, CacheError> {
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

        Ok(RedisCacheConnection {
            pool,
            read_timeout: Duration::from_millis(read_timeout),
            write_timeout: Duration::from_millis(write_timeout),
        })
    }

    pub async fn conn(&self) -> Result<Object<ConnectionWrapper, RedisError>, CacheError> {
        let connection = self.pool.get().await?;

        Ok(connection)
    }
}

//#[async_trait]
impl RedisCacheConnection {
    pub async fn get(&mut self, key: &str) -> Result<Option<String>, CacheError> {
        let mut conn = self.conn().await?;
        Ok(
            timeout(self.read_timeout, conn.get(key)).await??
        )
    }

    pub async fn publish(&mut self, channel: &str, message: &str) -> Result<(), CacheError> {
        let mut conn = self.conn().await?;
        timeout(self.write_timeout, conn.publish(channel, message)).await??;
        Ok(())
    }

    pub async fn delete(&mut self, key: &str) -> Result<(), CacheError> {
        let mut conn = self.conn().await?;
        timeout(self.write_timeout, conn.del(key.to_string())).await??;
        Ok(())
    }

    pub async fn delete_by_key_fragment(&mut self, key_fragment: &str) -> Result<(), CacheError> {
        let mut conn = self.conn().await?;
        let matches: Vec<String> = conn.keys(key_fragment).await?;
        for key in matches {
            timeout(self.write_timeout, conn.del(key.to_string())).await??;
        }
        Ok(())
    }

    pub async fn add(&mut self, key: &str, data: &str, ttl: Option<Milliseconds>) -> Result<(), CacheError> {
        let mut conn = self.conn().await?;
        timeout(self.write_timeout, conn.set(key, data)).await??;
        if let Some(ttl_val) = ttl {
            // Set a key's time to live in milliseconds.
            let _: () = conn.pexpire(key, ttl_val).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn sleep(duration: Milliseconds) {
        let duration = Duration::from_millis(duration as u64);
        tokio::time::delay_for(duration).await;
    }

    #[tokio::test]
    async fn test_caching() {
        if let Some(mut conn) =
            RedisCacheConnection::create_connection_pool("redis://127.0.0.1/", 10, 10, 10, Some(4)).ok()
        {
            // store key for 10 milliseconds
            conn.add("key", "value", Some(10)).await.unwrap();
            assert_eq!(Some("value".to_string()), conn.get("key").await.unwrap());

            sleep(11).await;
            // key should now be expired
            assert!(conn.get("key").await.unwrap().is_none());
        }
    }
}
