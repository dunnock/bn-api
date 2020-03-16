use access_token::AccessToken;
use facebook_client::FacebookClientInner;
use std::rc::Rc;

pub struct OAuthEndpoint {
    client: Rc<FacebookClientInner>,
}

impl OAuthEndpoint {
    pub fn new(client: Rc<FacebookClientInner>) -> OAuthEndpoint {
        OAuthEndpoint { client }
    }

    pub async fn access_token(&self, redirect_uri: Option<&str>, code: &str) -> AccessToken {
        let client = reqwest::Client::new();
         let mut resp = client
            .get(&format!(
                "{}/v3.2/oauth/access_token?client_id={}&redirect_uri={}&client_secret={}&code={}",
                &self.client.base_url,
                &self.client.app_id,
                redirect_uri.unwrap_or("https://www.facebook.com/connect/login_success.html"),
                &self.client.secret,
                code
            ))
            .send().await?;
        let status = resp.status();
        let value: serde_json::Value = resp.json().await?;
        println!("{:?}", value);
    }
}
