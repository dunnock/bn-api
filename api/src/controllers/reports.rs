use crate::auth::user::User as AuthUser;
use crate::database::Connection;
use crate::errors::*;
use crate::helpers::application;
use crate::models::{PathParameters, WebPayload};
use actix_web::{
    http::StatusCode,
    web::{Path, Query},
    HttpResponse,
};
use chrono::prelude::*;
use db::models::*;
use serde_json::Value;
use std::collections::HashMap;
use std::str;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct ReportQueryParameters {
    pub report: String,
    pub start_utc: Option<NaiveDateTime>,
    pub end_utc: Option<NaiveDateTime>,
    pub event_id: Option<Uuid>,
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
        if let Some(ref start_utc) = s.start_utc {
            query_tags.insert("start_utc".to_owned(), json!(start_utc.clone()));
        }
        if let Some(ref end_utc) = s.end_utc {
            query_tags.insert("end_utc".to_owned(), json!(end_utc.clone()));
        }
        if let Some(ref event_id) = s.event_id {
            query_tags.insert("event_id".to_owned(), json!(event_id));
        }
        query_tags.insert("report".to_owned(), json!(s.report.clone()));

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

pub async fn get_report(
    (connection, query, path, user): (Connection, Query<ReportQueryParameters>, Path<PathParameters>, AuthUser),
) -> Result<HttpResponse, ApiError> {
    match query.report.trim() {
        "box_office_sales_summary" => box_office_sales_summary((connection, query, path, user)),
        "transaction_details" => Ok(transaction_detail_report((connection, query, path, user))?.into_http_response()?),
        "event_summary" => event_summary_report((connection, query, path, user)),
        "scan_count" => scan_counts((connection, query, user)),
        "weekly_settlement" => weekly_settlement_report((connection, query, path, user)),
        "ticket_count" => ticket_counts((connection, query, path, user)),
        "audit_report" => audit_report((connection, query, path, user)),
        "reconciliation_summary" => reconciliation_summary_report((connection, query, path, user)),
        "reconciliation_details" => reconciliation_detail_report((connection, query, path, user)),
        "promo_code" => promo_code_report((connection, query, path, user)),
        _ => application::not_found(),
    }
}

pub fn box_office_sales_summary(
    (connection, query, path, user): (Connection, Query<ReportQueryParameters>, Path<PathParameters>, AuthUser),
) -> Result<HttpResponse, ApiError> {
    let connection = connection.get();

    let organization = Organization::find(path.id, connection)?;
    user.requires_scope_for_organization(Scopes::OrgReports, &organization, connection)?;

    let result = Report::box_office_sales_summary_report(path.id, query.start_utc, query.end_utc, connection)?;
    Ok(HttpResponse::Ok().json(result))
}

pub fn transaction_detail_report(
    (connection, query, path, user): (Connection, Query<ReportQueryParameters>, Path<PathParameters>, AuthUser),
) -> Result<WebPayload<TransactionReportRow>, ApiError> {
    let connection = connection.get();
    //Check if they have org admin permissions
    let organization = Organization::find(path.id, connection)?;
    if let Some(event_id) = query.event_id {
        let event = Event::find(event_id, connection)?;
        user.requires_scope_for_organization_event(Scopes::EventFinancialReports, &organization, &event, connection)?;
    } else {
        user.requires_scope_for_organization(Scopes::OrgReports, &organization, connection)?;
    }

    let result = Report::transaction_detail_report(
        query.query.clone(),
        query.event_id,
        Some(path.id),
        query.start_utc,
        query.end_utc,
        query.page.unwrap_or(0),
        query.limit.unwrap_or(100),
        connection,
    )?;
    Ok(WebPayload::new(StatusCode::OK, result))
}

pub fn event_summary_report(
    (connection, query, path, user): (Connection, Query<ReportQueryParameters>, Path<PathParameters>, AuthUser),
) -> Result<HttpResponse, ApiError> {
    let connection = connection.get();
    //Check if they have org admin permissions
    let organization = Organization::find(path.id, connection)?;
    if let Some(event_id) = query.event_id {
        let event = Event::find(event_id, connection)?;
        user.requires_scope_for_organization_event(Scopes::EventFinancialReports, &organization, &event, connection)?;
    } else {
        return application::bad_request("event_id parameter is required");
    }

    let result = Report::summary_event_report(
        //We catch the is_none() above so I'll use unwrap here
        query.event_id.unwrap(),
        query.start_utc,
        query.end_utc,
        connection,
    )?;
    Ok(HttpResponse::Ok().json(result))
}

pub fn audit_report(
    (connection, query, path, user): (Connection, Query<ReportQueryParameters>, Path<PathParameters>, AuthUser),
) -> Result<HttpResponse, ApiError> {
    let connection = connection.get();
    //Check if they have org admin permissions
    let organization = Organization::find(path.id, connection)?;
    if let Some(event_id) = query.event_id {
        let event = Event::find(event_id, connection)?;
        user.requires_scope_for_organization_event(Scopes::EventFinancialReports, &organization, &event, connection)?;
    } else {
        return application::bad_request("event_id parameter is required");
    }

    let all_sales_result = Report::summary_event_report(
        //We catch the is_none() above so I'll use unwrap here
        query.event_id.unwrap(),
        query.start_utc,
        query.end_utc,
        connection,
    )?;

    let end_date = query.end_utc.unwrap_or(Utc::now().naive_utc());
    let end_date_sales_result = Report::summary_event_report(
        //We catch the is_none() above so I'll use unwrap here
        query.event_id.unwrap(),
        Some(end_date.date().and_hms(0, 0, 0)),
        Some(end_date),
        connection,
    )?;

    // ticket counts
    // TODO: update this query to do the inventory at end_date
    let ticket_counts = Report::ticket_count_report(query.event_id, Some(path.id), connection)?;

    Ok(HttpResponse::Ok().json(json!({
    "end_date_sales":end_date_sales_result,
    "all_sales": all_sales_result,
    "inventory": ticket_counts
    })))
}

pub fn weekly_settlement_report(
    (connection, query, path, user): (Connection, Query<ReportQueryParameters>, Path<PathParameters>, AuthUser),
) -> Result<HttpResponse, ApiError> {
    let connection = connection.get();
    //Check if they have org admin permissions
    let organization = Organization::find(path.id, connection)?;

    user.requires_scope_for_organization(Scopes::OrgFinancialReports, &organization, connection)?;

    let result = Report::organization_summary_report(path.id, query.start_utc, query.end_utc, connection)?;
    Ok(HttpResponse::Ok().json(result))
}

pub fn scan_counts(
    (connection, query, user): (Connection, Query<ReportQueryParameters>, AuthUser),
) -> Result<HttpResponse, ApiError> {
    let connection = connection.get();
    if let Some(event_id) = query.event_id {
        let event = Event::find(event_id, connection)?;
        let organization = event.organization(connection)?;
        user.requires_scope_for_organization_event(Scopes::ScanReportRead, &organization, &event, connection)?;

        let result = Report::scan_count_report(
            query.event_id.unwrap(),
            query.page.unwrap_or(0),
            query.limit.unwrap_or(100),
            connection,
        )?;
        Ok(HttpResponse::Ok().json(result))
    } else {
        application::bad_request("event_id parameter is required")
    }
}

pub fn ticket_counts(
    (connection, query, path, user): (Connection, Query<ReportQueryParameters>, Path<PathParameters>, AuthUser),
) -> Result<HttpResponse, ApiError> {
    let connection = connection.get();
    //Check if they have org admin permissions
    let organization = Organization::find(path.id, connection)?;
    if let Some(event_id) = query.event_id {
        let event = Event::find(event_id, connection)?;
        user.requires_scope_for_organization_event(Scopes::DashboardRead, &organization, &event, connection)?;
    } else {
        user.requires_scope_for_organization(Scopes::DashboardRead, &organization, connection)?;
    }

    let result = Report::ticket_count_report(query.event_id, Some(path.id), connection)?;
    Ok(HttpResponse::Ok().json(result))
}

pub fn promo_code_report(
    (connection, query, path, user): (Connection, Query<ReportQueryParameters>, Path<PathParameters>, AuthUser),
) -> Result<HttpResponse, ApiError> {
    let connection = connection.get();
    //Check if they have org admin permissions
    let organization = Organization::find(path.id, connection)?;
    if let Some(event_id) = query.event_id {
        let event = Event::find(event_id, connection)?;
        user.requires_scope_for_organization_event(Scopes::EventFinancialReports, &organization, &event, connection)?;
    } else {
        user.requires_scope_for_organization(Scopes::OrgReports, &organization, connection)?;
    }

    let result = Report::promo_code_report(query.event_id, Some(path.id), connection)?;
    Ok(HttpResponse::Ok().json(result))
}

pub fn reconciliation_summary_report(
    (connection, query, path, user): (Connection, Query<ReportQueryParameters>, Path<PathParameters>, AuthUser),
) -> Result<HttpResponse, ApiError> {
    let connection = connection.get();
    //Check if they have org admin permissions
    let organization = Organization::find(path.id, connection)?;

    user.requires_scope_for_organization(Scopes::OrgFinancialReports, &organization, connection)?;

    let result = Report::reconciliation_summary_report(path.id, query.start_utc, query.end_utc, connection)?;
    Ok(HttpResponse::Ok().json(result))
}

pub fn reconciliation_detail_report(
    (connection, query, path, user): (Connection, Query<ReportQueryParameters>, Path<PathParameters>, AuthUser),
) -> Result<HttpResponse, ApiError> {
    let connection = connection.get();
    //Check if they have org admin permissions
    let organization = Organization::find(path.id, connection)?;

    user.requires_scope_for_organization(Scopes::OrgFinancialReports, &organization, connection)?;

    let result = Report::reconciliation_detail_report(path.id, query.start_utc, query.end_utc, connection)?;
    Ok(HttpResponse::Ok().json(result))
}
