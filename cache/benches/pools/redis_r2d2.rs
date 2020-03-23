use super::cache_error::*;
use r2d2_redis::r2d2::{Pool, PooledConnection};
use r2d2_redis::RedisConnectionManager;
use r2d2_redis::r2d2::Error as R2D2Error;
use redis::Commands;
use std::sync::Arc;
use std::time::Duration;

type Milliseconds = usize;


// Implementation
#[derive(Debug, Clone)]
pub struct RedisR2D2 {
    pool: Arc<Pool<RedisConnectionManager>>,
    read_timeout: u64,
    write_timeout: u64,
}

impl RedisR2D2 {
    pub fn create_connection_pool(
        database_url: &str,
        connection_timeout: u64,
        read_timeout: u64,
        write_timeout: u64,
    ) -> RedisR2D2 {
        let manager = RedisConnectionManager::new(database_url).unwrap();
        let pool = r2d2_redis::r2d2::Pool::builder()
            .connection_timeout(Duration::from_millis(connection_timeout))
            .build(manager)
            .unwrap();
        RedisR2D2 {
            pool: Arc::from(pool),
            read_timeout,
            write_timeout,
        }
    }

    pub fn conn(&self) -> Result<PooledConnection<RedisConnectionManager>, R2D2Error> {
        let connection = self.pool.get()?;
        connection.set_read_timeout(Some(Duration::from_millis(self.read_timeout))).unwrap();
        connection.set_write_timeout(Some(Duration::from_millis(self.write_timeout))).unwrap();

        Ok(connection)
    }

    pub fn get(&mut self, key: &str) -> Result<Option<String>, CacheError> {
        let mut conn = self.conn().unwrap();
        Ok(conn.get(key)?)
    }

    pub fn add(&mut self, key: &str, data: &str, ttl: Option<Milliseconds>) {
        let mut conn = self.conn().unwrap();
        let () = conn.set(key, data).unwrap();
        if let Some(ttl_val) = ttl {
            // Set a key's time to live in milliseconds.
            let _: () = conn.pexpire(key, ttl_val).unwrap();
        };
    }
}