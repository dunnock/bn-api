use crate::auth::claims;
use crate::auth::user::User;
use crate::config::Config;
use crate::db::Connection;
use crate::errors::*;
use crate::jwt::{decode, Validation};
use crate::middleware::RequestConnection;
use crate::server::AppState;
use actix_web::error::*;
use actix_web::{FromRequest, HttpRequest};
use bigneon_db::models::User as DbUser;
use std::future::Future;
use std::pin::Pin;

impl FromRequest for User {
    type Config = ();
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<User, Error>>>>;


    fn from_request(req: &HttpRequest, _cfg: &Self::Config) -> Self::Future {
        Box::pin(async {
            match req.headers().get("Authorization") {
                Some(auth_header) => {
                    let mut parts = auth_header
                        .to_str()
                        .map_err(BigNeonError::from)?
                        .split_whitespace();
                    if str::ne(parts.next().unwrap_or("None"), "Bearer") {
                        return Err(ErrorUnauthorized("Authorization scheme not supported"));
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
                            let connection = req.app_data::<Connection>().ok_or(ErrorServiceUnavailable("Database connection unavailable"))?;
                            let id = token.claims.get_id()?;
                            match web::block(|| DbUser::find(id, connection.get())).await {
                                Ok(user) => {
                                    if user.deleted_at.is_some() {
                                        Err(ErrorUnauthorized("User account is disabled"))
                                    } else {
                                        Ok(User::new(user, req)
                                            .map_err(|_| ErrorUnauthorized("User has invalid role data"))?)
                                    }
                                }
                                Err(e) => Err(ErrorInternalServerError(e)),
                            }
                        }
                        None => Err(ErrorUnauthorized("No access token provided"))
                    }
                }
                None => Err(ErrorUnauthorized("Missing auth token")),
            }
        })
    }
}
