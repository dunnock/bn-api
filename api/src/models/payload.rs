use crate::errors::BigNeonError;
use actix_web::http::StatusCode;
use actix_web::HttpRequest;
use actix_web::HttpResponse;
use actix_web::{Error, Responder};
use bigneon_db::models::Payload;
use serde::Serialize;

#[derive(Debug)]
pub struct WebPayload<T>(StatusCode, Payload<T>);

impl<T> WebPayload<T>
where
    T: Serialize,
{
    pub fn new(code: StatusCode, payload: Payload<T>) -> WebPayload<T> {
        WebPayload(code, payload)
    }

    pub fn status(&self) -> StatusCode {
        self.0
    }

    pub fn payload(&self) -> &Payload<T> {
        &self.1
    }

    pub fn into_http_response(self) -> Result<HttpResponse, BigNeonError> {
        let body = serde_json::to_string(&self.1)?;
        Ok(HttpResponse::new(self.0)
            .into_builder()
            .content_type("application/json")
            .body(body))
    }
}

use futures::future::{Ready, ok};

impl<T> Responder for WebPayload<T>
where
    T: Serialize,
{
    type Future = Ready<Result<HttpResponse, Self::Error>>;
    type Error = Error;

    fn respond_to(self, _req: &HttpRequest) -> Self::Future {
        ok(self.into_http_response()?)
    }
}

pub struct WebResult<T>(StatusCode, T);

impl<T> WebResult<T> {
    pub fn new(code: StatusCode, data: T) -> WebResult<T> {
        WebResult(code, data)
    }

    pub fn status(&self) -> StatusCode {
        self.0
    }

    pub fn data(&self) -> &T {
        &self.1
    }
}

impl<T> Responder for WebResult<T>
where
    T: Serialize,
{
    type Future = Ready<Result<HttpResponse, Self::Error>>;
    type Error = Error;

    fn respond_to(self, _req: &HttpRequest) -> Self::Future {
        let body = serde_json::to_string(&self.1)?;
        ok(HttpResponse::new(self.0)
            .into_builder()
            .content_type("application/json")
            .body(body))
    }
}

impl<T> From<Vec<T>> for WebPayload<T> {
    fn from(vec: Vec<T>) -> Self {
        WebPayload(StatusCode::OK, vec.into())
    }
}
