use crate::cache_error::*;
use deadpool_redis::{Pool, Config, ConnectionWrapper};
use deadpool::managed::{PoolConfig, Timeouts, Object};
use std::time::Duration;
use async_trait::async_trait;
use redis::{RedisError, AsyncCommands};
use tokio::time::timeout;

type Milliseconds = usize;

#[async_trait]
// Contract for the Cache
pub trait CacheConnection {
    async fn get(&mut self, key: &str) -> Result<Option<String>, CacheError>;
    async fn delete(&mut self, key: &str) -> Result<(), CacheError>;
    async fn add(&mut self, key: &str, data: &str, ttl: Option<Milliseconds>) -> Result<(), CacheError>;
    async fn publish(&mut self, channel: &str, message: &str) -> Result<(), CacheError>;
    async fn delete_by_key_fragment(&mut self, key_fragment: &str) -> Result<(), CacheError>;
}

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
    ) -> Result<RedisCacheConnection, CacheError> {
        let pool_config = Config { 
            url: Some(database_url.to_string()),
            pool: Some ( PoolConfig { 
                timeouts: Timeouts {
                    create: Some(Duration::from_millis(connection_timeout)),
                    wait: Some(Duration::from_millis(connection_timeout)),
                    ..Timeouts::default()
                },
                ..PoolConfig::default()
             } ),
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

#[async_trait]
impl CacheConnection for RedisCacheConnection {
    async fn get(&mut self, key: &str) -> Result<Option<String>, CacheError> {
        Ok(
            timeout(
                self.read_timeout,
                self.conn().await?.get(key)
            ).await??
        )
    }

    async fn publish(&mut self, channel: &str, message: &str) -> Result<(), CacheError> {
        timeout(
            self.write_timeout,
            self.conn().await?.publish(channel, message)
        ).await??;
        Ok(())
    }

    async fn delete(&mut self, key: &str) -> Result<(), CacheError> {
        timeout(
            self.write_timeout,
            self.conn().await?.del(key.to_string())
        ).await??;
        Ok(())
    }

    async fn delete_by_key_fragment(&mut self, key_fragment: &str) -> Result<(), CacheError> {
        let mut conn = self.conn().await?;
        let matches: Vec<String> = conn.keys(key_fragment).await?;
        for key in matches {
            timeout(
                self.write_timeout,
                conn.del(key.to_string())
            ).await??;
        }
        Ok(())
    }

    async fn add(&mut self, key: &str, data: &str, ttl: Option<Milliseconds>) -> Result<(), CacheError> {
        let mut conn = self.conn().await?;
        timeout(
            self.write_timeout,
            conn.set(key, data)
        ).await??;
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
        if let Some(mut conn) = RedisCacheConnection::create_connection_pool("redis://127.0.0.1/", 10, 10, 10).ok() {
            // store key for 10 milliseconds
            conn.add("key", "value", Some(10)).await.unwrap();
            assert_eq!(Some("value".to_string()), conn.get("key").await.unwrap());

            sleep(11).await;
            // key should now be expired
            assert!(conn.get("key").await.unwrap().is_none());
        }
    }
}
