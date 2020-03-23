use crate::cache_error::*;
use redis_async::{client, client::paired::PairedConnection, resp_array};
use std::time::Duration;
use tokio::time::timeout;
use std::net::SocketAddr;
use std::str::FromStr;

type Milliseconds = usize;

// Implementation
#[derive(Clone, Debug)]
pub struct RedisAsyncCacheConnection {
    conn: PairedConnection,
    read_timeout: Duration,
    write_timeout: Duration,
}

impl RedisAsyncCacheConnection {
    pub async fn create_connection(
        database_url: &str,
        connection_timeout: u64,
        read_timeout: u64,
        write_timeout: u64,
    ) -> Result<RedisAsyncCacheConnection, CacheError> {

        let url = url::Url::parse(database_url)?;
        let host = url.host().unwrap();
        let port = url.port_or_known_default().unwrap_or(6379);
        
        let addr = SocketAddr::from_str(format!("{}:{}", host, port).as_str())?;
        let conn = client::paired_connect(&addr).await?;

        Ok(RedisAsyncCacheConnection {
            conn,
            read_timeout: Duration::from_millis(read_timeout),
            write_timeout: Duration::from_millis(write_timeout),
        })
    }
}

//#[async_trait]
impl RedisAsyncCacheConnection {
    pub async fn get(&self, key: &str) -> Result<Option<String>, CacheError> {
        let cmd = resp_array!["GET", key];
        Ok(self.conn.send(cmd).await?)
    }

    pub async fn publish(&self, channel: &str, message: &str) -> Result<(), CacheError> {
        let cmd = resp_array!["PUBLISH", channel, message];
        timeout(self.write_timeout, self.conn.send(cmd)).await??;
        Ok(())
    }

    pub async fn delete(&self, key: &str) -> Result<(), CacheError> {
        let cmd = resp_array!["DEL", key];
        timeout(self.write_timeout, self.conn.send(cmd)).await??;
        Ok(())
    }

    pub async fn delete_by_key_fragment(&self, key_fragment: &str) -> Result<(), CacheError> {
        let cmd = resp_array!["KEYS", key_fragment];
        let matches: Vec<String> = self.conn.send(cmd).await?;
        for key in matches {
            let cmd = resp_array!["DEL", key];
            timeout(self.write_timeout, self.conn.send(cmd)).await??;
        }
        Ok(())
    }

    pub async fn add(&self, key: &str, data: &str, ttl: Option<Milliseconds>) -> Result<(), CacheError> {
        let cmd = resp_array!["SET", key, data];
        timeout(self.write_timeout, self.conn.send(cmd)).await??;
        if let Some(ttl_val) = ttl {
            let cmd = resp_array!["PEXPIRE", key, ttl_val];
            // Set a key's time to live in milliseconds.
            let _: () = self.conn.send(cmd).await?;
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
        RedisAsyncCacheConnection::create_connection("redis://127.0.0.1/", 10, 10, 10).await.ok()
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
