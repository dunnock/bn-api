use crate::auth::auth_client::AuthClient;
use crate::error::*;
use crate::result::ShareTribeResult;
use crate::util::HttpResponseExt;
use crate::{Response, BASE_URI};
use chrono::{DateTime, Utc};
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use snafu::ResultExt;
use std::sync::{Arc, RwLock};

pub struct CurrentUserEndpoint {
    auth: Arc<RwLock<AuthClient>>,
}

impl CurrentUserEndpoint {
    pub fn new(auth: Arc<RwLock<AuthClient>>) -> CurrentUserEndpoint {
        Self { auth }
    }
    pub fn create(&mut self, user: CreateCurrentUserRequest) -> ShareTribeResult<CurrentUser> {
        let token = self
            .auth
            .write()
            .map_err(|_| ShareTribeError::ConcurrencyError)?
            .get_token()?;
        let client = reqwest::Client::new();
        let url = format!("{}{}", BASE_URI, "api/current_user/create?expand=true");
        // let url = format!("https://cc56e343.ngrok.io/{}", "api/current_user/create?expand=true");
        let mut resp = client
            .post(&url)
            .bearer_auth(token)
            .json(&user)
            .send()
            .context(HttpError { url })?;

        let result: Response<CurrentUser> = resp.json_or_error()?;
        if let Some(errors) = result.errors {
            return ResponseError { errors }.fail();
        }
        Ok(result.data.unwrap())
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentUser {
    pub banned: bool,
    pub deleted: bool,
    pub created_at: DateTime<Utc>,
    pub email: String,
    pub email_verified: bool,
    pub pending_email: Option<String>,
    pub stripe_connected: bool,
    pub stripe_payouts_enabled: bool,
    pub stripe_charges_enabled: bool,
    pub profile: CurrentUserProfile,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentUserProfile {
    pub first_name: String,
    pub last_name: String,
    pub display_name: String,
    pub abbreviated_name: String,
    pub bio: Option<String>,
    pub public_data: Value,
    pub protected_data: Value,
    pub private_data: Value,
    pub metadata: Value,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCurrentUserRequest {
    pub email: String,
    pub password: String,
    pub first_name: String,
    pub last_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protected_data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_data: Option<Value>,
}
