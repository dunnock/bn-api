use deadpool::managed::PoolError;
use redis::RedisError;
use redis_async::error::Error as RedisAsyncError;
use std::error::Error;
use std::fmt;
use std::net::AddrParseError;
use tokio::time::Elapsed;
use url::ParseError;

#[derive(Debug)]
pub struct CacheError {
    pub reason: String,
}

impl CacheError {
    pub fn new(reason: String) -> CacheError {
        CacheError { reason }
    }
}

impl Error for CacheError {
    fn description(&self) -> &str {
        &self.reason
    }
}

impl fmt::Display for CacheError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", &self.reason)
    }
}

impl From<PoolError<RedisError>> for CacheError {
    fn from(e: PoolError<RedisError>) -> Self {
        CacheError::new(e.to_string())
    }
}

impl From<RedisError> for CacheError {
    fn from(e: RedisError) -> Self {
        CacheError::new(e.to_string())
    }
}

impl From<RedisAsyncError> for CacheError {
    fn from(e: RedisAsyncError) -> Self {
        CacheError::new(format!("{:?}", e))
    }
}

impl From<Elapsed> for CacheError {
    fn from(e: Elapsed) -> Self {
        CacheError::new(e.to_string())
    }
}

impl From<AddrParseError> for CacheError {
    fn from(e: AddrParseError) -> Self {
        CacheError::new(e.to_string())
    }
}

impl From<ParseError> for CacheError {
    fn from(e: ParseError) -> Self {
        CacheError::new(e.to_string())
    }
}
