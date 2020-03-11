// Extractor based on Actix-Web's JSON extractor with a default error handler
// https://github.com/actix/actix-web/blob/master/src/json.rs

use actix_web::dev::JsonBody;
use actix_web::error::{Error, InternalError, JsonPayloadError};
use actix_web::{FromRequest, HttpRequest, HttpResponse, dev::Payload};
use serde::de::DeserializeOwned;
use std::ops::Deref;
use std::pin::Pin;
use std::future::Future;
use futures::future::TryFutureExt;

const LIMIT_DEFAULT: usize = 262_144; // 256Kb

pub struct Json<T>(pub T);

impl<T> Json<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Deref for Json<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> FromRequest for Json<T>
where
    T: DeserializeOwned + 'static,
{
    type Config = JsonConfig;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Error>>>>;

    #[inline]
    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let mut json_body = JsonBody::new(req, payload, None);
        if let Some(cfg) = req.app_data::<JsonConfig>() {
            json_body = json_body.limit(cfg.limit);
        }
        Box::pin(
            json_body
                .map_err(json_error)
                .map_ok(Json),
        )
    }
}

pub struct JsonConfig {
    limit: usize,
}

impl JsonConfig {
    pub fn limit(&mut self, limit: usize) -> &mut Self {
        self.limit = limit;
        self
    }
}

impl Default for JsonConfig {
    fn default() -> Self {
        JsonConfig { limit: LIMIT_DEFAULT }
    }
}

fn json_error(err: JsonPayloadError) -> Error {
    let response = match err {
        JsonPayloadError::Deserialize(ref json_error) => {
            HttpResponse::BadRequest().json(json!({ "error": json_error.to_string() }))
        }
        _ => HttpResponse::BadRequest().finish(),
    };
    InternalError::from_response(err, response).into()
}
