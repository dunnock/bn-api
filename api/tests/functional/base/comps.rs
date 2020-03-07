use actix_web::{http::StatusCode, FromRequest, HttpResponse, Path, Query};
use bigneon_api::controllers::comps::{self, NewCompRequest};
use bigneon_api::controllers::holds::UpdateHoldRequest;
use bigneon_api::extractors::*;
use bigneon_api::models::PathParameters;
use bigneon_db::models::*;
use serde_json;
use std::collections::HashMap;
use crate::support;
use crate::support::database::TestDatabase;
use crate::support::test_request::TestRequest;

pub fn index(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let hold = database.create_hold().with_hold_type(HoldTypes::Comp).finish();
    let event = Event::find(hold.event_id, connection).unwrap();
    let organization = event.organization(connection).unwrap();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);
    let comp1 = database
        .create_comp()
        .with_hold(&hold)
        .with_name("Comp1".into())
        .finish();
    let comp2 = database
        .create_comp()
        .with_hold(&hold)
        .with_name("Comp2".into())
        .finish();
    let expected_comps = vec![
        comp1.into_display(&connection).unwrap(),
        comp2.into_display(&connection).unwrap(),
    ];

    let test_request = TestRequest::create_with_uri(&format!("/limits?"));
    let query_parameters = Query::<PagingParameters>::extract(&test_request.request).unwrap();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = hold.id;

    let response = comps::index((database.connection.clone().into(), path, query_parameters, auth_user));
    let counter = expected_comps.len() as u32;
    let wrapped_expected_orgs = Payload {
        data: expected_comps,
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
        assert_eq!(wrapped_expected_orgs, *response.payload());
    } else {
        assert_eq!(
            response.err().unwrap().to_string(),
            "User does not have the required permissions"
        );
    }
}

pub fn show(role: Roles, should_succeed: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let comp = database.create_comp().finish();
    let comp_id = comp.id;
    let event = Event::find(comp.event_id, connection).unwrap();
    let organization = event.organization(connection).unwrap();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);

    let expected_json = serde_json::to_string(&comp.into_display(connection).unwrap()).unwrap();

    let test_request = TestRequest::create_with_uri_custom_params("/", vec!["id"]);
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = comp_id;

    let response: HttpResponse = comps::show((database.connection.clone().into(), path, auth_user)).into();

    if should_succeed {
        assert_eq!(response.status(), StatusCode::OK);
        let body = support::unwrap_body_to_string(&response).unwrap();
        assert_eq!(body, expected_json);
    } else {
        support::expects_unauthorized(&response);
    }
}

pub fn create(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let hold = database.create_hold().with_hold_type(HoldTypes::Comp).finish();
    let event = Event::find(hold.event_id, connection).unwrap();
    let organization = event.organization(connection).unwrap();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);

    let name = "Comp Example".to_string();
    let email = Some("email@address.com".to_string());
    let quantity = 10;

    let json = Json(NewCompRequest {
        name: name.clone(),
        email: email.clone(),
        phone: None,
        quantity,
        redemption_code: "OHHNOHEREITCOMES".to_string(),
        end_at: None,
        max_per_user: None,
    });

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = hold.id;

    let response = comps::create((database.connection.clone().into(), json, path, auth_user));

    if should_test_succeed {
        let response = response.unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let comp = response.data();
        assert_eq!(comp.name, name);
        assert_eq!(comp.parent_hold_id, Some(hold.id));
        assert_eq!(comp.email, email);
        assert_eq!(comp.quantity, 10);
    } else {
        assert_eq!(
            response.err().unwrap().to_string(),
            "User does not have the required permissions",
        );
    }
}

pub fn destroy(role: Roles, should_succeed: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let comp = database.create_comp().finish();
    let event = Event::find(comp.event_id, connection).unwrap();
    let organization = event.organization(connection).unwrap();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);

    let test_request = TestRequest::create_with_uri_custom_params("/", vec!["id"]);
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = comp.id;

    let response: HttpResponse = comps::destroy((database.connection.clone().into(), path, auth_user)).into();

    if should_succeed {
        assert_eq!(response.status(), StatusCode::OK);
        let comp = Hold::find(comp.id, connection).unwrap();
        assert!(comp.deleted_at.is_some());
    } else {
        support::expects_unauthorized(&response);
    }
}

pub fn update(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let comp = database.create_comp().finish();
    let event = Event::find(comp.event_id, connection).unwrap();
    let organization = event.organization(connection).unwrap();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);

    let name = "New Name";
    let test_request = TestRequest::create_with_uri_custom_params("/", vec!["id"]);
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = comp.id;

    let json = Json(UpdateHoldRequest {
        name: Some(name.into()),
        ..Default::default()
    });

    let response: HttpResponse = comps::update((database.connection.clone().into(), json, path, auth_user)).into();
    let body = support::unwrap_body_to_string(&response).unwrap();

    if should_test_succeed {
        assert_eq!(response.status(), StatusCode::OK);
        let updated_comp: DisplayHold = serde_json::from_str(&body).unwrap();
        assert_eq!(updated_comp.name, name);
    } else {
        support::expects_unauthorized(&response);
    }
}
