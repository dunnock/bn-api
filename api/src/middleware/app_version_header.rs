use actix_web::http::header::{HeaderName, HeaderValue};
use actix_web::middleware::DefaultHeaders;

const SEMVER_HEADER_NAME: &'static str = "X-App-Version";
const APP_VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub struct AppVersionHeader {
    header_name: HeaderName,
    app_version: HeaderValue,
}

impl AppVersionHeader {
    pub fn new() -> DefaultHeaders {
        DefaultHeaders::new()
            .header(SEMVER_HEADER_NAME, APP_VERSION)
    }
}
