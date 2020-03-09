use crate::auth::user::User;
use crate::middleware::RequestConnection;
use actix_web::error::*;
use actix_web::{FromRequest, HttpRequest};
use bigneon_db::models::User as DbUser;
use futures::future::{Ready, ok, err};
use super::Uuid;

impl FromRequest for User {
    type Config = ();
    type Error = Error;
    type Future = Ready<Result<User, Error>>;

    fn from_request(req: &HttpRequest, _cfg: &Self::Config) -> Self::Future {
        let id = Uuid::from_request(req)?;
        let connection = req.connection()?;
        match DbUser::find(id, connection.get()) {
            // ^^ should be moved to web::block(|| ) but would require Connection to be Sync
            Ok(user) => {
                if user.deleted_at.is_some() {
                    err(ErrorUnauthorized("User account is disabled"))
                } else {
                    ok(User::new(user, req)
                        .map_err(|_| ErrorUnauthorized("User has invalid role data"))?)
                }
            }
            Err(e) => err(ErrorInternalServerError(e)),
        }
    }
}
