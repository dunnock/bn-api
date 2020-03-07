use actix_web::{http::StatusCode, FromRequest, HttpResponse, Path};
use bigneon_api::controllers::settlement_adjustments::{self, NewSettlementAdjustmentRequest};
use bigneon_api::extractors::Json;
use bigneon_api::models::PathParameters;
use bigneon_db::prelude::*;
use serde_json;
use crate::support;
use crate::support::database::TestDatabase;
use crate::support::test_request::TestRequest;

pub fn index(role: Roles, should_succeed: bool) {
    let database = TestDatabase::new();
    let organization = database.create_organization().finish();
    let user = database.create_user().finish();
    let settlement = database.create_settlement().with_organization(&organization).finish();
    let settlement_adjustment = database
        .create_settlement_adjustment()
        .with_settlement(&settlement)
        .finish();
    let _settlement_adjustment2 = database.create_settlement_adjustment().finish();

    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);
    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = settlement.id;

    let response = settlement_adjustments::index((database.connection.clone().into(), path, auth_user));

    if should_succeed {
        let response = response.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = support::unwrap_body_to_string(&response).unwrap();
        let settlement_adjustments: Vec<SettlementAdjustment> = serde_json::from_str(&body).unwrap();
        assert_eq!(vec![settlement_adjustment], settlement_adjustments);
    } else {
        assert_eq!(
            response.err().unwrap().to_string(),
            "User does not have the required permissions"
        );
    }
}

pub fn create(role: Roles, should_succeed: bool) {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let settlement = database.create_settlement().with_organization(&organization).finish();

    let json = Json(NewSettlementAdjustmentRequest {
        note: None,
        settlement_adjustment_type: SettlementAdjustmentTypes::ManualCredit,
        amount_in_cents: 100,
    });

    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);
    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = settlement.id;
    let response: HttpResponse =
        settlement_adjustments::create((database.connection.clone().into(), path, json, auth_user)).into();
    if !should_succeed {
        support::expects_unauthorized(&response);
        return;
    }
    assert_eq!(response.status(), StatusCode::CREATED);
    let body = support::unwrap_body_to_string(&response).unwrap();
    let returned_settlement_adjustment: SettlementAdjustment = serde_json::from_str(&body).unwrap();
    assert_eq!(returned_settlement_adjustment.settlement_id, settlement.id);
    assert_eq!(returned_settlement_adjustment.amount_in_cents, 100);
    assert_eq!(
        returned_settlement_adjustment.settlement_adjustment_type,
        SettlementAdjustmentTypes::ManualCredit
    );
}

pub fn destroy(role: Roles, should_succeed: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let settlement = database.create_settlement().with_organization(&organization).finish();
    let settlement_adjustment = database
        .create_settlement_adjustment()
        .with_settlement(&settlement)
        .finish();

    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);
    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = settlement_adjustment.id;
    let response: HttpResponse =
        settlement_adjustments::destroy((database.connection.clone().into(), path, auth_user)).into();
    if !should_succeed {
        support::expects_unauthorized(&response);
        return;
    }
    assert_eq!(response.status(), StatusCode::OK);
    assert!(SettlementAdjustment::find(settlement_adjustment.id, connection).is_err());
}
