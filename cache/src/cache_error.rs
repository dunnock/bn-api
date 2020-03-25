use redis_async::error::Error as RedisAsyncError;
use std::error::Error;
use std::fmt;
use std::net::AddrParseError;
use std::num::ParseIntError;
use tokio::time::Elapsed;
use url::ParseError;

#[derive(Debug)]
pub struct CacheError {
    pub reason: String,
    pub timeout: bool,
}

impl CacheError {
    pub fn new(reason: String) -> CacheError {
        CacheError { reason, timeout: false }
    }
    pub fn new_timeout(reason: String) -> CacheError {
        CacheError { reason, timeout: true }
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

impl From<std::io::Error> for CacheError {
    fn from(e: std::io::Error) -> Self {
        CacheError::new(format!("{:?}", e))
    }
}

impl From<RedisAsyncError> for CacheError {
    fn from(e: RedisAsyncError) -> Self {
        CacheError::new(format!("{:?}", e))
    }
}

impl From<Elapsed> for CacheError {
    fn from(e: Elapsed) -> Self {
        CacheError::new_timeout(e.to_string())
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

impl From<ParseIntError> for CacheError {
    fn from(e: ParseIntError) -> Self {
        CacheError::new(e.to_string())
    }
}
