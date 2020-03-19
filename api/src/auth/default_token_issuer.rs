use db::models::{AccessToken, Scopes, TokenIssuer};

use chrono::Duration;
use jwt::{decode, encode, errors, Header, TokenData, Validation};
use uuid::Uuid;

#[derive(Clone)]
pub struct DefaultTokenIssuer {
    pub token_secret: String,
    pub token_issuer: String,
}

impl DefaultTokenIssuer {
    pub fn new(token_secret: String, token_issuer: String) -> Self {
        DefaultTokenIssuer {
            token_secret,
            token_issuer,
        }
    }
}

impl TokenIssuer for DefaultTokenIssuer {
    fn encode(&self, claims: &AccessToken) -> Result<String, errors::Error> {
        encode(&Header::default(), claims, self.token_secret.as_bytes())
    }
    fn decode(&self, access_token: &str) -> Result<TokenData<AccessToken>, jwt::errors::Error> {
        decode::<AccessToken>(access_token, self.token_secret.as_bytes(), &Validation::default())
    }

    fn issue(&self, user_id: Uuid, expires: Duration) -> Result<String, errors::Error> {
        let access_token_claims = AccessToken::new(user_id, self.token_issuer.to_string(), expires.num_minutes());

        encode(&Header::default(), &access_token_claims, self.token_secret.as_bytes())
    }

    fn issue_with_limited_scopes(
        &self,
        user_id: Uuid,
        scopes: Vec<Scopes>,
        expires: Duration,
    ) -> Result<String, errors::Error> {
        let access_token_claims =
            AccessToken::new_limited_scope(user_id, self.token_issuer.to_string(), expires.num_minutes(), scopes);
        encode(&Header::default(), &access_token_claims, self.token_secret.as_bytes())
    }
}
