use actix_web::{http::StatusCode, FromRequest, HttpResponse, Path, Query};
use bigneon_api::controllers::venues::{self, NewVenueData};
use bigneon_api::extractors::*;
use bigneon_api::models::PathParameters;
use bigneon_db::models::*;
use serde_json;
use std::collections::HashMap;
use crate::support;
use crate::support::database::TestDatabase;
use crate::support::test_request::TestRequest;
use uuid::Uuid;

pub fn index(role: Roles, should_succeed: bool) {
    let database = TestDatabase::new();
    let venue = database.create_venue().with_name("Venue1".to_string()).finish();
    let venue2 = database.create_venue().with_name("Venue2".to_string()).finish();

    let expected_venues = vec![venue, venue2];
    let wrapped_expected_venues = Payload {
        data: expected_venues,
        paging: Paging {
            page: 0,
            limit: 100,
            sort: "".to_string(),
            dir: SortingDir::Asc,
            total: 2,
            tags: HashMap::new(),
        },
    };
    let expected_json = serde_json::to_string(&wrapped_expected_venues).unwrap();

    let test_request = TestRequest::create_with_uri(&format!("/limits?"));
    let query_parameters = Query::<PagingParameters>::extract(&test_request.request).unwrap();

    let user = support::create_auth_user(role, None, &database);
    let response: HttpResponse =
        venues::index((database.connection.into(), query_parameters, OptionalUser(Some(user)))).into();

    if !should_succeed {
        support::expects_unauthorized(&response);
        return;
    }
    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(body, expected_json);
}

pub fn create(role: Roles, should_succeed: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);

    let name = "Venue Example";
    let json = Json(NewVenueData {
        name: name.to_string(),
        organization_ids: vec![organization.id],
        timezone: "America/Los_Angeles".to_string(),
        ..Default::default()
    });

    let response: HttpResponse = venues::create((database.connection.clone().into(), json, auth_user.clone())).into();

    if !should_succeed {
        support::expects_unauthorized(&response);
        return;
    }
    assert_eq!(response.status(), StatusCode::CREATED);
    let body = support::unwrap_body_to_string(&response).unwrap();
    let venue: Venue = serde_json::from_str(&body).unwrap();
    assert_eq!(venue.name, name);
    assert_eq!(
        venue
            .organizations(connection)
            .unwrap()
            .into_iter()
            .map(|o| o.id)
            .collect::<Vec<Uuid>>(),
        vec![organization.id]
    );
    assert!(venue.is_private);
}

pub fn update(role: Roles, should_succeed: bool) {
    let database = TestDatabase::new();
    let venue = database.create_venue().finish();
    let new_name = "New Name";

    let user = support::create_auth_user(role, None, &database);
    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = venue.id;

    let mut attributes: VenueEditableAttributes = Default::default();
    attributes.name = Some(new_name.to_string());
    let json = Json(attributes);

    let response: HttpResponse = venues::update((database.connection.into(), path, json, user)).into();
    if !should_succeed {
        support::expects_unauthorized(&response);
        return;
    }
    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    let updated_venue: Venue = serde_json::from_str(&body).unwrap();
    assert_eq!(updated_venue.name, new_name);
}

pub fn toggle_privacy(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let venue = database.create_venue().finish();

    let auth_user = support::create_auth_user(role, None, &database);
    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = venue.id;

    let response: HttpResponse = venues::toggle_privacy((database.connection.into(), path, auth_user)).into();
    let body = support::unwrap_body_to_string(&response).unwrap();

    if should_test_succeed {
        assert_eq!(response.status(), StatusCode::OK);
        let updated_venue: Venue = serde_json::from_str(&body).unwrap();
        assert_ne!(updated_venue.is_private, venue.is_private)
    } else {
        support::expects_unauthorized(&response);
    }
}

pub fn update_with_organization(role: Roles, should_succeed: bool, is_private: bool) {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);

    let mut venue = database.create_venue().with_organization(&organization).finish();

    if is_private {
        venue = venue.set_privacy(true, database.connection.get()).unwrap();
    }
    let new_name = "New Name";

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = venue.id;

    let mut attributes: VenueEditableAttributes = Default::default();
    attributes.name = Some(new_name.to_string());
    let json = Json(attributes);

    let response: HttpResponse = venues::update((database.connection.into(), path, json, auth_user.clone())).into();
    if !should_succeed {
        support::expects_unauthorized(&response);
        return;
    }
    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    let updated_venue: Venue = serde_json::from_str(&body).unwrap();
    assert_eq!(updated_venue.name, new_name);
}

pub fn show_from_organizations(role: Option<Roles>, should_succeed: bool) {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let organization = database
        .create_organization()
        .with_member(&user, Roles::OrgMember)
        .finish();
    let venue = database.create_venue().with_name("Venue 1".to_string()).finish();
    let venue2 = database.create_venue().with_name("Venue 2".to_string()).finish();
    venue
        .add_to_organization(organization.id, database.connection.get())
        .unwrap();
    venue2
        .add_to_organization(organization.id, database.connection.get())
        .unwrap();

    let all_venues = vec![venue, venue2];
    let wrapped_expected_venues = Payload {
        data: all_venues,
        paging: Paging {
            page: 0,
            limit: 100,
            sort: "".to_string(),
            dir: SortingDir::Asc,
            total: 2,
            tags: HashMap::new(),
        },
    };
    let expected_json = serde_json::to_string(&wrapped_expected_venues).unwrap();

    let test_request = TestRequest::create();

    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = organization.id;

    let user = if role.is_some() {
        Some(support::create_auth_user(role.unwrap(), None, &database))
    } else {
        None
    };

    let test_request = TestRequest::create_with_uri(&format!("/limits?"));
    let query_parameters = Query::<PagingParameters>::extract(&test_request.request).unwrap();
    let response: HttpResponse =
        venues::show_from_organizations((database.connection.into(), path, query_parameters, OptionalUser(user)))
            .into();

    if !should_succeed {
        support::expects_unauthorized(&response);
        return;
    }
    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(expected_json, body);
}
