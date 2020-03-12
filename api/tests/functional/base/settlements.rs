use crate::support;
use crate::support::database::TestDatabase;
use crate::support::test_request::TestRequest;
use actix_web::{
    http::StatusCode,
    web::{Path, Query},
    FromRequest, HttpResponse,
};
use bigneon_api::controllers::settlements::{self, *};
use bigneon_api::extractors::*;
use bigneon_api::models::PathParameters;
use bigneon_db::prelude::*;
use bigneon_db::utils::dates;
use serde_json;
use uuid::Uuid;

pub async fn create(role: Roles, should_succeed: bool) {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let organization = database
        .create_organization()
        .with_settlement_type(SettlementTypes::Rolling)
        .finish();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);
    let start_time = dates::now().add_hours(-12).finish();
    let end_time = dates::now().add_hours(1).finish();
    let comment = "Example settlement comment".to_string();
    let json = Json(NewSettlementRequest {
        comment: Some(comment.clone()),
        start_time,
        end_time,
    });

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = organization.id;
    let response: HttpResponse = settlements::create((database.connection.into(), json, path, auth_user))
        .await
        .into();
    if !should_succeed {
        support::expects_unauthorized(&response);
        return;
    }
    assert_eq!(response.status(), StatusCode::CREATED);
    let body = support::unwrap_body_to_string(&response).unwrap();
    let settlement: Settlement = serde_json::from_str(&body).unwrap();
    assert_eq!(settlement.organization_id, organization.id);
    assert_eq!(settlement.comment, Some(comment));
    assert_eq!(settlement.start_time.timestamp(), start_time.timestamp());
    assert_eq!(settlement.end_time.timestamp(), end_time.timestamp());
    assert_eq!(settlement.only_finished_events, false);
}

pub async fn index(role: Roles, should_succeed: bool) {
    let database = TestDatabase::new();
    let organization = database.create_organization().finish();
    let organization2 = database.create_organization().finish();
    let user = database.create_user().finish();

    let settlement = database
        .create_settlement()
        .finalized()
        .with_organization(&organization)
        .finish();
    // Not finalized, not included in the response
    let _settlement2 = database.create_settlement().with_organization(&organization2).finish();
    let _settlement3 = database
        .create_settlement()
        .finalized()
        .with_organization(&organization2)
        .finish();

    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);
    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = organization.id;
    let query_parameters = Query::<PagingParameters>::extract(&test_request.request).await.unwrap();

    let response = settlements::index((database.connection.clone().into(), query_parameters, path, auth_user)).await;

    if should_succeed {
        let response = response.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            vec![settlement.id],
            response.payload().data.iter().map(|i| i.id).collect::<Vec<Uuid>>()
        );
    } else {
        assert_eq!(
            response.err().unwrap().to_string(),
            "User does not have the required permissions"
        );
    }
}

pub async fn show(role: Roles, should_succeed: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();

    let settlement = database
        .create_settlement()
        .finalized()
        .with_organization(&organization)
        .finish();

    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);
    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = settlement.id;
    let response: HttpResponse = settlements::show((database.connection.clone().into(), path, auth_user))
        .await
        .into();
    if !should_succeed {
        support::expects_unauthorized(&response);
        return;
    }
    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    let returned_settlement: DisplaySettlement = serde_json::from_str(&body).unwrap();
    assert_eq!(returned_settlement, settlement.for_display(connection).unwrap());
}

pub async fn destroy(role: Roles, should_succeed: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let settlement = database.create_settlement().with_organization(&organization).finish();

    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);
    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = settlement.id;
    let response: HttpResponse = settlements::destroy((database.connection.clone().into(), path, auth_user))
        .await
        .into();
    if !should_succeed {
        support::expects_unauthorized(&response);
        return;
    }
    assert_eq!(response.status(), StatusCode::OK);
    assert!(Settlement::find(settlement.id, connection).is_err());
}
