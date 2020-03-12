use crate::support;
use crate::support::database::TestDatabase;
use crate::support::test_request::TestRequest;
use crate::support::unwrap_body_to_string;
use actix_web::{FromRequest, http::StatusCode, HttpResponse, web::{Path, Query}};
use bigneon_api::controllers::organization_venues;
use bigneon_api::extractors::*;
use bigneon_api::models::*;
use bigneon_db::models::*;
use serde_json;
use std::collections::HashMap;

pub async fn show(role: Roles, should_succeed: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);
    let venue = database.create_venue().finish();
    let organization_venue = OrganizationVenue::create(organization.id, venue.id)
        .commit(connection)
        .unwrap();

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = organization_venue.id;

    let response: HttpResponse = organization_venues::show((database.connection.clone(), path, auth_user)).await.into();

    if should_succeed {
        assert_eq!(response.status(), StatusCode::OK);
        let found_organization_venue: OrganizationVenue =
            serde_json::from_str(support::unwrap_body_to_string(&response).unwrap()).unwrap();
        assert_eq!(found_organization_venue.id, organization_venue.id);
        assert_eq!(found_organization_venue.venue_id, venue.id);
        assert_eq!(found_organization_venue.organization_id, organization.id);
    } else {
        support::expects_unauthorized(&response);
    }
}

pub async fn create(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);
    let venue = database.create_venue().finish();

    let json = Json(NewOrganizationVenue {
        organization_id: organization.id,
        venue_id: venue.id,
    });

    let response: HttpResponse =
        organization_venues::create((database.connection.into(), json, auth_user.clone())).await.into();
    let body = support::unwrap_body_to_string(&response).unwrap();

    if should_test_succeed {
        assert_eq!(response.status(), StatusCode::CREATED);
        let organization_venue: OrganizationVenue = serde_json::from_str(&body).unwrap();
        assert_eq!(organization_venue.organization_id, organization.id);
        assert_eq!(organization_venue.venue_id, venue.id);
    } else {
        support::expects_unauthorized(&response);
    }
}

pub async fn destroy(role: Roles, with_number_of_extra_venues: i64, should_succeed: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let venue = database.create_venue().finish();
    let organization = database.create_organization().finish();
    let organization_venue = OrganizationVenue::create(organization.id, venue.id)
        .commit(connection)
        .unwrap();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);
    for _ in 0..with_number_of_extra_venues {
        let new_organization = database.create_organization().finish();
        OrganizationVenue::create(new_organization.id, venue.id)
            .commit(connection)
            .unwrap();
    }

    let test_request = TestRequest::create_with_uri_custom_params("/", vec!["id"]);
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = organization_venue.id;

    let response: HttpResponse =
        organization_venues::destroy((database.connection.clone().into(), path, auth_user)).await.into();

    if should_succeed && with_number_of_extra_venues > 0 {
        assert_eq!(response.status(), StatusCode::OK);
        let organization_venue = OrganizationVenue::find(organization_venue.id, connection);
        assert!(organization_venue.is_err());
    } else if should_succeed {
        let expected_json = HttpResponse::UnprocessableEntity()
            .json(json!({
                "error": "Unable to remove organization venue link, at least one organization must be associated with venue"
            }));
        let expected_text = unwrap_body_to_string(&expected_json).unwrap();
        let body = unwrap_body_to_string(&response).unwrap();
        assert_eq!(body, expected_text);
    } else {
        support::expects_unauthorized(&response);
    }
}

