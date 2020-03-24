use crate::cache_error::CacheError;
use crate::Config;
use redis_async::{client, client::paired::PairedConnection, resp_array};
use std::time::Duration;
use tokio::time::timeout;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use futures::future::try_join_all;
use std::sync::atomic::{AtomicUsize, Ordering};

pub type Milliseconds = usize;

// Implementation
#[derive(Clone, Debug)]
pub struct RedisAsyncPool {
    connections: Vec<PairedConnection>,
    current: Arc<AtomicUsize>,
    read_timeout: Duration,
    write_timeout: Duration,
    max_size: usize,
}

impl RedisAsyncPool {
    pub async fn from_config(config: Config) -> Result<RedisAsyncPool, CacheError> {

        let url = url::Url::parse(config.database_url.as_str())?;
        let host = url.host().unwrap();
        let port = url.port_or_known_default().unwrap_or(6379);
        
        let addr = SocketAddr::from_str(format!("{}:{}", host, port).as_str())?;
        let conns: Vec<_> = (0..config.max_size).map(|_| client::paired_connect(&addr) ).collect();
        let connections = try_join_all(conns).await?;

        Ok(RedisAsyncPool {
            connections,
            current: Arc::new(AtomicUsize::new(0)),
            read_timeout: config.read_timeout,
            write_timeout: config.write_timeout,
            max_size: config.max_size,
        })
    }

    /// Retrieve object from pool or wait for one to become available.
    /// redis-async has self healing connections, hence there is no health check options here
    fn conn(&self) -> &PairedConnection {
        let idx = self.current.fetch_add(1, Ordering::Relaxed) % self.max_size;
        &self.connections[idx]
    }
}

impl RedisAsyncPool {
    pub async fn get(&self, key: &str) -> Result<Option<String>, CacheError> {
        let cmd = resp_array!["GET", key];
        Ok(timeout(self.read_timeout, self.conn().send(cmd)).await??)
    }

    pub async fn publish(&self, channel: &str, message: &str) -> Result<(), CacheError> {
        let cmd = resp_array!["PUBLISH", channel, message];
        timeout(self.write_timeout, self.conn().send::<i64>(cmd)).await??;
        Ok(())
    }

    pub async fn delete(&self, key: &str) -> Result<(), CacheError> {
        let cmd = resp_array!["DEL", key];
        timeout(self.write_timeout, self.conn().send::<i64>(cmd)).await??;
        Ok(())
    }

    pub async fn delete_by_key_fragment(&self, key_fragment: &str) -> Result<(), CacheError> {
        let mut pattern = String::from("*");
        pattern.push_str(key_fragment);
        pattern.push('*');
        let mut cursor = 0;
        loop {
            let cmd = resp_array!["SCAN", cursor.to_string(), "MATCH", pattern.as_str()];
            let (next, matches): (String, Vec<String>) = self.conn().send(cmd).await?;
            if matches.len() > 0 {
                let cmd = resp_array!["DEL"].append(matches);
                timeout(self.write_timeout, self.conn().send::<i64>(cmd)).await??;
            };
            if next == "0" {
                break;
            } else {
                cursor = next.parse()?;
            }
        }
        Ok(())
    }

    pub async fn add(&self, key: &str, data: &str, ttl: Option<Milliseconds>) -> Result<(), CacheError> {
        let cmd = resp_array!["SET", key, data];
        // sending data and ttl through same connection intentionally
        let conn = self.conn();
        timeout(self.write_timeout, conn.send::<String>(cmd)).await??;
        if let Some(ttl_val) = ttl {
            let cmd = resp_array!["PEXPIRE", key, ttl_val.to_string()];
            // Set a key's time to live in milliseconds.
            conn.send::<i64>(cmd).await.unwrap();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> Config {
        Config {
            read_timeout: Duration::from_millis(100),
            write_timeout: Duration::from_millis(100),
            max_size: 1,
            concurrency: 1,
            ..Config::default()
        }
    }

    async fn sleep(duration: Milliseconds) {
        let duration = Duration::from_millis(duration as u64);
        tokio::time::delay_for(duration).await;
    }

    #[tokio::test]
    async fn test_expire() {
        let pool = RedisAsyncPool::from_config(test_config()).await.unwrap();
        // store key for 10 milliseconds
        pool.add("key1", "value", Some(10)).await.unwrap();
        sleep(11).await;
        // key should now be expired
        assert!(pool.get("key1").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_set_get() {
        let pool = RedisAsyncPool::from_config(test_config()).await.unwrap();
        // store key for 10 milliseconds
        pool.add("key2", "value", Some(10)).await.unwrap();
        assert_eq!(Some("value".to_string()), pool.get("key2").await.unwrap());
    }


    #[tokio::test]
    async fn test_publish() {
        let pool = RedisAsyncPool::from_config(test_config()).await.unwrap();
        pool.publish("test_channel", "cache test message").await.unwrap();
    }

    #[tokio::test]
    async fn test_delete() {
        let pool = RedisAsyncPool::from_config(test_config()).await.unwrap();
        pool.add("uniquekey1", "value", Some(100)).await.unwrap();
        pool.add("uniquekey2", "value", Some(100)).await.unwrap();
        pool.delete("uniquekey1").await.unwrap();
        assert!(pool.get("uniquekey1").await.unwrap().is_none(), "deleted key should not be present");
        assert!(pool.get("uniquekey2").await.unwrap().is_some(), "not deleted key should be present");
    }

    #[tokio::test]
    async fn test_delete_by_key_fragment() {
        let pool = RedisAsyncPool::from_config(test_config()).await.unwrap();
        pool.add("uniquekey3", "value", Some(100)).await.unwrap();
        pool.add("keyset:1", "value", Some(100)).await.unwrap();
        pool.add("keyset:2", "value", Some(100)).await.unwrap();

        pool.delete_by_key_fragment("keyset").await.unwrap();
        assert!(pool.get("keyset:1").await.unwrap().is_none(), "deleted key should not be present");
        assert!(pool.get("keyset:2").await.unwrap().is_none(), "deleted key should not be present");
        assert!(pool.get("uniquekey3").await.unwrap().is_some(), "not deleted key should be present");
    }

    #[tokio::test]
    async fn test_concurrency() {
        let pool = RedisAsyncPool::from_config(Config::default()).await.unwrap();
        pool.add("uniquekey4", "value", Some(100)).await.unwrap();
        let cmds = (1u8..100).map(|_| pool.get("uniquekey4"));
        let res = try_join_all(cmds).await.unwrap();
        assert!(res.iter().all(|i| i.is_some()), "all results should be ok");
    }
}
