use crate::auth::access_token::AccessToken;
use crate::auth::token::{TokenEndpoint, TokenRequest};
use crate::error::ShareTribeError;

pub struct AuthClient {
    pub token: TokenEndpoint,
    last_access_token: Option<AccessToken>,
}

impl AuthClient {
    pub fn new(credentials: TokenRequest) -> AuthClient {
        AuthClient {
            token: TokenEndpoint::new(credentials),
            last_access_token: None,
        }
    }

    pub fn get_token(&mut self) -> Result<String, ShareTribeError> {
        if let Some(token) = self.last_access_token.as_ref() {
            if token.is_expired() {
                self.last_access_token = Some(self.token.create()?);
            }
        } else {
            self.last_access_token = Some(self.token.create()?);
        }
        Ok(self.last_access_token.as_ref().unwrap().access_token.to_string())
    }
}
