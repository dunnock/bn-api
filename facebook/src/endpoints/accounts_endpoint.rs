use crate::edges::Account;
use crate::error::FacebookError;
use crate::facebook_client::FacebookClientInner;
use crate::paging::Paging;
use std::rc::Rc;

pub struct AccountsEndpoint {
    pub client: Rc<FacebookClientInner>,
}

impl AccountsEndpoint {
    pub async fn list(&self) -> Result<Paging<Account>, FacebookError> {
        let client = reqwest::Client::new();

        if let Some(access_token) = self.client.user_access_token.as_ref() {
            let resp = client
                .get(&format!("{}/me/accounts", &self.client.base_url))
                .header("Authorization", format!("Bearer {}", &access_token))
                .send()
                .await?;
            //        let status = resp.status();
            let value: serde_json::Value = resp.json().await?;
            println!("{:?}", value.clone().to_string());
            let results: Paging<Account> = serde_json::from_value(value)?;
            Ok(results)
        } else {
            Err(FacebookError::Unauthorized)
        }
    }
}
