use super::AccessTokenExtractor;
use crate::auth::user::User;
use crate::errors::{ApiError, AuthError};
use crate::middleware::RequestConnection;
use actix_web::{dev::Payload, FromRequest, HttpRequest};
use db::models::User as DbUser;
use futures::future::{err, ready, Ready};

impl FromRequest for User {
    type Config = ();
    type Error = ApiError;
    type Future = Ready<Result<User, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let token = match AccessTokenExtractor::from_request(req) {
            Ok(token) => token,
            Err(e) => return err(e),
        };

        let connection = match req.connection() {
            Ok(conn) => conn,
            Err(e) => return err(e),
        };

        let user_id = match token.get_id() {
            Ok(id) => id,
            Err(_) => return err(AuthError::unauthorized("Invalid Token").into()),
        };

        let user = match DbUser::find(user_id, connection.get()) {
            Ok(user) => user,
            Err(_) => return err(AuthError::unauthorized("Invalid Token").into()),
        };

        if user.deleted_at.is_some() {
            err(AuthError::unauthorized("User account is disabled").into())
        } else {
            ready(
                User::new(user, req, token.scopes)
                    .map_err(|_| AuthError::unauthorized("User has invalid role data").into()),
            )
        }
    }
}
