use super::cache_error::*;
use redis_async::{client, client::paired::PairedConnection, resp_array};
use std::time::Duration;
use tokio::time::timeout;
use std::net::SocketAddr;
use std::str::FromStr;

// Implementation
#[derive(Clone, Debug)]
pub struct RedisAsyncConnection {
    conn: PairedConnection,
    read_timeout: Duration,
    write_timeout: Duration,
}

impl RedisAsyncConnection {
    pub async fn create_connection(
        database_url: &str,
        read_timeout: u64,
        write_timeout: u64,
    ) -> Result<RedisAsyncConnection, CacheError> {

        let url = url::Url::parse(database_url)?;
        let host = url.host().unwrap();
        let port = url.port_or_known_default().unwrap_or(6379);
        
        let addr = SocketAddr::from_str(format!("{}:{}", host, port).as_str())?;
        let conn = client::paired_connect(&addr).await?;

        Ok(RedisAsyncConnection {
            conn,
            read_timeout: Duration::from_millis(read_timeout),
            write_timeout: Duration::from_millis(write_timeout),
        })
    }

    pub async fn get(&self, key: &str) -> Result<Option<String>, CacheError> {
        let cmd = resp_array!["GET", key];
        Ok(timeout(self.read_timeout, self.conn.send(cmd)).await??)
    }
}
