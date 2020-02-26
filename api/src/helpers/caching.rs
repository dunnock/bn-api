use actix_web::HttpResponse;
use cache::CacheConnection;
use config::Config;
use errors::*;
use helpers::*;
use serde::Serialize;
use serde_json::{self, Value};
use std::borrow::Borrow;
use utils::redis::*;

pub(crate) fn set_cached_value<T: Serialize>(
    mut cache_connection: impl CacheConnection,
    config: &Config,
    http_response: &HttpResponse,
    query: &T,
) -> Result<(), BigNeonError> {
    let body = application::unwrap_body_to_string(http_response).map_err(|e| ApplicationError::new(e.to_string()))?;
    let cache_period = config.redis_cache_period.clone();
    let query_serialized = serde_json::to_string(query)?;
    if let Err(err) = cache_connection.add(query_serialized.borrow(), body, cache_period) {
        error!("helpers::caching#set_cached_value: {:?}", err);
    }
    Ok(())
}

pub(crate) fn get_cached_value<T: Serialize>(
    mut cache_connection: impl CacheConnection,
    config: &Config,
    query: T,
) -> Option<HttpResponse> {
    let cache_period = config.redis_cache_period.clone();
    let query_serialized = serde_json::to_string(&query).ok()?;
    if cache_period.is_some() {
        match cache_connection.get(&query_serialized) {
            Ok(cached_value) => {
                let payload: Value = serde_json::from_str(&cached_value).ok()?;
                return Some(HttpResponse::Ok().json(&payload));
            }
            Err(err) => {
                error!("helpers::caching#get_cached_value: {:?}", err);
                return None;
            }
        }
    }
    None
}

pub(crate) fn publish<T: Serialize>(
    mut cache_connection: impl CacheConnection,
    redis_pubsub_channel: RedisPubSubChannel,
    message: T,
) -> Result<(), BigNeonError> {
    if let Err(err) = cache_connection.publish(&redis_pubsub_channel.to_string(), &serde_json::to_string(&message)?) {
        error!("helpers::caching#publish: {:?}", err);
    }
    Ok(())
}
