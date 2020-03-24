use super::cache_error::*;
use bb8_redis::bb8::Pool;
use bb8_redis::RedisConnectionManager;
use std::ops::DerefMut;
use std::time::Duration;
use tokio::time::timeout;

// Implementation
#[derive(Clone)]
pub struct RedisBB8 {
    pool: Pool<RedisConnectionManager>,
    read_timeout: Duration,
    write_timeout: Duration,
}

impl RedisBB8 {
    pub async fn create_connection_pool(
        database_url: &str,
        connection_timeout: u64,
        read_timeout: u64,
        write_timeout: u64,
    ) -> RedisBB8 {
        let client = redis::Client::open(database_url).unwrap();
        let manager = RedisConnectionManager::new(client).unwrap();
        let pool = bb8_redis::bb8::Pool::builder()
            .connection_timeout(Duration::from_millis(connection_timeout))
//            .test_on_check_out(false)
            .build(manager)
            .await
            .unwrap();
        RedisBB8 {
            pool,
            read_timeout: Duration::from_millis(read_timeout),
            write_timeout: Duration::from_millis(write_timeout),
        }
    }

    pub async fn get(&mut self, key: &str) -> Result<Option<String>, CacheError> {
        let mut pooled_conn = self.pool.get().await.unwrap();
        let conn: &mut redis::aio::Connection = pooled_conn.deref_mut().as_mut().unwrap();
        let mut get = redis::cmd("GET");
        let cmd = get.arg(key).query_async(conn);
        Ok(timeout(self.read_timeout, cmd).await??)
    }
}
