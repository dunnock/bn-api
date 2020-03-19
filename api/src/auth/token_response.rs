use crate::errors::ApiError;
use actix_web::Error;
use actix_web::HttpRequest;
use actix_web::HttpResponse;
use actix_web::Responder;
use chrono::Duration;
use db::models::{Scopes, TokenIssuer, User};
use futures::future::{err, ok, Ready};
use serde_json;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
}

impl Responder for TokenResponse {
    type Future = Ready<Result<HttpResponse, Error>>;
    type Error = Error;

    fn respond_to(self, _req: &HttpRequest) -> Self::Future {
        match serde_json::to_string(&self) {
            Ok(body) => ok(HttpResponse::Ok().content_type("application/json").body(body)),
            Err(e) => err(e.into()),
        }
    }
}

impl TokenResponse {
    pub fn new(access_token: String, refresh_token: String) -> Self {
        TokenResponse {
            access_token,
            refresh_token,
        }
    }

    pub fn create_from_user(token_issuer: &dyn TokenIssuer, expires: Duration, user: &User) -> Result<Self, ApiError> {
        Ok(TokenResponse {
            access_token: token_issuer.issue(user.id, expires)?,
            refresh_token: token_issuer.issue_with_limited_scopes(user.id, vec![Scopes::TokenRefresh], expires * 60)?,
        })
    }

    pub fn create_from_refresh_token(
        token_issuer: &dyn TokenIssuer,
        expires: Duration,
        user_id: Uuid,
    ) -> Result<Self, ApiError> {
        Ok(TokenResponse {
            access_token: token_issuer.issue(user_id, expires)?,
            refresh_token: token_issuer.issue_with_limited_scopes(user_id, vec![Scopes::TokenRefresh], expires * 60)?,
        })
    }
}
