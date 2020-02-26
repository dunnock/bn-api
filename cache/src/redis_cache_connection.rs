use crate::cache_error::*;
use crate::r2d2_redis::r2d2::{Pool, PooledConnection};
use crate::r2d2_redis::RedisConnectionManager;
use crate::redis::Commands;
use std::sync::Arc;
use std::time::Duration;

type Milliseconds = usize;

// Contract for the Cache
pub trait CacheConnection {
    fn get(&mut self, key: &str) -> Result<String, CacheError>;
    fn delete(&mut self, key: &str) -> Result<(), CacheError>;
    fn add(&mut self, key: &str, data: &str, ttl: Option<Milliseconds>) -> Result<(), CacheError>;
    fn publish(&mut self, channel: &str, message: &str) -> Result<(), CacheError>;
}

// Implementation
#[derive(Debug, Clone)]
pub struct RedisCacheConnection {
    pool: Arc<Pool<RedisConnectionManager>>,
    read_timeout: u64,
    write_timeout: u64,
}

impl RedisCacheConnection {
    pub fn create_connection_pool(
        database_url: &str,
        connection_timeout: u64,
        read_timeout: u64,
        write_timeout: u64,
    ) -> Result<RedisCacheConnection, CacheError> {
        let manager = RedisConnectionManager::new(database_url)?;
        let pool = r2d2_redis::r2d2::Pool::builder()
            .connection_timeout(Duration::from_millis(connection_timeout))
            .build(manager)?;
        Ok(RedisCacheConnection {
            pool: Arc::from(pool),
            read_timeout,
            write_timeout,
        })
    }

    pub fn conn(&self) -> Result<PooledConnection<RedisConnectionManager>, CacheError> {
        let connection = self.pool.get()?;
        connection.set_read_timeout(Some(Duration::from_millis(self.read_timeout)))?;
        connection.set_write_timeout(Some(Duration::from_millis(self.write_timeout)))?;

        Ok(connection)
    }
}

impl CacheConnection for RedisCacheConnection {
    fn get(&mut self, key: &str) -> Result<String, CacheError> {
        Ok(self.conn()?.get(key)?)
    }

    fn publish(&mut self, channel: &str, message: &str) -> Result<(), CacheError> {
        self.conn()?.publish(channel, message)?;
        Ok(())
    }

    fn delete(&mut self, key: &str) -> Result<(), CacheError> {
        self.conn()?.del(key.to_string())?;
        Ok(())
    }

    fn add(&mut self, key: &str, data: &str, ttl: Option<Milliseconds>) -> Result<(), CacheError> {
        let mut conn = self.conn()?;
        conn.set(key, data)?;
        if let Some(ttl_val) = ttl {
            // Set a key's time to live in milliseconds.
            let _: () = conn.pexpire(key, ttl_val)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    fn sleep(duration: Milliseconds) {
        let duration = Duration::from_millis(duration as u64);
        thread::sleep(duration);
    }

    #[test]
    fn test_caching() {
        if let Some(mut conn) = RedisCacheConnection::create_connection_pool("redis://127.0.0.1/", 10, 10, 10).ok() {
            // store key for 10 milliseconds
            conn.add("key", "value", Some(10)).unwrap();
            assert_eq!("value", conn.get("key").unwrap());

            sleep(11);
            // key should now be expired
            assert!(conn.get("key").is_err());
        }
    }
}
