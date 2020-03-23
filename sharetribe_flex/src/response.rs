use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct Response<T> {
    pub data: Option<ResponseData<T>>,
    pub errors: Option<Vec<ErrorData>>,
}

#[derive(Deserialize)]
pub struct ResponseData<T> {
    pub id: Uuid,
    #[serde(alias = "type")]
    pub response_type: Option<String>,
    pub attributes: Option<T>,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct ErrorData {
    pub id: Uuid,
    pub status: i64,
    pub code: String,
    pub title: String,
    pub details: Option<String>,
    pub source: Option<ErrorSource>,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct ErrorSource {
    pub path: Vec<String>,
    #[serde(alias = "type")]
    pub source_type: String,
}
