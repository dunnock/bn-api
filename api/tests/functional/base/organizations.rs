use crate::support;
use crate::support::database::TestDatabase;
use crate::support::test_request::TestRequest;
use actix_web::ResponseError;
use actix_web::{http::StatusCode, HttpResponse, web::{Path, Query}};
use bigneon_api::controllers::organizations;
use bigneon_api::controllers::organizations::*;
use bigneon_api::extractors::*;
use bigneon_api::models::{OrganizationUserPathParameters, PathParameters};
use bigneon_db::models::*;
use chrono::NaiveDateTime;
use serde_json;
use std::collections::HashMap;
use uuid::Uuid;

pub fn index(role: Roles) {
    let database = TestDatabase::new();
    let user = database.create_user().finish();

    let organization = database
        .create_organization()
        .with_name("Organization 1".to_string())
        .finish();

    let organization2 = if ![Roles::User, Roles::Admin].contains(&role) {
        database.create_organization_with_user(&user, role == Roles::OrgOwner)
    } else {
        database.create_organization()
    }
    .with_name("Organization 2".to_string())
    .finish();

    let user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);
    // reload organization
    let organization = Organization::find(organization.id, database.connection.get()).unwrap();
    let expected_organizations = if role != Roles::User {
        vec![organization.clone(), organization2]
    } else {
        Vec::new()
    };

    let test_request = TestRequest::create_with_uri(&format!("/limits?"));
    let query_parameters = Query::<PagingParameters>::extract(&test_request.request).unwrap();
    let response: HttpResponse = organizations::index((database.connection.into(), query_parameters, user)).into();
    let body = support::unwrap_body_to_string(&response).unwrap();
    let counter = expected_organizations.len();
    let wrapped_expected_orgs = Payload {
        data: expected_organizations,
        paging: Paging {
            page: 0,
            limit: 100,
            sort: "".to_string(),
            dir: SortingDir::Asc,
            total: counter as u64,
            tags: HashMap::new(),
        },
    };

    let expected_json = serde_json::to_string(&wrapped_expected_orgs).unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body, expected_json);
}

pub fn show(role: Roles, should_succeed: bool) {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);

    // reload organization
    let organization = Organization::find(organization.id, database.connection.get()).unwrap();
    let organization_expected_json = serde_json::to_string(&organization).unwrap();

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = organization.id;
    let response: HttpResponse = organizations::show((
        test_request.extract_state(),
        database.connection.into(),
        path,
        auth_user.clone(),
    ))
    .into();

    if !should_succeed {
        support::expects_unauthorized(&response);
        return;
    }

    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(body, organization_expected_json);
}

pub fn index_for_all_orgs(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();

    let user = database.create_user().finish();
    let user2 = database.create_user().finish();

    let organization = database
        .create_organization()
        .with_name("Organization 1".to_string())
        .with_member(&user, Roles::OrgOwner)
        .finish();
    let organization2 = database
        .create_organization()
        .with_name("Organization 2".to_string())
        .with_member(&user2, Roles::OrgOwner)
        .finish();

    let expected_organizations = vec![organization.clone(), organization2];

    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);
    let test_request = TestRequest::create_with_uri(&format!("/limits?"));
    let query_parameters = Query::<PagingParameters>::extract(&test_request.request).unwrap();
    let response: HttpResponse =
        organizations::index_for_all_orgs((database.connection.into(), query_parameters, auth_user)).into();

    let body = support::unwrap_body_to_string(&response).unwrap();

    let counter = expected_organizations.len();
    let wrapped_expected_orgs = Payload {
        data: expected_organizations,
        paging: Paging {
            page: 0,
            limit: 100,
            sort: "".to_string(),
            dir: SortingDir::Asc,
            total: counter as u64,
            tags: HashMap::new(),
        },
    };

    let expected_json = serde_json::to_string(&wrapped_expected_orgs).unwrap();
    if should_test_succeed {
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(body, expected_json);
    } else {
        support::expects_unauthorized(&response);
    }
}

pub fn create(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let name = "Organization Example";
    let auth_user = support::create_auth_user(role, None, &database);

    FeeSchedule::create(
        Uuid::nil(),
        "Zero fees".to_string(),
        vec![NewFeeScheduleRange {
            min_price_in_cents: 0,
            client_fee_in_cents: 0,
            company_fee_in_cents: 0,
        }],
    )
    .commit(None, database.connection.get())
    .unwrap();

    let json = Json(NewOrganizationRequest {
        name: name.to_string(),
        address: None,
        city: None,
        state: None,
        postal_code: None,
        country: None,
        phone: None,
        sendgrid_api_key: None,
        google_ga_key: None,
        facebook_pixel_key: None,
        client_event_fee_in_cents: None,
        company_event_fee_in_cents: None,
        allowed_payment_providers: None,
        cc_fee_percent: None,
        timezone: None,
        globee_api_key: None,
        max_instances_per_ticket_type: Some(11000),
        settlement_type: None,
    });

    let test_request = TestRequest::create_with_uri("/organizations");
    let response: HttpResponse = organizations::create((
        test_request.extract_state(),
        database.connection.into(),
        json,
        auth_user,
    ))
    .into();
    let body = support::unwrap_body_to_string(&response).unwrap();
    if should_test_succeed {
        assert_eq!(response.status(), StatusCode::CREATED);
        let org: Organization = serde_json::from_str(&body).unwrap();
        assert_eq!(org.name, name);
    } else {
        support::expects_unauthorized(&response);
    }
}

