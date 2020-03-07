// Extractor based on Actix-Web's JSON extractor with a default error handler
// https://github.com/actix/actix-web/blob/master/src/json.rs

use actix_web::dev::JsonBody;
use actix_web::error::{Error, InternalError, JsonPayloadError};
use actix_web::{FromRequest, HttpRequest, HttpResponse};
use futures::Future;
use serde::de::DeserializeOwned;
use crate::server::AppState;
use std::ops::Deref;

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

impl<T> FromRequest<AppState> for Json<T>
where
    T: DeserializeOwned + 'static,
{
    type Config = JsonConfig;
    type Result = Box<dyn Future<Item = Self, Error = Error>>;

    #[inline]
    fn from_request(req: &HttpRequest<AppState>, cfg: &Self::Config) -> Self::Result {
        let req2 = req.clone();
        Box::new(
            JsonBody::new::<()>(req, None)
                .limit(cfg.limit)
                .map_err(move |e| json_error(e, &req2))
                .map(Json),
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

fn json_error(err: JsonPayloadError, _req: &HttpRequest<AppState>) -> Error {
    let response = match err {
        JsonPayloadError::Deserialize(ref json_error) => {
            HttpResponse::BadRequest().json(json!({ "error": json_error.to_string() }))
        }
        _ => HttpResponse::BadRequest().finish(),
    };
    InternalError::from_response(err, response).into()
}