pub async fn organizations_index(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let organization = database
        .create_organization()
        .with_name("Organization1".to_string())
        .finish();
    let organization2 = database
        .create_organization()
        .with_name("Organization2".to_string())
        .finish();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);
    let venue = database.create_venue().with_name("Venue1".to_string()).finish();
    let venue2 = database.create_venue().with_name("Venue2".to_string()).finish();
    let venue3 = database.create_venue().with_name("Venue3".to_string()).finish();
    let organization_venue = OrganizationVenue::create(organization.id, venue.id)
        .commit(connection)
        .unwrap();
    let organization_venue2 = OrganizationVenue::create(organization.id, venue2.id)
        .commit(connection)
        .unwrap();
    let _organization_venue3 = OrganizationVenue::create(organization2.id, venue2.id)
        .commit(connection)
        .unwrap();
    let _organization_venue4 = OrganizationVenue::create(organization2.id, venue3.id)
        .commit(connection)
        .unwrap();

    let test_request = TestRequest::create();
    let query_parameters = Query::<PagingParameters>::extract(&test_request.request).await.unwrap();

    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = organization.id;

    let response = organization_venues::organizations_index((
        database.connection.clone().into(),
        path,
        query_parameters,
        auth_user,
    )).await;

    let wrapped_expected_organization_venues = Payload {
        data: vec![
            OrganizationVenue {
                id: organization_venue.id,
                organization_id: organization_venue.organization_id,
                venue_id: organization_venue.venue_id,
                created_at: organization_venue.created_at,
                updated_at: organization_venue.updated_at,
            },
            OrganizationVenue {
                id: organization_venue2.id,
                organization_id: organization_venue2.organization_id,
                venue_id: organization_venue2.venue_id,
                created_at: organization_venue2.created_at,
                updated_at: organization_venue2.updated_at,
            },
        ],
        paging: Paging {
            page: 0,
            limit: 100,
            sort: "".to_string(),
            dir: SortingDir::Asc,
            total: 2 as u64,
            tags: HashMap::new(),
        },
    };

    if should_test_succeed {
        let response = response.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(wrapped_expected_organization_venues, *response.payload());
    } else {
        assert_eq!(
            response.err().unwrap().to_string(),
            "User does not have the required permissions"
        );
    }
}

pub async fn venues_index(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let organization = database
        .create_organization()
        .with_name("Organization1".to_string())
        .finish();
    let organization2 = database
        .create_organization()
        .with_name("Organization2".to_string())
        .finish();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);
    let venue = database.create_venue().with_name("Venue1".to_string()).finish();
    let venue2 = database.create_venue().with_name("Venue2".to_string()).finish();
    let venue3 = database.create_venue().with_name("Venue3".to_string()).finish();
    let _organization_venue = OrganizationVenue::create(organization.id, venue.id)
        .commit(connection)
        .unwrap();
    let organization_venue2 = OrganizationVenue::create(organization.id, venue2.id)
        .commit(connection)
        .unwrap();
    let organization_venue3 = OrganizationVenue::create(organization2.id, venue2.id)
        .commit(connection)
        .unwrap();
    let _organization_venue4 = OrganizationVenue::create(organization2.id, venue3.id)
        .commit(connection)
        .unwrap();

    let test_request = TestRequest::create();
    let query_parameters = Query::<PagingParameters>::extract(&test_request.request).await.unwrap();

    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = venue2.id;

    let response =
        organization_venues::venues_index((database.connection.clone().into(), path, query_parameters, auth_user)).await;

    let wrapped_expected_organization_venues = Payload {
        data: vec![
            OrganizationVenue {
                id: organization_venue2.id,
                organization_id: organization_venue2.organization_id,
                venue_id: organization_venue2.venue_id,
                created_at: organization_venue2.created_at,
                updated_at: organization_venue2.updated_at,
            },
            OrganizationVenue {
                id: organization_venue3.id,
                organization_id: organization_venue3.organization_id,
                venue_id: organization_venue3.venue_id,
                created_at: organization_venue3.created_at,
                updated_at: organization_venue3.updated_at,
            },
        ],
        paging: Paging {
            page: 0,
            limit: 100,
            sort: "".to_string(),
            dir: SortingDir::Asc,
            total: 2 as u64,
            tags: HashMap::new(),
        },
    };

    if should_test_succeed {
        let response = response.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(wrapped_expected_organization_venues, *response.payload());
    } else {
        assert_eq!(
            response.err().unwrap().to_string(),
            "User does not have the required permissions"
        );
    }
}
