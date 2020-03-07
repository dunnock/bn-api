use crate::models::*;
use crate::server::AppState;
use actix_web::error::*;
use actix_web::{FromRequest, HttpRequest};
use futures::future::{Ready, ok};

impl FromRequest for RequestInfo {
    type Config = ();
    type Error = Error;
    type Future = Ready<Result<RequestInfo, Error>>;

    fn from_request(req: &HttpRequest, _cfg: &Self::Config) -> Self::Future {
        ok(match req.headers().get("User-Agent") {
            Some(user_agent_header) => RequestInfo {
                user_agent: user_agent_header.to_str().ok().map(|ua| ua.to_string()),
            },
            None => RequestInfo { user_agent: None },
        })
    }
}
