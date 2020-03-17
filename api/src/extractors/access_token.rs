use crate::errors::{ApplicationError, AuthError, BigNeonError};
use crate::jwt::{decode, Validation};
use crate::server::GetAppState;
use actix_web::HttpMessage;
use bigneon_db::models::AccessToken;

pub(crate) struct AccessTokenExtractor;
impl AccessTokenExtractor {
    pub fn from_request<R>(req: &R) -> Result<AccessToken, BigNeonError>
    where
        R: HttpMessage + GetAppState,
    {
        if let Some(auth_header) = req.headers().get("Authorization") {
            let mut parts = auth_header
                .to_str()
                .map_err(|_| ApplicationError::bad_request("Invalid auth header"))?
                .split_whitespace();
            if str::ne(parts.next().unwrap_or("None"), "Bearer") {
                Err(AuthError::unauthorized("Authorization scheme not supported"))?;
            }

            match parts.next() {
                Some(access_token) => {
                    let token = decode::<AccessToken>(
                        &access_token,
                        req.state().config.token_issuer.token_secret.as_bytes(),
                        &Validation::default(),
                    )
                    .map_err(|_| AuthError::unauthorized("Invalid auth token"))?;
                    Ok(token.claims)
                }
                None => Err(AuthError::unauthorized("No access token provided").into()),
            }
        } else {
            Err(AuthError::unauthorized("Missing auth token").into())
        }
    }
}
