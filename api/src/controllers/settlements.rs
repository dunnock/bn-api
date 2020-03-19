use crate::auth::user::User as AuthUser;
use crate::database::Connection;
use crate::errors::*;
use crate::extractors::*;
use crate::helpers::application;
use crate::models::{PathParameters, WebPayload};
use actix_web::{
    http::StatusCode,
    web::{Path, Query},
    HttpResponse,
};
use chrono::prelude::*;
use db::models::*;

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct NewSettlementRequest {
    pub start_time: NaiveDateTime,
    pub end_time: NaiveDateTime,
    pub comment: Option<String>,
}

pub async fn index(
    (connection, query, path, user): (Connection, Query<PagingParameters>, Path<PathParameters>, AuthUser),
) -> Result<WebPayload<Settlement>, ApiError> {
    let connection = connection.get();
    let organization = Organization::find(path.id, connection)?;
    user.requires_scope_for_organization(Scopes::SettlementRead, &organization, connection)?;

    let payload = Settlement::find_for_organization(
        path.id,
        Some(query.limit()),
        Some(query.page() * query.limit()),
        // Hide settlements for default settlement period where users lack settlement read early scope
        !user.has_scope_for_organization(Scopes::SettlementReadEarly, &organization, connection)?,
        connection,
    )?;

    Ok(WebPayload::new(StatusCode::OK, payload))
}

pub async fn create(
    (connection, new_settlement, path, user): (Connection, Json<NewSettlementRequest>, Path<PathParameters>, AuthUser),
) -> Result<HttpResponse, ApiError> {
    let connection = connection.get();
    let organization = Organization::find(path.id, connection)?;
    user.requires_scope_for_organization(Scopes::SettlementWrite, &organization, connection)?;
    let new_settlement = Settlement::create(
        organization.id,
        new_settlement.start_time,
        new_settlement.end_time,
        SettlementStatus::PendingSettlement,
        new_settlement.comment.clone(),
        organization.settlement_type == SettlementTypes::PostEvent,
    );
    let settlement = new_settlement.commit(Some(user.user), connection)?;
    Ok(HttpResponse::Created().json(&settlement))
}

pub async fn show(
    (connection, path, user): (Connection, Path<PathParameters>, AuthUser),
) -> Result<HttpResponse, ApiError> {
    let connection = connection.get();
    let settlement = Settlement::find(path.id, connection)?;
    let organization = Organization::find(settlement.organization_id, connection)?;
    user.requires_scope_for_organization(Scopes::SettlementRead, &organization, connection)?;

    // Unauthorized access to settlement for default settlement period where users lack settlement read early scope
    if !user.has_scope_for_organization(Scopes::SettlementReadEarly, &organization, connection)?
        && settlement.status != SettlementStatus::FinalizedSettlement
    {
        return application::unauthorized_with_message("Unauthorized access of settlement", None, None);
    }

    let display_settlement: DisplaySettlement = settlement.for_display(connection)?;
    Ok(HttpResponse::Ok().json(&display_settlement))
}

pub async fn destroy(
    (connection, path, user): (Connection, Path<PathParameters>, AuthUser),
) -> Result<HttpResponse, ApiError> {
    let connection = connection.get();
    user.requires_scope(Scopes::OrgAdmin)?;
    let settlement = Settlement::find(path.id, connection)?;
    settlement.destroy(connection)?;
    Ok(HttpResponse::Ok().json({}))
}
