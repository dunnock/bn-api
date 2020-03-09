use actix_web::error::*;
use actix_web::{FromRequest, HttpRequest};
use crate::auth::claims;
use crate::auth::user::User;
use bigneon_db::models::User as DbUser;
use crate::errors::*;
use crate::jwt::{decode, Validation};
use crate::middleware::RequestConnection;
use crate::config::Config;
use futures::future::{Ready, ok, err};

impl FromRequest for User {
    type Config = ();
    type Error = Error;
    type Future = Ready<Result<User, Error>>;


    fn from_request(req: &HttpRequest, _cfg: &Self::Config) -> Self::Future {
        match req.headers().get("Authorization") {
            Some(auth_header) => {
                let mut parts = auth_header
                    .to_str()
                    .map_err(BigNeonError::from)?
                    .split_whitespace();
                if str::ne(parts.next().unwrap_or("None"), "Bearer") {
                    return err(ErrorUnauthorized("Authorization scheme not supported"));
                }

                match parts.next() {
                    Some(access_token) => {
                        let token = decode::<claims::AccessToken>(
                            &access_token,
                            req.app_data::<Config>()
                                .ok_or(ErrorServiceUnavailable("Config is not available"))?
                                .token_secret.as_bytes(),
                            &Validation::default(),
                        )
                        .map_err(|e| BigNeonError::from(e))?;
                        let connection = req.connection()?;
                        let id = token.claims.get_id()?;
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
                    None => err(ErrorUnauthorized("No access token provided"))
                }
            }
            None => err(ErrorUnauthorized("Missing auth token")),
        }
    }
}
