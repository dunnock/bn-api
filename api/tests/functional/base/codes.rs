use crate::support;
use crate::support::database::TestDatabase;
use crate::support::test_request::TestRequest;
use actix_web::{http::StatusCode, FromRequest, HttpResponse, web::Path};
use bigneon_api::controllers::codes::{self, *};
use bigneon_api::extractors::*;
use bigneon_api::models::PathParameters;
use bigneon_db::models::*;
use chrono::prelude::*;
use chrono::Duration;
use chrono::NaiveDateTime;
use serde_json;

pub async fn show(role: Roles, should_succeed: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let event = database.create_event().finish();
    let organization = event.organization(connection).unwrap();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);
    let code = database.create_code().with_event(&event).finish();
    let expected_json = serde_json::to_string(&code.for_display(connection).unwrap()).unwrap();

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = code.id;

    let response: HttpResponse = codes::show((database.connection.clone(), path, auth_user)).await.into();

    if should_succeed {
        assert_eq!(response.status(), StatusCode::OK);
        let body = support::unwrap_body_to_string(&response).unwrap();
        assert_eq!(body, expected_json);
    } else {
        support::expects_unauthorized(&response);
    }
}

pub async fn destroy(role: Roles, should_succeed: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let event = database.create_event().finish();
    let organization = event.organization(connection).unwrap();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);
    let code = database.create_code().with_event(&event).finish();

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = code.id;

    let response: HttpResponse = codes::destroy((database.connection.clone().into(), path, auth_user)).await.into();

    if should_succeed {
        assert_eq!(response.status(), StatusCode::OK);
        let code = Code::find(code.id, connection);
        assert!(code.is_err());
    } else {
        support::expects_unauthorized(&response);
    }
}

pub async fn create(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let event = database.create_event().with_ticket_pricing().finish();
    let ticket_type_id = event.ticket_types(true, None, connection).unwrap()[0].id;
    let organization = event.organization(connection).unwrap();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);

    let name = "Code Example".to_string();
    let redemption_code = "REDEEMCODE".to_string();

    let start_date = NaiveDateTime::from(Utc::now().naive_utc() - Duration::days(1));
    let end_date = NaiveDateTime::from(Utc::now().naive_utc() + Duration::days(2));
    let json = Json(CreateCodeRequest {
        name: name.clone(),
        redemption_codes: vec![redemption_code.clone()],
        code_type: CodeTypes::Discount,
        max_uses: 10,
        discount_in_cents: Some(100),
        discount_as_percentage: None,
        start_date: Some(start_date),
        end_date: Some(end_date),
        max_tickets_per_user: None,
        ticket_type_ids: vec![ticket_type_id],
    });

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = event.id;

    let response: HttpResponse = codes::create((database.connection.clone().into(), json, path, auth_user)).await.into();

    if should_test_succeed {
        let body = support::unwrap_body_to_string(&response).unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let code: DisplayCode = serde_json::from_str(&body).unwrap();
        assert_eq!(code.name, name);
        assert_eq!(code.redemption_codes, vec![redemption_code]);
        assert_eq!(code.max_uses, 10);
        assert_eq!(code.discount_in_cents, Some(100));
        assert_eq!(code.ticket_type_ids, vec![ticket_type_id]);
    } else {
        support::expects_unauthorized(&response);
    }
}

pub async fn update(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let event = database.create_event().with_ticket_pricing().finish();
    let ticket_type_id = event.ticket_types(true, None, connection).unwrap()[0].id;
    let code = database.create_code().with_event(&event).finish();
    let organization = event.organization(connection).unwrap();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);

    let name = "New Name";
    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = code.id;

    let json = Json(UpdateCodeRequest {
        name: Some(name.into()),
        ticket_type_ids: Some(vec![ticket_type_id]),
        ..Default::default()
    });

    let response: HttpResponse = codes::update((database.connection.clone().into(), json, path, auth_user)).await.into();
    let body = support::unwrap_body_to_string(&response).unwrap();

    if should_test_succeed {
        assert_eq!(response.status(), StatusCode::OK);
        let updated_code: DisplayCode = serde_json::from_str(&body).unwrap();
        assert_eq!(updated_code.name, name);
        assert_eq!(updated_code.ticket_type_ids, vec![ticket_type_id]);
    } else {
        support::expects_unauthorized(&response);
    }
}
