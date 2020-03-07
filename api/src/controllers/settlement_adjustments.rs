use actix_web::{HttpResponse, Path};
use crate::auth::user::User as AuthUser;
use bigneon_db::models::*;
use crate::db::Connection;
use crate::errors::*;
use crate::extractors::*;
use crate::helpers::application;
use crate::models::PathParameters;

pub fn index(
    (connection, path, user): (Connection, Path<PathParameters>, AuthUser),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let settlement = Settlement::find(path.id, connection)?;
    let organization = Organization::find(settlement.organization_id, connection)?;
    user.requires_scope_for_organization(Scopes::OrgFinancialReports, &organization, connection)?;
    Ok(HttpResponse::Ok().json(settlement.adjustments(connection)?))
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct NewSettlementAdjustmentRequest {
    pub amount_in_cents: i64,
    pub note: Option<String>,
    pub settlement_adjustment_type: SettlementAdjustmentTypes,
}

pub fn create(
    (connection, path, json, user): (
        Connection,
        Path<PathParameters>,
        Json<NewSettlementAdjustmentRequest>,
        AuthUser,
    ),
) -> Result<HttpResponse, BigNeonError> {
    user.requires_scope(Scopes::SettlementAdjustmentWrite)?;
    let connection = connection.get();

    let settlement = Settlement::find(path.id, connection)?;
    if settlement.status == SettlementStatus::FinalizedSettlement {
        return application::forbidden("Unable to create new adjustments, settlement has been finalized");
    }

    let settlement_adjustment = SettlementAdjustment::create(
        path.id,
        json.settlement_adjustment_type,
        json.note.clone(),
        json.amount_in_cents,
    )
    .commit(connection)?;
    Ok(HttpResponse::Created().json(&settlement_adjustment))
}

pub fn destroy(
    (connection, path, user): (Connection, Path<PathParameters>, AuthUser),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    user.requires_scope(Scopes::SettlementAdjustmentDelete)?;
    let settlement_adjustment = SettlementAdjustment::find(path.id, connection)?;
    let settlement = Settlement::find(settlement_adjustment.settlement_id, connection)?;
    if settlement.status == SettlementStatus::FinalizedSettlement {
        return application::forbidden("Unable to delete adjustments, settlement has been finalized");
    }

    settlement_adjustment.destroy(connection)?;
    Ok(HttpResponse::Ok().json({}))
}
