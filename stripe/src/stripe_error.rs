use reqwest;
use serde_json;
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;

#[derive(Debug)]
pub struct StripeError {
    pub description: String,
    pub cause: Option<Arc<dyn Error>>,
    pub error_code: Option<String>,
}

impl Error for StripeError {
    fn description(&self) -> &str {
        &self.description
    }
}

unsafe impl Send for StripeError {}
unsafe impl Sync for StripeError {}

use std::fmt;

impl fmt::Display for StripeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match &self.cause {
            Some(c) => write!(f, "{} caused by: {}", self.description, c.description()),
            None => write!(f, "{}", self.description),
        }
    }
}

impl StripeError {
    pub async fn from_response(response: reqwest::Response) -> StripeError {
        let status = response.status();
        let response_text = response.text().await.unwrap_or("<Error reading response body>".to_string());
        Self::response_to_stripe_error(status, &response_text)
    }

    pub fn from_response_blocking(response: reqwest::blocking::Response) -> StripeError {
        let status = response.status();
        let response_text = response.text().unwrap_or("<Error reading response body>".to_string());
        Self::response_to_stripe_error(status, &response_text)
    }

    fn response_to_stripe_error(status: reqwest::StatusCode, response_text: &str) -> StripeError {
        #[derive(Deserialize)]
        struct R {
            error: HashMap<String, String>,
        }
        let deserialized_response: Result<R, _> = serde_json::from_str(&response_text);
        let error_code = if let Ok(deserialized_response) = deserialized_response {
            deserialized_response.error.get("code".into()).map(|e| e.to_string())
        } else {
            None
        };
        StripeError {
            description: format!(
                "Error calling Stripe: HTTP Code {}: Body:{}",
                status,
                response_text
            ),
            cause: None,
            error_code,
        }
    }

}

impl From<reqwest::Error> for StripeError {
    fn from(r: reqwest::Error) -> Self {
        StripeError {
            description: format!("Error calling Stripe: reqwest error {}", r),
            cause: Some(Arc::new(r)),
            error_code: None,
        }
    }
}

impl From<serde_json::Error> for StripeError {
    fn from(r: serde_json::Error) -> Self {
        StripeError {
            description: format!("Error deserializing response:{}", r),
            cause: Some(Arc::new(r)),
            error_code: None,
        }
    }
}
