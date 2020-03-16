use crate::ChargeResult;
use crate::Customer;
use crate::RefundResult;
use crate::StripeError;
use reqwest;

#[derive(Clone)]
pub struct StripeClient {
    api_key: String,
}

impl StripeClient {
    pub fn new(api_key: String) -> StripeClient {
        StripeClient { api_key }
    }

    pub fn charge(
        &self,
        token: &str,
        amount: i64,
        currency: &str,
        description: &str,
        metadata: Vec<(String, String)>,
    ) -> Result<ChargeResult, StripeError> {
        self.create_charge(token, amount, currency, description, true, metadata)
    }

    pub fn auth(
        &self,
        token: &str,
        amount: i64,
        currency: &str,
        description: &str,
        metadata: Vec<(String, String)>,
    ) -> Result<ChargeResult, StripeError> {
        self.create_charge(token, amount, currency, description, false, metadata)
    }

    pub fn update_metadata(
        &self,
        charge_id: &str,
        metadata: Vec<(String, String)>,
    ) -> Result<ChargeResult, StripeError> {
        let mut params = Vec::new();
        for key_value in metadata {
            params.push((format!("metadata[{}]", key_value.0), key_value.1));
        }

        let client = reqwest::Client::new();
        let mut resp = client
            .post(&format!("https://api.stripe.com/v1/charges/{}", charge_id))
            .basic_auth(&self.api_key, Some(""))
            .form(&params)
            .send()?;
        match resp.status() {
            reqwest::StatusCode::OK => {
                return ChargeResult::from_response(resp);
            }
            _ => return Err(StripeError::from_response(&mut resp)),
        }
    }

    fn create_charge(
        &self,
        token: &str,
        amount: i64,
        currency: &str,
        description: &str,
        capture: bool,
        metadata: Vec<(String, String)>,
    ) -> Result<ChargeResult, StripeError> {
        let mut params = vec![
            ("currency".to_string(), currency.to_string()),
            ("amount".to_string(), amount.to_string()),
            ("description".to_string(), description.to_string()),
            (
                if token.starts_with("tok_") {
                    "source".to_string()
                } else {
                    "customer".to_string()
                },
                token.to_string(),
            ),
            ("capture".to_string(), capture.to_string()),
        ];

        for key_value in metadata {
            params.push((format!("metadata[{}]", key_value.0), key_value.1));
        }
        let client = reqwest::Client::new();
        let mut resp = client
            .post("https://api.stripe.com/v1/charges")
            .basic_auth(&self.api_key, Some(""))
            .form(&params)
            .send()?;
        match resp.status() {
            reqwest::StatusCode::OK => {
                return ChargeResult::from_response(resp);
            }
            _ => return Err(StripeError::from_response(&mut resp)),
        }
    }

    pub fn refund(&self, charge_id: &str) -> Result<RefundResult, StripeError> {
        let params = vec![("charge".to_string(), charge_id.to_string())];

        let client = reqwest::Client::new();
        let mut resp = client
            .post("https://api.stripe.com/v1/refunds")
            .basic_auth(&self.api_key, Some(""))
            .form(&params)
            .send()?;
        match resp.status() {
            reqwest::StatusCode::OK => {
                return RefundResult::from_response(resp);
            }
            _ => return Err(StripeError::from_response(&mut resp)),
        }
    }

    pub fn partial_refund(&self, charge_id: &str, amount: i64) -> Result<RefundResult, StripeError> {
        let params = vec![
            ("charge".to_string(), charge_id.to_string()),
            ("amount".to_string(), amount.to_string()),
        ];

        let client = reqwest::Client::new();
        let mut resp = client
            .post("https://api.stripe.com/v1/refunds")
            .basic_auth(&self.api_key, Some(""))
            .form(&params)
            .send()?;
        match resp.status() {
            reqwest::StatusCode::OK => {
                return RefundResult::from_response(resp);
            }
            _ => return Err(StripeError::from_response(&mut resp)),
        }
    }

    pub fn complete(&self, charge_id: &str) -> Result<ChargeResult, StripeError> {
        let client = reqwest::Client::new();

        let mut resp = client
            .post(&format!("https://api.stripe.com/v1/charges/{}/capture", charge_id))
            .basic_auth(&self.api_key, Some(""))
            .send()?;
        match resp.status() {
            reqwest::StatusCode::OK => {
                return ChargeResult::from_response(resp);
            }
            _ => return Err(StripeError::from_response(&mut resp)),
        }
    }

    pub fn update_customer(
        &self,
        client_id: &str,
        description: &str,
        source: &str,
        metadata: Vec<(String, String)>,
    ) -> Result<Customer, StripeError> {
        let mut params = vec![
            ("description".to_string(), description.to_string()),
            ("source".to_string(), source.to_string()),
        ];

        for key_value in metadata {
            params.push((format!("metadata[{}]", key_value.0), key_value.1));
        }
        let client = reqwest::Client::new();
        let mut resp = client
            .post(&format!("https://api.stripe.com/v1/customers/{}", client_id,))
            .basic_auth(&self.api_key, Some(""))
            .form(&params)
            .send()?;
        match resp.status() {
            reqwest::StatusCode::OK => {
                return Customer::from_response(resp);
            }
            _ => return Err(StripeError::from_response(&mut resp)),
        }
    }

    pub fn create_customer(
        &self,
        description: &str,
        source: &str,
        metadata: Vec<(String, String)>,
    ) -> Result<Customer, StripeError> {
        let mut params = vec![
            ("description".to_string(), description.to_string()),
            ("source".to_string(), source.to_string()),
        ];

        for key_value in metadata {
            params.push((format!("metadata[{}]", key_value.0), key_value.1));
        }
        let client = reqwest::Client::new();
        let mut resp = client
            .post("https://api.stripe.com/v1/customers")
            .basic_auth(&self.api_key, Some(""))
            .form(&params)
            .send()?;
        match resp.status() {
            reqwest::StatusCode::OK => {
                return Customer::from_response(resp);
            }
            _ => return Err(StripeError::from_response(&mut resp)),
        }
    }
}
