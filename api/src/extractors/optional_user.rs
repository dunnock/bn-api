use actix_web::error::*;
use actix_web::{FromRequest, HttpRequest};
use crate::auth::user::User;
use crate::server::AppState;
use uuid::Uuid;

#[derive(Clone)]
pub struct OptionalUser(pub Option<User>);

impl FromRequest<AppState> for OptionalUser {
    type Config = ();
    type Result = Result<OptionalUser, Error>;

    fn from_request(req: &HttpRequest<AppState>, cfg: &Self::Config) -> Self::Result {
        // If auth header exists pass authorization errors back to client
        if let Some(_auth_header) = req.headers().get("Authorization") {
            return User::from_request(req, cfg).map(|u| OptionalUser(Some(u)));
        }
        Ok(OptionalUser(None))
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
