use crate::cache_error::CacheError;
use crate::Config;
use futures::future::try_join_all;
use redis_async::{client, client::paired::PairedConnection, resp_array};
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::time::timeout;

pub type Milliseconds = usize;

// Implementation
#[derive(Clone, Debug)]
pub struct RedisAsyncPool {
    connections: Vec<PairedConnection>,
    current: Arc<AtomicUsize>,
    concurrency: Arc<Semaphore>,
    read_timeout: Duration,
    write_timeout: Duration,
    max_size: usize,
}

impl RedisAsyncPool {
    pub async fn from_config(config: &Config) -> Result<RedisAsyncPool, CacheError> {
        let url = url::Url::parse(config.database_url.as_str())?;
        let host = url.host().unwrap();
        let port = url.port_or_known_default().unwrap_or(6379);

        let addr = SocketAddr::from_str(format!("{}:{}", host, port).as_str())?;
        let conns: Vec<_> = (0..config.max_size).map(|_| client::paired_connect(&addr)).collect();
        let connections = try_join_all(conns).await?;
        let concurrency = Arc::new(Semaphore::new(config.max_size * config.concurrency));

        Ok(RedisAsyncPool {
            connections,
            current: Arc::new(AtomicUsize::new(0)),
            concurrency,
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
        let guard = self.concurrency.acquire().await;
        let cmd = resp_array!["GET", key];
        let res = Ok(timeout(self.read_timeout, self.conn().send(cmd)).await??);
        drop(guard);
        res
    }

    // Will return microsedonds taken to perform request
    // This is for testing and profiling purposes only
    #[allow(dead_code)]
    pub(crate) async fn get_bench(&self, key: &str) -> Result<Option<u128>, CacheError> {
        let guard = self.concurrency.acquire().await;
        let cmd = resp_array!["GET", key];
        let time = std::time::Instant::now();
        let res: Option<String> = self.conn().send(cmd).await?;
        drop(guard);
        Ok(res.map(|_| time.elapsed().as_micros()))
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
        pool.add("key2", "value", Some(1000)).await.unwrap();
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
        pool.add("uniquekey1", "value", Some(1000)).await.unwrap();
        pool.add("uniquekey2", "value", Some(1000)).await.unwrap();
        pool.delete("uniquekey1").await.unwrap();
        assert!(
            pool.get("uniquekey1").await.unwrap().is_none(),
            "deleted key should not be present"
        );
        assert!(
            pool.get("uniquekey2").await.unwrap().is_some(),
            "not deleted key should be present"
        );
    }

    #[tokio::test]
    async fn test_delete_by_key_fragment() {
        let pool = RedisAsyncPool::from_config(test_config()).await.unwrap();
        pool.add("uniquekey3", "value", Some(1000)).await.unwrap();
        pool.add("keyset:1", "value", Some(1000)).await.unwrap();
        pool.add("keyset:2", "value", Some(1000)).await.unwrap();

        pool.delete_by_key_fragment("keyset").await.unwrap();
        assert!(
            pool.get("keyset:1").await.unwrap().is_none(),
            "deleted key should not be present"
        );
        assert!(
            pool.get("keyset:2").await.unwrap().is_none(),
            "deleted key should not be present"
        );
        assert!(
            pool.get("uniquekey3").await.unwrap().is_some(),
            "not deleted key should be present"
        );
    }

    #[tokio::test]
    async fn concurrency_tests() {
        // delay this test's start to not interfere with other tests
        sleep(1000).await;

        let data = String::from_utf8(vec![90u8; 5000]).unwrap();
        const REQUESTS: u128 = 1000;

        // set low concurrency to avoid timeout failure
        let pool = RedisAsyncPool::from_config(Config {
            concurrency: 1,
            max_size: 1,
            ..Config::default()
        })
        .await
        .unwrap();
        pool.add("uniquekey4", data.as_str(), Some(10_000)).await.unwrap();
        let cmds = (1..REQUESTS).map(|_| pool.get_bench("uniquekey4"));
        let res = try_join_all(cmds).await.unwrap();
        responses_report("Concurrency = 1", res.clone());
        let avg_response_single = res.iter().fold(0, |s, i| s + i.unwrap_or(10_000)) / REQUESTS;

        // with high concurrency we guaranteed to timeout
        let pool = RedisAsyncPool::from_config(Config {
            concurrency: 1_000,
            max_size: 1,
            ..Config::default()
        })
        .await
        .unwrap();
        pool.add("uniquekey5", data.as_str(), Some(10_000)).await.unwrap();
        let cmds = (1..REQUESTS).map(|_| pool.get_bench("uniquekey5"));
        let res = try_join_all(cmds).await.unwrap();
        responses_report("Concurrency = 1_000", res.clone());
        let avg_response_concur = res.iter().fold(0, |s, i| s + i.unwrap_or(10_000)) / REQUESTS;

        assert!(
            avg_response_concur / avg_response_single > 10,
            "response time for single vs concurrent less than factor 10"
        );
    }

    fn responses_report(name: &str, data: Vec<Option<u128>>) {
        let mut histo = hdrhistogram::Histogram::<u64>::new_with_bounds(1, 1000 * 1000 * 10, 3).unwrap();
        for item in data.iter() {
            histo += item.unwrap() as u64;
        }
        println!("{} / response times percentiles:", name);
        for val in histo.iter_quantiles(1).take(8) {
            println!(
                "{:.2}% < {:.3}ms",
                val.percentile(),
                val.value_iterated_to() as f64 / 1000.0
            );
        }
    }
}
