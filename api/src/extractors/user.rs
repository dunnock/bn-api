use super::AuthorizationUuid;
use crate::auth::user::User;
use crate::middleware::RequestConnection;
use actix_web::error::*;
use actix_web::{dev::Payload, FromRequest, HttpRequest};
use bigneon_db::models::User as DbUser;
use futures::future::{err, ok, Ready};

impl FromRequest for User {
    type Config = ();
    type Error = Error;
    type Future = Ready<Result<User, Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let id = match AuthorizationUuid::from_request(req) {
            Ok(id) => id,
            Err(e) => return err(e),
        };
        let connection = match req.connection() {
            Ok(conn) => conn,
            Err(e) => return err(e),
        };
        match DbUser::find(id, connection.get()) {
            Ok(user) => {
                if user.deleted_at.is_some() {
                    err(ErrorUnauthorized("User account is disabled"))
                } else {
                    match User::new(user, req) {
                        Ok(u) => ok(u),
                        Err(_) => err(ErrorUnauthorized("User has invalid role data")),
                    }
                }
            }
            Err(e) => err(ErrorInternalServerError(e)),
        }
    }
}
