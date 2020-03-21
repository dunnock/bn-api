use crate::config::Config;
use crate::database::CacheDatabase;
use crate::errors::*;
use crate::helpers::*;
use crate::utils::redis::*;
use cache::CacheConnection;
use actix_web::HttpResponse;
use serde::Serialize;
use serde_json::{self, Value};
use std::borrow::Borrow;

pub(crate) async fn set_cached_value<T: Serialize>(
    cache_db: &CacheDatabase,
    config: &Config,
    http_response: &HttpResponse,
    query: &T,
) -> Result<(), ApiError> {
    if cache_db.inner.is_none() {
        return Ok(());
    }
    let mut cache_connection = cache_db.inner.clone().unwrap();

    let body = application::unwrap_body_to_string(http_response).map_err(|e| ApplicationError::new(e.to_string()))?;
    let cache_period = config.redis_cache_period;
    let query_serialized = serde_json::to_string(query)?;
    if let Err(err) = cache_connection
        .add(query_serialized.borrow(), body, Some(cache_period as usize))
        .await
    {
        error!("helpers::caching#set_cached_value: {:?}", err);
    }
    Ok(())
}

pub(crate) async fn get_cached_value<T: Serialize>(
    cache_db: &CacheDatabase,
    _config: &Config,
    query: T,
) -> Option<HttpResponse> {
    if cache_db.inner.is_none() {
        return None;
    }
    let mut cache_connection = cache_db.inner.clone().unwrap();

    let query_serialized = serde_json::to_string(&query).ok()?;

    match cache_connection.get(&query_serialized).await {
        Ok(cached_value) => {
            if let Some(value) = cached_value {
                let payload: Value = serde_json::from_str(&value).ok()?;
                return Some(HttpResponse::Ok().json(&payload));
            }
        }
        Err(err) => {
            error!("helpers::caching#get_cached_value: {:?}", err);
            return None;
        }
    }
    None
}

pub(crate) async fn delete_by_key_fragment(cache_db: &CacheDatabase, key_fragment: String) -> Result<(), ApiError> {
    if cache_db.inner.is_none() {
        return Ok(());
    }
    let mut cache_connection = cache_db.inner.clone().unwrap();

    if let Err(err) = cache_connection.delete_by_key_fragment(&key_fragment).await {
        error!("helpers::caching#delete_by_key_fragment: {:?}", err);
    }
    Ok(())
}

pub(crate) async fn publish<T: Serialize>(
    cache_db: &CacheDatabase,
    redis_pubsub_channel: RedisPubSubChannel,
    message: T,
) -> Result<(), ApiError> {
    if cache_db.inner.is_none() {
        return Ok(());
    }
    let mut cache_connection = cache_db.inner.clone().unwrap();

    if let Err(err) = cache_connection
        .publish(&redis_pubsub_channel.to_string(), &serde_json::to_string(&message)?)
        .await
    {
        error!("helpers::caching#publish: {:?}", err);
    }
    Ok(())
}
