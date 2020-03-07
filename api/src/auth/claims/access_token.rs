use crate::errors::BigNeonError;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct AccessToken {
    pub sub: String,
    pub iss: String,
    pub exp: u64,
}

impl AccessToken {
    pub fn new(user_id: &Uuid, issuer: String, expiry_in_minutes: &u64) -> Self {
        let mut timer = SystemTime::now();
        timer += Duration::from_secs(expiry_in_minutes * 60);
        let exp = timer.duration_since(UNIX_EPOCH).unwrap().as_secs();

        AccessToken {
            iss: issuer,
            sub: user_id.hyphenated().to_string(),
            exp,
        }
    }

    pub fn get_id(&self) -> Result<Uuid, BigNeonError> {
        Ok(Uuid::parse_str(&self.sub)?)
    }
}