pub fn update(role: Roles, should_succeed: bool) {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);

    let new_name = "New Name";
    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = organization.id;
    let json = Json(OrganizationEditableAttributes {
        name: Some(new_name.to_string()),
        address: Some("address".to_string()),
        city: Some("city".to_string()),
        state: Some("state".to_string()),
        country: Some("country".to_string()),
        postal_code: Some("postal_code".to_string()),
        phone: Some("phone".to_string()),
        company_event_fee_in_cents: (Some(100)),
        client_event_fee_in_cents: None,
        sendgrid_api_key: Some(Some("sendgrid_api_key".to_string())),
        google_ga_key: Some(Some("google_ga_key".to_string())),
        facebook_pixel_key: Some(Some("facebook_pixel_key".to_string())),
        allowed_payment_providers: Some(vec![PaymentProviders::Globee]),
        timezone: Some("America/Los_Angeles".to_string()),
        cc_fee_percent: Some(5.5),
        globee_api_key: Some(Some("Itsasecret".to_string())),
        ..Default::default()
    });

    let response: HttpResponse = organizations::update((
        test_request.extract_state(),
        database.connection.into(),
        path,
        json,
        auth_user.clone(),
    ))
    .into();
    if !should_succeed {
        support::expects_unauthorized(&response);
        return;
    }

    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    let updated_organization: Organization = serde_json::from_str(&body).unwrap();
    assert_eq!(updated_organization.name, new_name);
    assert_eq!(
        updated_organization.allowed_payment_providers,
        vec![PaymentProviders::Globee]
    );
    assert_eq!(updated_organization.cc_fee_percent, 5.5);
}

pub fn update_restricted_field(restricted_field: &str, role: Roles, should_succeed: bool) {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);

    let new_name = "New Name";
    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = organization.id;

    let mut attributes = OrganizationEditableAttributes {
        name: Some(new_name.to_string()),
        ..Default::default()
    };

    match restricted_field {
        "settlement_type" => {
            attributes.settlement_type = Some(SettlementTypes::Rolling);
        }
        "max_instances_per_ticket_type" => {
            attributes.max_instances_per_ticket_type = Some(11000);
        }
        _ => panic!("Unexpected restricted field"),
    }

    let json = Json(attributes);

    let response: HttpResponse = organizations::update((
        test_request.extract_state(),
        database.connection.into(),
        path,
        json,
        auth_user.clone(),
    ))
    .into();
    if !should_succeed {
        support::expects_unauthorized(&response);
        return;
    }
    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    let updated_organization: Organization = serde_json::from_str(&body).unwrap();
    assert_eq!(updated_organization.name, new_name);

    match restricted_field {
        "settlement_type" => {
            assert_eq!(updated_organization.settlement_type, SettlementTypes::Rolling);
        }
        "max_instances_per_ticket_type" => {
            assert_eq!(updated_organization.max_instances_per_ticket_type, 11000);
        }
        _ => panic!("Unexpected restricted field"),
    }
}

pub fn remove_user(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let user2 = database.create_user().finish();
    let user3 = database.create_user().finish();

    let organization = database
        .create_organization()
        .with_member(&user2, Roles::OrgMember)
        .with_member(&user3, Roles::OrgMember)
        .finish();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);

    let test_request = TestRequest::create_with_uri_custom_params("/", vec!["id", "user_id"]);
    let mut path = Path::<OrganizationUserPathParameters>::extract(&test_request.request).unwrap();
    path.id = organization.id;
    path.user_id = user3.id;

    let response: HttpResponse =
        organizations::remove_user((database.connection.into(), path, auth_user.clone())).into();
    let count = 1;
    let body = support::unwrap_body_to_string(&response).unwrap();
    if should_test_succeed {
        assert_eq!(response.status(), StatusCode::OK);
        let removed_entries: usize = serde_json::from_str(&body).unwrap();
        assert_eq!(removed_entries, count);
    } else {
        support::expects_unauthorized(&response);
    }
}

pub fn add_user(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let user2 = database.create_user().finish();
    let organization = database.create_organization().finish();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);
    let test_request = TestRequest::create();
    let json = Json(organizations::AddUserRequest {
        user_id: user2.id,
        roles: vec![Roles::OrgMember],
        event_ids: None,
    });
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = organization.id;

    let response: HttpResponse =
        organizations::add_or_replace_user((database.connection.into(), path, json, auth_user.clone())).into();
    if should_test_succeed {
        assert_eq!(response.status(), StatusCode::CREATED);
    } else {
        support::expects_unauthorized(&response);
    }
}

