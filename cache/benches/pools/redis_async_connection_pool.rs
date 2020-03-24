use super::cache_error::*;
use futures::future::try_join_all;
use redis_async::{client, client::paired::PairedConnection, resp_array};
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

// Implementation
#[derive(Clone, Debug)]
pub struct RedisAsyncConnectionPool {
    objects: Vec<PairedConnection>,
    current: Arc<AtomicUsize>,
    read_timeout: Duration,
    write_timeout: Duration,
    max_size: usize,
}

impl RedisAsyncConnectionPool {
    pub async fn create_connection_pool(
        database_url: &str,
        read_timeout: u64,
        write_timeout: u64,
        max_size: Option<usize>,
    ) -> Result<RedisAsyncConnectionPool, CacheError> {
        let max_size = max_size.unwrap_or(num_cpus::get()); //change to num_cpus

        let url = url::Url::parse(database_url)?;
        let host = url.host().unwrap();
        let port = url.port_or_known_default().unwrap_or(6379);

        let addr = SocketAddr::from_str(format!("{}:{}", host, port).as_str())?;
        let conns: Vec<_> = (0..max_size).map(|_| client::paired_connect(&addr)).collect();
        let objects = try_join_all(conns).await?;

        Ok(RedisAsyncConnectionPool {
            objects,
            current: Arc::new(AtomicUsize::new(0)),
            read_timeout: Duration::from_millis(read_timeout),
            write_timeout: Duration::from_millis(write_timeout),
            max_size,
        })
    }

    /// Retrieve object from pool or wait for one to become available.
    fn conn(&self) -> &PairedConnection {
        let idx = self.current.fetch_add(1, Ordering::Relaxed) % self.max_size;
        &self.objects[idx]
    }

    pub async fn get(&self, key: &str) -> Result<Option<String>, CacheError> {
        let cmd = resp_array!["GET", key];
        Ok(timeout(self.read_timeout, self.conn().send(cmd)).await??)
    }
}
