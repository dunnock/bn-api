use crate::r2d2_redis::r2d2::Error as R2D2Error;
use crate::redis::RedisError;
use std::error::Error;
use std::fmt;

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

impl From<R2D2Error> for CacheError {
    fn from(e: R2D2Error) -> Self {
        CacheError::new(e.to_string())
    }
}

impl From<RedisError> for CacheError {
    fn from(e: RedisError) -> Self {
        CacheError::new(e.to_string())
    }
}
