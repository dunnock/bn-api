use crate::StripeError;
use reqwest;
use serde_json;

pub struct Customer {
    pub id: String,
    pub raw_data: String,
}

impl Customer {
    pub fn to_json(&self) -> String {
        self.raw_data.clone()
    }
    pub fn from_response(mut resp: reqwest::Response) -> Result<Customer, StripeError> {
        let raw: String = resp.text()?;
        #[derive(Deserialize)]
        struct R {
            id: String,
        }
        let result: R = serde_json::from_str(&raw)?;
        Ok(Customer {
            id: result.id,
            raw_data: raw,
        })
    }
}
