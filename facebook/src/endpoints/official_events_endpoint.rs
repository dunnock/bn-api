use crate::error::{FacebookError, FacebookErrorResponse};
use crate::facebook_client::FacebookClientInner;
use crate::facebook_request::FacebookRequest;
use crate::fbid::FBID;
use log::Level::Info;
use crate::nodes::Event;
use reqwest::StatusCode;
use std::rc::Rc;

pub struct OfficialEventsEndpoint {
    pub client: Rc<FacebookClientInner>,
}

impl OfficialEventsEndpoint {
    pub fn create(&self, event: Event) -> Result<FBID, FacebookError> {
        if self.client.app_access_token.is_none() {
            return Err(FacebookError::Unauthorized);
        }

        let request = FacebookRequest {
            access_token: self.client.app_access_token.as_ref().unwrap(),
            data: event,
        };

        let client = reqwest::Client::new();

        jlog!(Info, "Sending request to Facebook", { "request": &request });

        let mut resp = client
            .post(&format!("{}/v5.0/official_events", &self.client.base_url))
            .json(&request)
            .send()?;

        let value: serde_json::Value = resp.json()?;

        jlog!(Info, "Response from Facebook", { "response": &value });

        if resp.status() != StatusCode::OK {
            if let Some(error) = value.get("error") {
                let error: FacebookErrorResponse = serde_json::from_value(error.clone())?;
                return Err(FacebookError::FacebookError(error));
            }
            resp.error_for_status()?;
        }

        #[derive(Deserialize)]
        struct R {
            id: String,
        }

        Ok(FBID(serde_json::from_value::<R>(value)?.id))
    }
}
