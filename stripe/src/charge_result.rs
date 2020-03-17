use crate::StripeError;
use reqwest;
use serde_json;

#[derive(Deserialize, Serialize)]
pub struct ChargeResult {
    pub id: String,
    pub raw_data: String,
}

impl ChargeResult {
    pub fn to_json(&self) -> String {
        self.raw_data.clone()
    }
    pub async fn from_response(resp: reqwest::Response) -> Result<ChargeResult, StripeError> {
        let raw_data: String = resp.text().await?;
        #[derive(Deserialize)]
        struct R {
            id: String,
        }
        let result: R = serde_json::from_str(&raw_data)?;
        Ok(ChargeResult {
            id: result.id,
            raw_data,
        })
    }
}
