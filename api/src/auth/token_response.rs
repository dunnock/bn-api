use actix_web::Error;
use actix_web::HttpRequest;
use actix_web::HttpResponse;
use actix_web::Responder;
use crate::auth::{claims::AccessToken, claims::RefreshToken};
use bigneon_db::models::User;
use crate::errors::BigNeonError;
use crate::jwt::{encode, Header};
use serde_json;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
}

impl Responder for TokenResponse {
    type Item = HttpResponse;
    type Error = Error;

    fn respond_to<S>(self, _req: &HttpRequest<S>) -> Result<HttpResponse, Error> {
        let body = serde_json::to_string(&self)?;
        Ok(HttpResponse::Ok().content_type("application/json").body(body))
    }
}

impl TokenResponse {
    pub fn new(access_token: &str, refresh_token: &str) -> Self {
        TokenResponse {
            access_token: String::from(access_token),
            refresh_token: String::from(refresh_token),
        }
    }

    pub fn create_from_user(
        token_secret: &str,
        token_issuer: &str,
        expiry: &u64,
        user: &User,
    ) -> Result<Self, BigNeonError> {
        let access_token_claims = AccessToken::new(&user.id, token_issuer.to_string(), expiry);
        let access_token = encode(&Header::default(), &access_token_claims, token_secret.as_bytes())?;

        let refresh_token_claims = RefreshToken::new(&user.id, token_issuer.to_string());
        let refresh_token = encode(&Header::default(), &refresh_token_claims, token_secret.as_bytes())?;

        Ok(TokenResponse {
            access_token,
            refresh_token,
        })
    }

    pub fn create_from_refresh_token(
        token_secret: &str,
        token_issuer: &str,
        expiry_time_in_minutes: &u64,
        user_id: &Uuid,
        signed_refresh_token: &str,
    ) -> Result<Self, BigNeonError> {
        let access_token_claims = AccessToken::new(&user_id, token_issuer.to_string(), expiry_time_in_minutes);
        let access_token = encode(&Header::default(), &access_token_claims, token_secret.as_bytes())?;

        Ok(TokenResponse {
            access_token,
            refresh_token: String::from(signed_refresh_token),
        })
    }
}
