use reqwest;
use serde_json;
use std::collections::HashMap;

use crate::errors::ApplicationError;

const GOOGLE_RECAPTCHA_SITE_VERIFY_URL: &str = "https://www.google.com/recaptcha/api/siteverify";

#[derive(Debug, Deserialize)]
pub struct Response {
    pub success: bool,
    #[serde(rename = "error-codes")]
    pub error_codes: Option<Vec<String>>,
}

pub fn verify_response(
    google_recaptcha_secret_key: &str,
    captcha_response: String,
    remote_ip: Option<&str>,
) -> Result<Response, ApplicationError> {
    let client = reqwest::Client::new();
    let mut params = HashMap::new();
    params.insert("secret", google_recaptcha_secret_key);
    params.insert("response", captcha_response.as_str());

    if let Some(val) = remote_ip {
        params.insert("remoteip", val);
    }

    let response = client
        .post(GOOGLE_RECAPTCHA_SITE_VERIFY_URL)
        .form(&params)
        .send()
        .map_err(|_err| ApplicationError::new("Error making recaptcha request".to_string()))?
        .text()
        .map_err(|_err| ApplicationError::new("Error getting recaptcha response".to_string()))?;

    let response = serde_json::from_str::<Response>(&response)
        .map_err(|_err| ApplicationError::new("Error parsing recaptcha response".to_string()))?;
    if let Some(error_codes) = response.error_codes.clone() {
        warn!("Google captcha error encountered: {:?}", error_codes);
    }

    Ok(response)
}
