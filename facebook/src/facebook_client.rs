use crate::access_token::AccessToken;
use crate::endpoints::*;
use crate::error::FacebookError;
use std::rc::Rc;

use url::form_urlencoded;

pub struct FacebookClient {
    pub official_events: OfficialEventsEndpoint,
    pub me: MeEndpoint,
    pub inner_client: Rc<FacebookClientInner>,
    pub permissions: PermissionsEndpoint,
}

const BASE_URL: &str = "https://graph.facebook.com";
const API_VERSION: &str = "v5.0";

impl FacebookClient {
    pub fn from_page_access_token(access_token: String) -> FacebookClient {
        let inner = FacebookClientInner {
            base_url: BASE_URL,
            app_access_token: None,
            page_access_token: Some(access_token),
            user_access_token: None,
        };

        let inner = Rc::new(inner);

        FacebookClient {
            inner_client: inner.clone(),
            official_events: OfficialEventsEndpoint { client: inner.clone() },
            me: MeEndpoint::new(inner.clone()),
            permissions: PermissionsEndpoint { client: inner.clone() },
        }
    }

    pub async fn from_app_access_token(app_id: &str, app_secret: &str) -> Result<FacebookClient, FacebookError> {
        let inner = FacebookClientInner {
            base_url: BASE_URL,
            app_access_token: Some(
                FacebookClient::get_app_access_token(app_id, app_secret)
                    .await?
                    .access_token,
            ),
            page_access_token: None,
            user_access_token: None,
        };

        let inner = Rc::new(inner);

        Ok(FacebookClient {
            inner_client: inner.clone(),
            official_events: OfficialEventsEndpoint { client: inner.clone() },
            me: MeEndpoint::new(inner.clone()),
            permissions: PermissionsEndpoint { client: inner.clone() },
        })
    }

    pub fn from_user_access_token(access_token: String) -> FacebookClient {
        let inner = FacebookClientInner {
            base_url: BASE_URL,
            app_access_token: None,
            page_access_token: None,
            user_access_token: Some(access_token),
        };

        let inner = Rc::new(inner);

        FacebookClient {
            inner_client: inner.clone(),
            official_events: OfficialEventsEndpoint { client: inner.clone() },
            me: MeEndpoint::new(inner.clone()),
            permissions: PermissionsEndpoint { client: inner.clone() },
        }
    }

    pub fn get_login_url(app_id: &str, redirect_uri: Option<&str>, state: &str, scopes: &[&str]) -> String {
        let scope = scopes.iter().fold(
            "".to_string(),
            |s, t| {
                if s.len() == 0 {
                    t.to_string()
                } else {
                    s + "," + t
                }
            },
        );

        let result = form_urlencoded::Serializer::new(String::new())
            .append_pair("client_id", app_id)
            .append_pair(
                "redirect_uri",
                redirect_uri.unwrap_or("https://www.facebook.com/connect/login_success.html"),
            )
            .append_pair("state", state)
            .append_pair("scope", &scope)
            .finish();
        format!("https://www.facebook.com/{}/dialog/oauth?{}", API_VERSION, result)
    }

    pub async fn get_app_access_token(app_id: &str, app_secret: &str) -> Result<AccessToken, FacebookError> {
        let client = reqwest::Client::new();
        let resp = client
            .get(&format!(
                "{}/{}/oauth/access_token?client_id={}&client_secret={}&grant_type=client_credentials",
                BASE_URL, API_VERSION, app_id, app_secret,
            ))
            .send()
            .await?;
        //        let status = resp.status();
        let value: serde_json::Value = resp.json().await?;
        println!("{:?}", value);
        let result: AccessToken = serde_json::from_value(value)?;
        Ok(result)
    }
}

pub struct FacebookClientInner {
    pub base_url: &'static str,
    pub app_access_token: Option<String>,
    pub page_access_token: Option<String>,
    pub user_access_token: Option<String>,
}
