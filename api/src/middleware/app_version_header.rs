use actix_web::http::header::{HeaderName, HeaderValue};
use actix_web::http::HttpTryFrom;
use actix_web::middleware::{Middleware, Response};
use actix_web::{HttpRequest, HttpResponse, Result};

use crate::server::AppState;

const SEMVER_HEADER_NAME: &'static str = "X-App-Version";
const APP_VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub struct AppVersionHeader {
    header_name: HeaderName,
    app_version: HeaderValue,
}

impl AppVersionHeader {
    pub fn new() -> AppVersionHeader {
        AppVersionHeader {
            header_name: HeaderName::try_from(SEMVER_HEADER_NAME).unwrap(),
            app_version: HeaderValue::from_static(APP_VERSION),
        }
    }
}

impl Middleware<AppState> for AppVersionHeader {
    fn response(&self, _request: &HttpRequest<AppState>, mut response: HttpResponse) -> Result<Response> {
        response
            .headers_mut()
            .insert(&self.header_name, self.app_version.clone());

        Ok(Response::Done(response))
    }
}
