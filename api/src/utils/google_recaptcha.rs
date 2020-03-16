use crate::errors::BigNeonError;
use std::collections::HashMap;

const GOOGLE_RECAPTCHA_SITE_VERIFY_URL: &str = "https://www.google.com/recaptcha/api/siteverify";

#[derive(Debug, Deserialize)]
pub struct Response {
    pub success: bool,
    #[serde(rename = "error-codes")]
    pub error_codes: Option<Vec<String>>,
}

pub async fn verify_response(
    google_recaptcha_secret_key: &str,
    captcha_response: String,
    remote_ip: Option<&str>,
) -> Result<Response, BigNeonError> {
    let client = reqwest::Client::new();
    let mut params = HashMap::new();
    params.insert("secret", google_recaptcha_secret_key);
    params.insert("response", captcha_response.as_str());

    if let Some(val) = remote_ip {
        params.insert("remoteip", val);
    }

    let response: Response = client
        .post(GOOGLE_RECAPTCHA_SITE_VERIFY_URL)
        .form(&params)
        .send()
        .await?
        .json()
        .await?;

    if let Some(ref error_codes) = response.error_codes {
        warn!("Google captcha error encountered: {:?}", error_codes);
    }

    Ok(response)
}
