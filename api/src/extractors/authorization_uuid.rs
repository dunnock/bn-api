use crate::auth::claims;
use crate::jwt::{decode, Validation};
use crate::server::GetAppState;
use actix_web::error::{self, ErrorBadRequest, ErrorUnauthorized};
use actix_web::HttpMessage;
use bigneon_db::AccessToken;

pub(crate) struct AuthorizationUuid;
impl AuthorizationUuid {
    pub(crate) fn from_request<R>(req: &R) -> error::Result<uuid::Uuid>
    where
        R: HttpMessage + GetAppState,
    {
        if let Some(auth_header) = req.headers().get("Authorization") {
            let mut parts = auth_header
                .to_str()
                .map_err(|_| ErrorBadRequest("Invalid auth header"))?
                .split_whitespace();
            if str::ne(parts.next().unwrap_or("None"), "Bearer") {
                return Err(ErrorUnauthorized("Authorization scheme not supported"));
            }

            match parts.next() {
                Some(access_token) => {
                    let token = decode::<AccessToken>(
                        &access_token,
                        req.state().config.token_issuer.token_secret.as_bytes(),
                        &Validation::default(),
                    )
                    .map_err(|_| ErrorUnauthorized("Invalid auth token"))?;
                    token.claims.get_id().map_err(|e| BigNeonError::from(e))
                }
                None => Err(ErrorUnauthorized("No access token provided")),
            }
        } else {
            Err(ErrorUnauthorized("Missing auth token"))
        }
    }
}
