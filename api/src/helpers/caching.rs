use crate::config::Config;
use crate::errors::*;
use crate::helpers::*;
use crate::utils::redis::*;
use actix_web::HttpResponse;
use cache::CacheConnection;
use serde::Serialize;
use serde_json::{self, Value};
use std::borrow::Borrow;

pub(crate) fn set_cached_value<T: Serialize>(
    mut cache_connection: impl CacheConnection,
    config: &Config,
    http_response: &HttpResponse,
    query: &T,
) -> Result<(), ApiError> {
    let body = application::unwrap_body_to_string(http_response).map_err(|e| ApplicationError::new(e.to_string()))?;
    let cache_period = config.redis_cache_period;
    let query_serialized = serde_json::to_string(query)?;
    if let Err(err) = cache_connection.add(query_serialized.borrow(), body, Some(cache_period as usize)) {
        error!("helpers::caching#set_cached_value: {:?}", err);
    }
    Ok(())
}

pub(crate) fn get_cached_value<T: Serialize>(
    mut cache_connection: impl CacheConnection,
    config: &Config,
    query: T,
) -> Option<HttpResponse> {
    let query_serialized = serde_json::to_string(&query).ok()?;
    if config.redis_connection_string.is_some() {
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

pub(crate) fn delete_by_key_fragment(
    mut cache_connection: impl CacheConnection,
    key_fragment: String,
) -> Result<(), ApiError> {
    if let Err(err) = cache_connection.delete_by_key_fragment(&key_fragment) {
        error!("helpers::caching#delete_by_key_fragment: {:?}", err);
    }
    Ok(())
}

pub(crate) fn publish<T: Serialize>(
    mut cache_connection: impl CacheConnection,
    redis_pubsub_channel: RedisPubSubChannel,
    message: T,
) -> Result<(), ApiError> {
    if let Err(err) = cache_connection.publish(&redis_pubsub_channel.to_string(), &serde_json::to_string(&message)?) {
        error!("helpers::caching#publish: {:?}", err);
    }
    Ok(())
}
