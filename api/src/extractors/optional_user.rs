use crate::auth::user::User;
use crate::server::AppState;
use actix_web::error::*;
use actix_web::{FromRequest, HttpRequest};
use uuid::Uuid;
use futures::future::ok;
use std::future::Future;
use std::pin::Pin;

#[derive(Clone)]
pub struct OptionalUser(pub Option<User>);

impl FromRequest for OptionalUser {
    type Config = ();
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<OptionalUser, Error>>>>;

    fn from_request(req: &HttpRequest, cfg: &Self::Config) -> Self::Future {
        // If auth header exists pass authorization errors back to client
        if let Some(_auth_header) = req.headers().get("Authorization") {
            return User::from_request(req, cfg).map(|u| OptionalUser(Some(u)));
        }
        Box::pin(ok(OptionalUser(None)))
    }
}

impl OptionalUser {
    pub fn into_inner(self) -> Option<User> {
        self.0
    }
    pub fn id(&self) -> Option<Uuid> {
        self.0.as_ref().map(|u| u.id())
    }
}
