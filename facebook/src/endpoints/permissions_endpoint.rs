use crate::error::{FacebookError, FacebookErrorResponse};
use crate::facebook_client::FacebookClientInner;
use crate::paging::Paging;
use crate::permission::Permission;
use reqwest::StatusCode;
use std::rc::Rc;

pub struct PermissionsEndpoint {
    pub client: Rc<FacebookClientInner>,
}

impl PermissionsEndpoint {
    pub async fn list(&self, user_id: &str) -> Result<Paging<Permission>, FacebookError> {
        if self.client.user_access_token.is_none() {
            return Err(FacebookError::Unauthorized);
        }

        let client = reqwest::Client::new();

        let resp = client
            .get(&format!("{}/{}/permissions", &self.client.base_url, user_id))
            .header(
                "Authorization",
                format!("Bearer {}", self.client.user_access_token.as_ref().unwrap()),
            )
            .send()
            .await?;
        let status = resp.status();
        let value: serde_json::Value = resp.json().await?;
        //        println!("{:?}", value.clone().to_string());

        if !status.is_success() {
            if status == StatusCode::UNAUTHORIZED {
                return Err(FacebookError::Unauthorized);
            }
            let error: FacebookErrorResponse = serde_json::from_value(value)?;
            return Err(FacebookError::FacebookError(error));
        }

        let results: Paging<Permission> = serde_json::from_value(value)?;
        Ok(results)
    }
}
