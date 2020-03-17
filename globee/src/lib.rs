//#![deny(unreachable_patterns)]
//#![deny(unused_variables)]
//#![deny(unused_imports)]
//// Unused results is more often than not an error
//#![deny(unused_must_use)]

use chrono::prelude::*;
use derive_error::Error;
use log::Level::Debug;
use logging::jlog;
use reqwest::header::HeaderName;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::error::Error as StdError;
use std::fmt;

#[derive(Serialize, Deserialize, Debug)]
pub struct GlobeeIpnRequest {
    pub id: String,
    pub status: Option<String>,
    pub total: Option<String>,
    pub adjusted_total: Option<String>,
    pub currency: Option<String>,
    pub custom_payment_id: Option<String>,
    pub custom_store_reference: Option<String>,
    pub callback_data: Option<String>,
    pub customer: Customer,
    pub payment_details: PaymentDetails,
    pub redirect_url: Option<String>,
    pub success_url: Option<String>,
    pub cancel_url: Option<String>,
    pub ipn_url: Option<String>,
    pub notification_email: Option<String>,
    pub confirmation_speed: Option<String>,
    pub expires_at: Option<String>,
    pub created_at: Option<String>,
}

pub struct GlobeeClient {
    key: String,
    base_url: String,
}

impl GlobeeClient {
    /// Creates a new Globee client
    /// base_url: Live: https://globee.com/payment-api/v1/, test: https://test.globee.com/payment-api/v1/
    pub fn new(key: String, base_url: String) -> GlobeeClient {
        GlobeeClient {
            key,
            base_url: if base_url.ends_with("/") {
                base_url
            } else {
                format!("{}/", base_url)
            },
        }
    }

    pub async fn create_payment_request(&self, request: PaymentRequest) -> Result<PaymentResponse, GlobeeError> {
        let client = reqwest::Client::new();
        jlog!(Debug, "Sending payment request to Globee", { "request": &request });

        let resp = client
            .post(&format!("{}payment-request", &self.base_url))
            .header(HeaderName::from_static("x-auth-key"), self.key.as_str())
            .json(&request)
            .send()
            .await?;
        let status = resp.status();
        if status != StatusCode::UNPROCESSABLE_ENTITY && status != StatusCode::OK {
            return Err(resp.error_for_status().err().map(|e| e.into()).unwrap_or(
                GlobeeError::UnexpectedResponseError(format!("Unexpected status code from Globee: {}", status)),
            ));
        };
        let value: serde_json::Value = resp.json().await?;
        jlog!(Debug, "Response from Globee", { "response": &value });
        let value: GlobeeResponse<PaymentResponse> = serde_json::from_value(value)?;

        if value.success {
            match value.data {
                Some(data) => Ok(data),
                None => Err(GlobeeError::UnexpectedResponseError(
                    "API did not return a response that was expected".to_string(),
                )),
            }
        } else {
            match value.errors {
                Some(errors) => Err(GlobeeError::ValidationError(Errors(errors))),
                None => Err(GlobeeError::UnexpectedResponseError(
                    "API did not return a response that was expected".to_string(),
                )),
            }
        }
    }

    pub fn get_payment_request_blocking(&self, id: &str) -> Result<GlobeeIpnRequest, GlobeeError> {
        let client = reqwest::blocking::Client::new();
        jlog!(Debug, "Retrieving payment request from Globee", { "id": id });

        let resp = client
            .get(&format!("{}payment-request/{}", &self.base_url, id))
            .header(HeaderName::from_static("x-auth-key"), self.key.as_str())
            .send()?;
        let status = resp.status();
        if status != StatusCode::UNPROCESSABLE_ENTITY && status != StatusCode::OK {
            return Err(resp.error_for_status().err().map(|e| e.into()).unwrap_or(
                GlobeeError::UnexpectedResponseError(format!("Unexpected status code from Globee: {}", status)),
            ));
        };
        let value: serde_json::Value = resp.json()?;
        jlog!(Debug, "Response from Globee", { "response": &value });
        let value: GlobeeResponse<GlobeeIpnRequest> = serde_json::from_value(value)?;

        if value.success {
            match value.data {
                Some(data) => Ok(data),
                None => Err(GlobeeError::UnexpectedResponseError(
                    "API did not return a response that was expected".to_string(),
                )),
            }
        } else {
            match value.errors {
                Some(errors) => Err(GlobeeError::ValidationError(Errors(errors))),
                None => Err(GlobeeError::UnexpectedResponseError(
                    "API did not return a response that was expected".to_string(),
                )),
            }
        }
    }
}

