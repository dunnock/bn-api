use crate::support;
use crate::support::database::TestDatabase;
use crate::support::test_request::TestRequest;
use actix_web::{FromRequest, http::StatusCode, HttpResponse, web::{Path, Query}};
use bigneon_api::controllers::holds;
use bigneon_api::controllers::holds::*;
use bigneon_api::extractors::*;
use bigneon_api::models::PathParameters;
use bigneon_db::models::*;
use serde_json;
use std::collections::HashMap;

pub async fn create(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let organization = database.create_organization().finish();
    let user = database.create_user().finish();
    let event = database
        .create_event()
        .with_tickets()
        .with_organization(&organization)
        .finish();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);

    let name = "Hold Example".to_string();
    let redemption_code = "IHAVEACODE".to_string();
    let discount_in_cents = Some(10);
    let hold_type = HoldTypes::Discount;

    let json = Json(CreateHoldRequest {
        name: name.clone(),
        redemption_code: Some(redemption_code.clone()),
        discount_in_cents,
        hold_type,
        end_at: None,
        max_per_user: None,
        quantity: 2,
        ticket_type_id: event.ticket_types(true, None, database.connection.get()).unwrap()[0].id,
    });

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = event.id;

    let response: HttpResponse = holds::create((database.connection.into(), json, path, auth_user)).await.into();

    if should_test_succeed {
        let body = support::unwrap_body_to_string(&response).unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let hold: DisplayHold = serde_json::from_str(&body).unwrap();
        assert_eq!(hold.name, name);
        assert_eq!(hold.redemption_code, Some(redemption_code));
        assert_eq!(hold.discount_in_cents, Some(10));
    } else {
        support::expects_unauthorized(&response);
    }
}

pub async fn split(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let hold = database.create_hold().finish();
    let event = Event::find(hold.event_id, connection).unwrap();
    let organization = event.organization(connection).unwrap();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = hold.id;

    let split_name = "Split".to_string();
    let redemption_code = "REDEEM1234".to_string();
    let json = Json(SplitHoldRequest {
        name: split_name.clone(),
        redemption_code: redemption_code.clone(),
        discount_in_cents: Some(5),
        hold_type: HoldTypes::Discount,
        quantity: 2,
        child: Some(true),
        end_at: None,
        max_per_user: None,
        email: None,
        phone: None,
    });

    let response: HttpResponse = holds::split((database.connection.clone(), json, path, auth_user)).await.into();
    let body = support::unwrap_body_to_string(&response).unwrap();

    if should_test_succeed {
        assert_eq!(response.status(), StatusCode::CREATED);
        let new_hold: Hold = serde_json::from_str(&body).unwrap();
        assert_eq!(new_hold.parent_hold_id, Some(hold.id));
        assert_eq!(new_hold.quantity(connection).unwrap().0, 2);
        assert_eq!(new_hold.name, split_name);
        assert_eq!(new_hold.discount_in_cents, Some(5));
        assert_eq!(new_hold.redemption_code, Some(redemption_code));
    } else {
        support::expects_unauthorized(&response);
    }
}

pub async fn children(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();

    let organization = database.create_organization().finish();
    let event = database
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let hold = database.create_hold().with_event(&event).finish();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);
    let comp = database
        .create_comp()
        .with_quantity(2)
        .with_hold(&hold)
        .with_name("Comp1".into())
        .finish();
    let comp2 = database
        .create_comp()
        .with_quantity(2)
        .with_hold(&hold)
        .with_name("Comp2".into())
        .finish();
    let hold2 = hold
        .split(
            Some(auth_user.id()),
            "Hold2".to_string(),
            None,
            None,
            "REDEEM282837".to_string(),
            2,
            Some(22),
            HoldTypes::Discount,
            None,
            None,
            true,
            connection,
        )
        .unwrap();
    let expected_holds = vec![
        comp.into_display(&connection).unwrap(),
        comp2.into_display(&connection).unwrap(),
        hold2.into_display(&connection).unwrap(),
    ];

    let test_request = TestRequest::create_with_uri(&format!("/limits?"));
    let query_parameters = Query::<PagingParameters>::extract(&test_request.request).await.unwrap();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = hold.id;

    let response = holds::children((database.connection.clone().into(), path, query_parameters, auth_user));
    let counter = expected_holds.len() as u32;
    let wrapped_expected_holds = Payload {
        data: expected_holds,
        paging: Paging {
            page: 0,
            limit: 100,
            sort: "".to_string(),
            dir: SortingDir::Asc,
            total: counter as u64,
            tags: HashMap::new(),
        },
    };

    if should_test_succeed {
        let response = response.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(wrapped_expected_holds, *response.payload());
    } else {
        assert_eq!(
            response.err().unwrap().to_string(),
            "User does not have the required permissions"
        );
    }
}

pub async fn update(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let hold = database.create_hold().finish();
    let event = Event::find(hold.event_id, connection).unwrap();
    let organization = event.organization(connection).unwrap();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);
    let name = "New Name";

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = hold.id;

    let json = Json(UpdateHoldRequest {
        name: Some(name.into()),
        quantity: Some(1),
        ..Default::default()
    });

    let response: HttpResponse = holds::update((database.connection.clone(), json, path, auth_user)).await.into();
    let body = support::unwrap_body_to_string(&response).unwrap();

    if should_test_succeed {
        assert_eq!(response.status(), StatusCode::OK);
        let updated_hold: Hold = serde_json::from_str(&body).unwrap();
        assert_eq!(updated_hold.name, name);
        assert_eq!(updated_hold.quantity(&connection).unwrap(), (1, 1));
    } else {
        support::expects_unauthorized(&response);
    }
}
