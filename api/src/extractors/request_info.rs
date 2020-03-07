use actix_web::error::*;
use actix_web::{FromRequest, HttpRequest};
use crate::models::*;
use crate::server::AppState;

impl FromRequest<AppState> for RequestInfo {
    type Config = ();
    type Result = Result<RequestInfo, Error>;

    fn from_request(req: &HttpRequest<AppState>, _cfg: &Self::Config) -> Self::Result {
        Ok(match req.headers().get("User-Agent") {
            Some(user_agent_header) => RequestInfo {
                user_agent: user_agent_header.to_str().ok().map(|ua| ua.to_string()),
            },
            None => RequestInfo { user_agent: None },
        })
    }
}
