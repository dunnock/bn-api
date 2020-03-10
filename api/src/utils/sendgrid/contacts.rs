use crate::errors::*;
use reqwest::blocking::Client;
use serde_json;
use std::convert::From;

const SENDGRID_API_URL: &'static str = "https://api.sendgrid.com/v3";
const LOG_TARGET: &'static str = "bigneon::utils::sendgrid";

#[derive(Clone, Default, Serialize)]
pub struct SGContact {
    pub email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_name: Option<String>,
}

#[derive(Clone, Deserialize)]
pub struct SGErrorDetail {
    pub message: String,
    pub error_indices: Option<Vec<i64>>,
}

#[derive(Clone, Default, Deserialize)]
pub struct SGCreateContactResponse {
    pub new_count: u32,
    pub updated_count: u32,
    pub error_count: u32,
    pub error_indices: Vec<u32>,
    pub unmodified_indices: Vec<u32>,
    pub persisted_recipients: Vec<String>,
    pub errors: Vec<SGErrorDetail>,
}

impl From<serde_json::Value> for SGCreateContactResponse {
    fn from(v: serde_json::Value) -> Self {
        jlog!(log::Level::Debug, LOG_TARGET, "Got reponse from sendgrid", v);

        serde_json::from_value(v).unwrap()
    }
}

impl SGContact {
    pub fn new(email: String, first_name: Option<String>, last_name: Option<String>) -> Self {
        Self {
            email,
            first_name,
            last_name,
        }
    }

    pub fn create(&self, api_key: &str) -> Result<SGCreateContactResponse, BigNeonError> {
        let client = Client::new();
        let msg_body = json!(vec![self]).to_string();
        send_request_json(api_key, client.post(Self::api_url(None).as_str()).body(msg_body)).map(|json| json.into())
    }

    pub fn create_many(api_key: &str, contacts: Vec<Self>) -> Result<SGCreateContactResponse, BigNeonError> {
        let client = Client::new();
        let msg_body = json!(contacts).to_string();
        send_request_json(api_key, client.post(Self::api_url(None).as_str()).body(msg_body)).map(|json| json.into())
    }

    fn api_url(recipient_id: Option<String>) -> String {
        let base_url = SENDGRID_API_URL.to_owned();
        let id = recipient_id.map(|s| format!("/{}", s)).unwrap_or("".to_string());
        base_url + &format!("/contactdb/recipients{recipient_id}", recipient_id = id)
    }
}

#[derive(Clone, Default, Serialize)]
pub struct SGContactList {
    pub name: String,
}

impl SGContactList {
    pub fn new(name: String) -> Self {
        Self { name }
    }

    pub async fn get_async(
        &self,
        api_key: &str,
        id: String,
    ) -> Result<SGContactListResponse, BigNeonError> {
        reqwest::Client::new()
            .get(Self::api_url(Some(id)).as_str())
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .header("user-agent", "sendgrid-rs")
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
            .map_err(|err| err.into())
    }

    pub fn create(&self, api_key: &str) -> Result<SGContactListResponse, BigNeonError> {
        let client = Client::new();
        let msg_body = self.to_json();
        send_request_json(api_key, client.post(&Self::api_url(None)).body(msg_body)).map(|json| json.into())
    }

    pub fn create_or_return(&self, api_key: &str) -> Result<SGContactListResponse, BigNeonError> {
        let client = Client::new();
        let msg_body = self.to_json();
        let req = client.post(&Self::api_url(None)).body(msg_body);

        send_request(api_key, req)
            .and_then(|r| r.error_for_status())
            .and_then(|mut r| r.json())
            .map(|json: serde_json::Value| json.into())
            .or_else(|err| {
                if err.is_client_error() {
                    // 4XX error
                    // Possible duplicate error being returned from sendgrid
                    // Fetch all lists and if there's a matching name, return the list object
                    let lists =
                        Self::fetch_all(api_key).map_err(|err| ApplicationError::new(format!("{}", err).into()))?;

                    let found_list = lists.iter().find(|l| l.name == self.name);
                    if let Some(list) = found_list {
                        Ok(list.clone())
                    } else {
                        Err(BigNeonError::new(Box::new(err)))
                    }
                } else {
                    Err(BigNeonError::new(Box::new(err)))
                }
            })
    }

    pub fn get_by_id(api_key: &str, id: u64) -> Result<SGContactListResponse, BigNeonError> {
        let client = Client::new();
        send_request_json(api_key, client.get(&Self::api_url(Some(id.to_string())))).map(|json| json.into())
    }

    pub fn fetch_all(api_key: &str) -> Result<Vec<SGContactListResponse>, BigNeonError> {
        let client = Client::new();
        let req = client.get(&Self::api_url(None));
        send_request_json(api_key, req).and_then(|json| {
            match json.get("lists") {
                Some(lists) => serde_json::from_value::<Vec<SGContactListResponse>>(lists.clone())
                    .map_err(|err| ApplicationError::new(format!("{}", err))),
                None => Err(ApplicationError::new(
                    "Unexpected sendgrid result: no lists key in result".to_string(),
                )),
            }
            .map_err(|err| err.into())
        })
    }

    fn api_url(list_id: Option<String>) -> String {
        let base_url = SENDGRID_API_URL.to_owned();
        let id = list_id.map(|l| format!("/{}", l)).unwrap_or("".to_string());
        base_url + &format!("/contactdb/lists{list_id}", list_id = id)
    }

    fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

#[derive(Clone, Deserialize)]
pub struct SGContactListResponse {
    pub id: u64,
    pub name: String,
    pub recipient_count: u32,
}

impl From<serde_json::Value> for SGContactListResponse {
    fn from(v: serde_json::Value) -> Self {
        jlog!(log::Level::Debug, LOG_TARGET, "Got reponse from sendgrid", v);
        serde_json::from_value(v).unwrap()
    }
}

impl SGContactListResponse {
    pub fn add_recipients(&self, api_key: &str, recipient_ids: Vec<String>) -> Result<(), BigNeonError> {
        let client = Client::new();
        let msg_body = json!(recipient_ids).to_string();
        let req = client.post(&Self::api_url(self.id.to_string())).body(msg_body);

        // Sendgrid sends back a blank response on success
        send_request(api_key, req)
            .and_then(|r| r.error_for_status())
            .map(|_r| ())
            .map_err(|err| ApplicationError::new(err.to_string()).into())
    }

    fn api_url(list_id: String) -> String {
        let base_url = SENDGRID_API_URL.to_owned();
        base_url + &format!("/contactdb/lists/{list_id}/recipients", list_id = list_id)
    }
}

fn send_request_json(api_key: &str, req: reqwest::RequestBuilder) -> Result<serde_json::Value, BigNeonError> {
    send_request(api_key, req)
        .and_then(|r| r.error_for_status())
        .and_then(|mut r| r.json())
        .map_err(|err| ApplicationError::new(err.to_string()).into())
}

fn send_request(api_key: &str, req: reqwest::RequestBuilder) -> reqwest::Result<reqwest::Response> {
    req.header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .header("user-agent", "sendgrid-rs")
        .send()
}

#[test]
fn test_sg_contact_api_url() {
    assert_eq!(
        SGContact::api_url(None),
        "https://api.sendgrid.com/v3/contactdb/recipients"
    );
    assert_eq!(
        SGContact::api_url(Some("abc123".to_string())),
        "https://api.sendgrid.com/v3/contactdb/recipients/abc123"
    );
}
