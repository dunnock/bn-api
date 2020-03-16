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
            Ok(conn) => conn.get(),
            Err(e) => return err(e),
        };

        let user = DbUser::find(user_id, &connection).map_err(|_| ErrorUnauthorized("Invalid Token"))?;

        if user.deleted_at.is_some() {
            Err(ErrorUnauthorized("User account is disabled"))
        } else {
            Ok(User::new(user, req, token.claims.scopes)
                .map_err(|_| ErrorUnauthorized("User has invalid role data"))?)
        }
    }
}
