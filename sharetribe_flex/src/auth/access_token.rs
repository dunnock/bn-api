use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct AccessToken {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub scope: String,
    pub refresh_token: Option<String>,
    #[serde(default = "Utc::now")]
    created_at: DateTime<Utc>,
}

impl AccessToken {
    pub fn is_expired(&self) -> bool {
        self.created_at + Duration::milliseconds(self.expires_in) < Utc::now()
    }
}
