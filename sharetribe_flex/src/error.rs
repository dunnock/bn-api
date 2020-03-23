use crate::ErrorData;
use reqwest;
use reqwest::StatusCode;
use serde_json::json;
use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility = "pub")]
pub enum ShareTribeError {
    #[snafu(display("Error occurred when calling {}: {}", url, source))]
    HttpError { url: String, source: reqwest::Error },
    #[snafu(display("Error reading HTTP response. Status: {}, Error:{}", status, source))]
    InvalidResponse { status: StatusCode, source: reqwest::Error },
    #[snafu(display("Could not deserialize response body:{}, Error:{}", body, source))]
    DeserializationError { body: String, source: serde_json::Error },
    #[snafu(display("Sharetribe error returned: {}", json!(errors)))]
    ResponseError { errors: Vec<ErrorData> },
    #[snafu(display("Could not exclusively lock auth object because the mutex is poisoned",))]
    ConcurrencyError,
}