pub fn add_artist(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);
    let test_request = TestRequest::create();
    let name = "Artist Example";
    let bio = "Bio";
    let website_url = "http://www.example.com";

    let json = Json(NewArtist {
        name: name.to_string(),
        bio: bio.to_string(),
        website_url: Some(website_url.to_string()),
        ..Default::default()
    });

    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = organization.id;

    let response: HttpResponse =
        organizations::add_artist((database.connection.into(), path, json, auth_user.clone())).into();
    if should_test_succeed {
        assert_eq!(response.status(), StatusCode::CREATED);
        let body = support::unwrap_body_to_string(&response).unwrap();
        let artist: Artist = serde_json::from_str(&body).unwrap();
        assert_eq!(artist.name, name);
    } else {
        support::expects_unauthorized(&response);
    }
}

pub fn list_organization_members(role: Roles, should_succeed: bool) {
    let database = TestDatabase::new();
    let user1 = database.create_user().with_last_name("User1".into()).finish();
    let user2 = database.create_user().with_last_name("User2".into()).finish();
    let organization = database
        .create_organization()
        .with_member(&user2, Roles::OrgMember)
        .finish();
    let auth_user = support::create_auth_user_from_user(&user1, role, Some(&organization), &database);

    let mut organization_members = Vec::new();
    if role != Roles::Admin {
        // create_auth_user_from_user adds the user to the organization if it is not an admin
        organization_members.push(DisplayOrganizationUser {
            user_id: Some(user1.id),
            first_name: user1.first_name,
            last_name: user1.last_name,
            email: user1.email,
            roles: vec![role],
            invite_or_member: "member".to_string(),
            invite_id: None,
        });
    }
    organization_members.push(DisplayOrganizationUser {
        user_id: Some(user2.id),
        first_name: user2.first_name,
        last_name: user2.last_name,
        email: user2.email,
        roles: vec![Roles::OrgMember],
        invite_or_member: "member".to_string(),
        invite_id: None,
    });

    let count = organization_members.len();
    let expected_data = Payload {
        data: organization_members,
        paging: Paging {
            page: 0,
            limit: 100,
            sort: "".to_string(),
            dir: SortingDir::Asc,
            total: count as u64,
            tags: HashMap::new(),
        },
    };

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = organization.id;
    let test_request = TestRequest::create_with_uri(&format!("/limits?"));
    let query_parameters = Query::<PagingParameters>::extract(&test_request.request).unwrap();
    let response = organizations::list_organization_members((
        database.connection.into(),
        path,
        query_parameters,
        auth_user.clone(),
    ));

    if !should_succeed {
        let http_response = response.err().unwrap().error_response();
        support::expects_unauthorized(&http_response);
        return;
    }

    let response = response.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(response.payload(), &expected_data);
}

pub fn show_fee_schedule(role: Roles, should_succeed: bool) {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let organization = database.create_organization().with_fees().finish();
    let fee_schedule = FeeSchedule::find(organization.fee_schedule_id, database.connection.get()).unwrap();
    let fee_schedule_ranges = fee_schedule.ranges(database.connection.get()).unwrap();

    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);

    #[derive(Serialize)]
    struct FeeScheduleWithRanges {
        id: Uuid,
        name: String,
        version: i64,
        created_at: NaiveDateTime,
        ranges: Vec<FeeScheduleRange>,
    }

    let expected_data = FeeScheduleWithRanges {
        id: fee_schedule.id,
        name: fee_schedule.name,
        version: 0,
        created_at: fee_schedule.created_at,
        ranges: fee_schedule_ranges,
    };

    let expected_json = serde_json::to_string(&expected_data).unwrap();
    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = organization.id;

    let response: HttpResponse =
        organizations::show_fee_schedule((database.connection.into(), path, auth_user.clone())).into();

    if !should_succeed {
        support::expects_unauthorized(&response);
        return;
    }
    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(body, expected_json.to_string());
}

pub fn add_fee_schedule(role: Roles, should_succeed: bool) {
    let database = TestDatabase::new();
    let organization = database.create_organization().finish();
    let auth_user = support::create_auth_user(role, Some(&organization), &database);

    let json = Json(NewFeeScheduleRequest {
        name: "Fees".to_string(),
        ranges: vec![
            NewFeeScheduleRange {
                min_price_in_cents: 20,
                company_fee_in_cents: 4,
                client_fee_in_cents: 6,
            },
            NewFeeScheduleRange {
                min_price_in_cents: 1000,
                company_fee_in_cents: 40,
                client_fee_in_cents: 60,
            },
        ],
    });
    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = organization.id;

    let response: HttpResponse =
        organizations::add_fee_schedule((database.connection.into(), path, json, auth_user)).into();

    if !should_succeed {
        support::expects_unauthorized(&response);
        return;
    }
    assert_eq!(response.status(), StatusCode::CREATED);
    let body = support::unwrap_body_to_string(&response).unwrap();
    let result: FeeScheduleWithRanges = serde_json::from_str(&body).unwrap();
    assert_eq!(result.name, "Fees".to_string());
}