#[derive(Error, Debug)]
pub enum GlobeeError {
    ValidationError(Errors),
    HttpError(reqwest::Error),
    #[error(msg_embedded, no_from, non_std)]
    UnexpectedResponseError(String),
    DeserializationError(serde_json::Error),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct GlobeeResponse<T> {
    success: bool,
    data: Option<T>,
    errors: Option<Vec<ValidationError>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PaymentRequest {
    /// The total amount in the invoice currency.
    // TODO: Replace with numeric type
    total: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    /// A reference or custom identifier that you can use to link the payment back to your system.

    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_payment_id: Option<String>,
    /// Passthrough data that will be returned in the IPN callback.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callback_data: Option<String>,
    /// The customer making the payment
    pub customer: Customer,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancel_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ipn_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notification_email: Option<Email>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirmation_speed: Option<ConfirmationSpeed>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_store_reference: Option<String>,
}

impl PaymentRequest {
    pub fn new(
        total: f64,
        email: String,
        custom_payment_id: Option<String>,
        ipn_url: Option<String>,
        success_url: Option<String>,
        cancel_url: Option<String>,
    ) -> PaymentRequest {
        PaymentRequest {
            total: total.to_string(),
            currency: None,
            custom_payment_id,
            callback_data: None,
            customer: Customer {
                name: None,
                email: Email(email),
            },
            success_url,
            cancel_url,
            ipn_url,
            notification_email: None,
            confirmation_speed: None,
            custom_store_reference: None,
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct PaymentResponse {
    pub id: String,
    pub status: String,
    pub adjusted_total: Option<String>,

    #[serde(flatten)]
    pub request: PaymentRequest,
    pub redirect_url: String,
    pub payment_details: PaymentDetails,
    #[serde(with = "date_serialize")]
    pub expires_at: NaiveDateTime,
    #[serde(with = "date_serialize")]
    pub created_at: NaiveDateTime,
}

mod date_serialize {
    use chrono::prelude::*;
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<NaiveDateTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S").map_err(serde::de::Error::custom)
    }

    pub fn serialize<S>(value: &NaiveDateTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.format("%Y-%m-%d %H:%M:%S").to_string())
    }
}

use serde::Deserializer;
use serde::Serializer;
use serde_json::Value;
use std::str::FromStr;

fn from_str_or_num_to_num<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: Deserializer<'de>,
{
    let value: Value = Deserialize::deserialize(deserializer)?;

    if value.is_string() {
        let value = value.as_str();
        if let Some(s) = value {
            f64::from_str(s).map_err(serde::de::Error::custom).map(|f| Some(f))
        } else {
            Ok(None)
        }
    } else if value.is_number() {
        Ok(value.as_f64())
    } else {
        Ok(None)
    }
}

pub fn string_to_num<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: Deserializer<'de>,
{
    let value: Value = Deserialize::deserialize(deserializer)?;

    let value = value.as_str();
    if let Some(s) = value {
        f64::from_str(s).map_err(serde::de::Error::custom).map(|f| Some(f))
    } else {
        Ok(None)
    }
}

pub fn num_to_string<S>(value: &Option<f64>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Some(v) => serializer.serialize_str(v.to_string().as_str()),
        None => serializer.serialize_none(),
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum ConfirmationSpeed {
    High,
    Medium,
    Low,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(transparent)]
pub struct Email(String);
impl Email {
    pub fn new(s: String) -> Email {
        Email(s)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Customer {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub email: Email,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PaymentDetails {
    pub currency: Option<String>,

    #[serde(deserialize_with = "from_str_or_num_to_num", serialize_with = "num_to_string")]
    pub received_amount: Option<f64>,

    #[serde(deserialize_with = "from_str_or_num_to_num", serialize_with = "num_to_string")]
    pub received_difference: Option<f64>,
}

#[derive(Deserialize, Debug)]
pub struct Errors(Vec<ValidationError>);

impl StdError for Errors {
    fn description(&self) -> &str {
        "One or more errors occurred"
    }
}

impl fmt::Display for Errors {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct ValidationError {
    #[serde(rename = "type")]
    pub type_: String,
    pub extra: Option<Vec<String>>,
    pub field: String,
    pub message: String,
}

#[cfg(test)]
pub mod test {
    use super::*;

    #[derive(Deserialize, Debug, Serialize, Clone)]
    pub struct TestDeserialize {
        #[serde(deserialize_with = "from_str_or_num_to_num")]
        pub total: Option<f64>,
    }

    #[test]
    pub fn test_string_number_conversions() {
        let og = r#"
            {"total": "0.1"}
        "#;
        let response: TestDeserialize = serde_json::from_str(og).unwrap();
        assert_eq!(response.total, Some(0.1f64));
        let og = r#"
            {"total": 0.1}
        "#;
        let response: TestDeserialize = serde_json::from_str(og).unwrap();
        assert_eq!(response.total, Some(0.1f64));

        let og = r#"
            {"total": 0}
        "#;
        let response: TestDeserialize = serde_json::from_str(og).unwrap();
        assert_eq!(response.total, Some(0f64));

        let og = r#"
            {"total": "0"}
        "#;
        let response: TestDeserialize = serde_json::from_str(og).unwrap();
        assert_eq!(response.total, Some(0f64));

        let og = r#"
            {"total": 10900}
        "#;
        let response: TestDeserialize = serde_json::from_str(og).unwrap();
        assert_eq!(response.total, Some(10900f64));

        let og = r#"
            {"total": 1}
        "#;
        let response: TestDeserialize = serde_json::from_str(og).unwrap();
        assert_eq!(response.total, Some(1f64));
    }

    #[test]
    #[should_panic]
    pub fn test_converting_invalid_data() {
        let og = r#"
            {"total": "not_a_number"}
        "#;
        let _response: TestDeserialize = serde_json::from_str(og).unwrap();
    }

    #[test]
    pub fn deserialize_data() {
        let orig_data = r#"
            {
          "success": true,
          "data": {
            "id": "a1B2c3D4e5F6g7H8i9J0kL",
            "status": "unpaid",
            "total": "123.45",
            "currency": "USD",
            "custom_payment_id": "742",
            "custom_store_reference": "abc",
            "callback_data": "example data",
            "customer": {
              "name": "John Smit",
              "email": "john.smit@hotmail.com"
            },
            "payment_details": {
              "currency": "BTC",
              "received_amount" : "61.2",
              "received_difference" :"0"
            },
            "redirect_url": "http:\/\/globee.com\/invoice\/a1B2c3D4e5F6g7H8i9J0kL",
            "success_url": "https:\/\/www.example.com/success",
            "cancel_url": "https:\/\/www.example.com/cancel",
            "ipn_url": "https:\/\/www.example.com/globee/ipn-callback",
            "notification_email": null,
            "confirmation_speed": "medium",
            "expires_at": "2018-01-25 12:31:04",
            "created_at": "2018-01-25 12:16:04"
          }
        }
        "#;
        let response: GlobeeResponse<PaymentResponse> = serde_json::from_str(orig_data).unwrap();

        assert_eq!(response.data.as_ref().unwrap().id, "a1B2c3D4e5F6g7H8i9J0kL");
        let data = response.clone().data.unwrap();
        assert_eq!(data.status, "unpaid");
        assert_eq!(data.payment_details.received_amount, Some(61.2f64));
        assert_eq!(data.payment_details.received_difference, Some(0f64));
        assert!(response.success);

        assert!(response.errors.is_none());
    }

    #[test]
    pub fn deserialize_error() {
        let data = r#"
            {
              "success": false,
              "errors": [
                {
                  "type": "required_field",
                  "extra": null,
                  "field": "customer.email",
                  "message": "The customer email field is required."
                },
                {
                  "type": "invalid_number",
                  "extra": null,
                  "field": "total",
                  "message": "The total must be a number."
                },
                {
                  "type": "below_minimum",
                  "extra": [
                    "10"
                  ],
                  "field": "total",
                  "message": "The total must be at least 10."
                },
                {
                  "type": "invalid_selection",
                  "extra": [
                    "AFN",
                    "ALL",
                    "DZD",
                    "..."
                  ],
                  "field": "currency",
                  "message": "The selected currency is invalid."
                }
              ]
            }"#;
        let response: GlobeeResponse<PaymentResponse> = serde_json::from_str(data).unwrap();

        assert!(response.data.is_none());
        assert!(!response.success);

        let errors = response.errors.unwrap();
        assert_eq!(errors.len(), 4);
    }
}
