use crate::errors::BigNeonError;
use crate::server::GetAppState;
use actix_web::{FromRequest, HttpRequest, Result};
use cache::RedisCacheConnection;
use futures::future::{Ready, ok};

#[derive(Debug, Clone)]
pub struct CacheDatabase {
    pub inner: Option<RedisCacheConnection>,
}

impl FromRequest for CacheDatabase {
    type Config = ();
    type Error = BigNeonError;
    type Future = Ready<Result<CacheDatabase, Self::Error>>;

    fn from_request(request: &HttpRequest, _config: &Self::Config) -> Self::Future {
        if let Some(connection) = request.extensions().get::<CacheDatabase>() {
            return ok(connection.clone());
        }

        let connection = request.state().database.cache_database.clone();

        request.extensions_mut().insert(connection.clone());
        ok(connection)
    }
}
