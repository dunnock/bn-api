use chrono::Duration;
use jsonwebtoken::errors::Error;
use jsonwebtoken::TokenData;
use models::Scopes;
use prelude::AccessToken;
use uuid::Uuid;

pub trait TokenIssuer {
    fn encode(&self, claims: &AccessToken) -> Result<String, Error>;
    fn decode(&self, access_token: &str) -> Result<TokenData<AccessToken>, Error>;
    fn issue(&self, user_id: Uuid, expires: Duration) -> Result<String, Error>;
    fn issue_with_limited_scopes(&self, user_id: Uuid, scopes: Vec<Scopes>, expires: Duration)
        -> Result<String, Error>;
}
