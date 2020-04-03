use crate::auth::access_token::AccessToken;
use crate::error::{HttpError, ShareTribeError};
use crate::util::HttpResponseExt;
use crate::BASE_URI;
use reqwest;
use serde::Serialize;
use snafu::ResultExt;
pub struct TokenEndpoint {
    pub credentials: TokenRequest,
}

impl TokenEndpoint {
    pub fn new(credentials: TokenRequest) -> TokenEndpoint {
        TokenEndpoint { credentials }
    }

    pub fn create(&self) -> Result<AccessToken, ShareTribeError> {
        let client = reqwest::blocking::Client::new();
        let url = format!("{}{}", BASE_URI, "auth/token");
        let resp = client
            .post(&url)
            .form(&self.credentials)
            .send()
            .context(HttpError { url })?;

        let access_token: AccessToken = resp.json_or_error()?;
        Ok(access_token)
    }
}

#[derive(Serialize)]

pub struct TokenRequest {
    pub client_id: String,
    pub scope: String,
    #[serde(flatten)]
    pub grant_type: GrantType,
}

#[derive(Serialize)]
#[serde(tag = "grant_type", rename_all = "snake_case")]
pub enum GrantType {
    #[serde(rename = "client_credentials")]
    Anonymous,
    Password {
        username: String,
        password: String,
    },
    RefreshToken {
        refresh_token: String,
    },
    Integration {
        client_secret: String,
    },
}

#[cfg(test)]
mod test {

    use serde_urlencoded;

    use super::*;
    #[test]
    fn serialize_password() {
        let req = TokenRequest {
            client_id: "asdf".to_string(),
            scope: "scope".to_string(),
            grant_type: GrantType::Password {
                username: "un".to_string(),
                password: "pw".to_string(),
            },
        };
        let actual = serde_urlencoded::to_string(&req).unwrap();
        assert_eq!(
            r#"client_id=asdf&scope=scope&grant_type=password&username=un&password=pw"#,
            actual
        );
    }

    #[test]
    fn serialize_anon() {
        let req = TokenRequest {
            client_id: "asdf".to_string(),
            scope: "scope".to_string(),
            grant_type: GrantType::Anonymous,
        };
        let actual = serde_urlencoded::to_string(&req).unwrap();
        assert_eq!(r#"client_id=asdf&scope=scope&grant_type=client_credentials"#, actual);
    }
}
