use crate::auth::user::User as AuthUser;
use crate::db::Connection;
use crate::errors::*;
use crate::helpers::application;
use crate::models::WebPayload;
use actix_web::{http::StatusCode, HttpResponse, web::Query};
use bigneon_db::models::*;
use chrono::prelude::*;
use serde_json::Value;
use std::collections::HashMap;
use std::str;

#[derive(Deserialize)]
pub struct ReportQueryParameters {
    pub name: String,
    pub transaction_start_utc: Option<NaiveDateTime>,
    pub transaction_end_utc: Option<NaiveDateTime>,
    pub event_start_utc: Option<NaiveDateTime>,
    pub event_end_utc: Option<NaiveDateTime>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    query: Option<String>,
    page: Option<u32>,
    limit: Option<u32>,
}

impl From<ReportQueryParameters> for Paging {
    fn from(s: ReportQueryParameters) -> Paging {
        let mut query_tags: HashMap<String, Value> = HashMap::new();
        if let Some(ref query) = s.query {
            query_tags.insert("query".to_owned(), json!(query.clone()));
        }
        if let Some(ref transaction_start_utc) = s.transaction_start_utc {
            query_tags.insert("transaction_start_utc".to_owned(), json!(transaction_start_utc.clone()));
        }
        if let Some(ref transaction_end_utc) = s.transaction_end_utc {
            query_tags.insert("transaction_end_utc".to_owned(), json!(transaction_end_utc.clone()));
        }
        if let Some(ref event_start_utc) = s.event_start_utc {
            query_tags.insert("event_start_utc".to_owned(), json!(event_start_utc.clone()));
        }
        if let Some(ref event_end_utc) = s.event_end_utc {
            query_tags.insert("event_end_utc".to_owned(), json!(event_end_utc.clone()));
        }
        query_tags.insert("name".to_owned(), json!(s.name.clone()));

        PagingParameters {
            page: s.page,
            limit: s.limit,
            dir: None,
            sort: None,
            tags: query_tags,
        }
        .into()
    }
}

pub fn get_report(
    (connection, query, user): (Connection, Query<ReportQueryParameters>, AuthUser),
) -> Result<HttpResponse, BigNeonError> {
    match query.name.trim() {
        "domain_transaction_detail" => {
            Ok(domain_transaction_detail_report((connection, query, user))?.into_http_response()?)
        }
        _ => application::not_found(),
    }
}

pub fn domain_transaction_detail_report(
    (connection, query, user): (Connection, Query<ReportQueryParameters>, AuthUser),
) -> Result<WebPayload<DomainTransactionReportRow>, BigNeonError> {
    let connection = connection.get();
    user.requires_scope(Scopes::ReportAdmin)?;

    let result = Report::domain_transaction_detail_report(
        query.transaction_start_utc,
        query.transaction_end_utc,
        query.event_start_utc,
        query.event_end_utc,
        query.page.unwrap_or(0),
        query.limit.unwrap_or(100),
        connection,
    )?;
    Ok(WebPayload::new(StatusCode::OK, result))
}
