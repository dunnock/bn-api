use crate::errors::BigNeonError;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct RefreshToken {
    pub sub: String,
    pub iss: String,
    pub issued: u64,
}

impl RefreshToken {
    pub fn new(user_id: &Uuid, issuer: String) -> Self {
        let issued = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

        RefreshToken {
            iss: issuer,
            sub: user_id.hyphenated().to_string(),
            issued,
        }
    }

    pub fn get_id(&self) -> Result<Uuid, BigNeonError> {
        Ok(Uuid::parse_str(&self.sub)?)
    }
}
