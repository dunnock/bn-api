use itertools::Itertools;
use models::Scopes;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use uuid::{ParseError, Uuid};

#[derive(Debug, Serialize, Deserialize)]
pub struct AccessToken {
    pub sub: String,
    pub iss: String,
    pub exp: u64,
    #[serde(default)]
    pub scopes: Option<Vec<String>>,
    pub issued: u64,
}

impl AccessToken {
    pub fn new(user_id: Uuid, issuer: String, expiry_in_minutes: i64) -> Self {
        let mut timer = SystemTime::now();
        timer += Duration::from_secs(expiry_in_minutes as u64 * 60);
        let exp = timer.duration_since(UNIX_EPOCH).unwrap().as_secs();
        let issued = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

        AccessToken {
            iss: issuer,
            sub: user_id.hyphenated().to_string(),
            exp,
            scopes: None,
            issued,
        }
    }

    pub fn new_limited_scope(user_id: Uuid, issuer: String, expiry_in_minutes: i64, scopes: Vec<Scopes>) -> Self {
        let mut timer = SystemTime::now();
        timer += Duration::from_secs(expiry_in_minutes as u64 * 60);
        let exp = timer.duration_since(UNIX_EPOCH).unwrap().as_secs();
        let issued = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        AccessToken {
            iss: issuer,
            sub: user_id.hyphenated().to_string(),
            exp,
            scopes: Some(scopes.into_iter().map(|s| s.to_string()).collect_vec()),
            issued,
        }
    }

    pub fn get_id(&self) -> Result<Uuid, ParseError> {
        Ok(Uuid::parse_str(&self.sub)?)
    }
}
