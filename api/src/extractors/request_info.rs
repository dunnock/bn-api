use crate::models::*;
use actix_web::error::*;
use actix_web::{dev::Payload, FromRequest, HttpRequest};
use futures::future::{ok, Ready};

impl FromRequest for RequestInfo {
    type Config = ();
    type Error = Error;
    type Future = Ready<Result<RequestInfo, Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        ok(match req.headers().get("User-Agent") {
            Some(user_agent_header) => RequestInfo {
                user_agent: user_agent_header.to_str().ok().map(|ua| ua.to_string()),
            },
            None => RequestInfo { user_agent: None },
        })
    }
}
