use crate::error::ShareTribeError;
use crate::error::{DeserializationError, InvalidResponse};
use serde::de::DeserializeOwned;
use snafu::ResultExt;

pub(crate) trait HttpResponseExt {
    fn json_or_error<T: DeserializeOwned>(self) -> Result<T, ShareTribeError>;
}

impl HttpResponseExt for reqwest::blocking::Response {
    fn json_or_error<T: DeserializeOwned>(self) -> Result<T, ShareTribeError> {
        let status = self.status();
        let body = self.text().context(InvalidResponse { status })?;
        let result: T = serde_json::from_str(&body).context(DeserializationError { body })?;
        Ok(result)
    }
}
