use cache::RedisCacheConnection;

#[derive(Debug, Clone)]
pub struct CacheDatabase {
    pub inner: Option<RedisCacheConnection>,
}
