use crate::auth::user::User;
use actix_web::error::*;
use actix_web::{dev, FromRequest, HttpRequest};
use futures::future::{err, ok, Ready};
use uuid::Uuid;

#[derive(Clone)]
pub struct OptionalUser(pub Option<User>);

impl FromRequest for OptionalUser {
    type Config = ();
    type Error = Error;
    type Future = Ready<Result<OptionalUser, Error>>;

    fn from_request(req: &HttpRequest, payload: &mut dev::Payload) -> Self::Future {
        // If auth header exists pass authorization errors back to client
        if let Some(_auth_header) = req.headers().get("Authorization") {
            let user = match User::from_request(req, payload).into_inner() {
                Ok(user) => user,
                Err(e) => return err(e),
            };
            return ok(OptionalUser(Some(user)));
        }
        ok(OptionalUser(None))
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
