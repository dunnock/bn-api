use crate::auth::auth_client::AuthClient;
use crate::error::*;
use crate::result::ShareTribeResult;
use crate::util::HttpResponseExt;
use crate::{Response, BASE_URI};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use snafu::ResultExt;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

pub struct OwnListingEndpoint {
    auth: Arc<RwLock<AuthClient>>,
}

impl OwnListingEndpoint {
    pub fn new(auth: Arc<RwLock<AuthClient>>) -> OwnListingEndpoint {
        Self { auth }
    }
    pub fn create(&mut self, listing: CreateListingRequest) -> ShareTribeResult<OwnListing> {
        let token = self
            .auth
            .write()
            .map_err(|_| ShareTribeError::ConcurrencyError)?
            .get_token()?;
        let client = reqwest::blocking::Client::new();
        let url = format!("{}{}", BASE_URI, "api/own_listings/create?expand=true");
        // let url = format!(
        //     "{}{}",
        //     "https://06d2f6c2.ngrok.io/", "api/own_listings/create?expand=true,include=images"
        // );
        let resp = client
            .post(&url)
            .bearer_auth(token)
            .json(&listing)
            .send()
            .context(HttpError { url })?;

        let result: Response<OwnListing> = resp.json_or_error()?;
        if let Some(errors) = result.errors {
            return ResponseError { errors }.fail();
        }
        Ok(result.data.unwrap())
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnListing {
    pub title: String,
    pub description: Option<String>,
    pub geolocation: Option<Geolocation>,
    pub created_at: DateTime<Utc>,
    pub price: Price,
    // TODO: fill this in
    pub availability_plan: Value,
    pub public_data: Value,
    pub private_data: Value,
    pub metadata: Value,
    pub state: String,
    pub deleted: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateListingRequest {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geolocation: Option<Geolocation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<Price>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<Uuid>>,
}

#[derive(Serialize, Deserialize)]
pub struct Price {
    // in cents
    pub amount: i64,
    pub currency: String,
}

#[derive(Serialize, Deserialize)]
pub struct Geolocation {
    pub lat: f32,
    pub lng: f32,
}
