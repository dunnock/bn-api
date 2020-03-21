use deadpool::managed::PoolError;
use redis::RedisError;
use std::error::Error;
use std::fmt;
use tokio::time::Elapsed;

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

impl From<Elapsed> for CacheError {
    fn from(e: Elapsed) -> Self {
        CacheError::new(e.to_string())
    }
}
