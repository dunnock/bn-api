use crate::support;
use crate::support::database::TestDatabase;
use crate::support::test_request::TestRequest;
use actix_web::{
    http::StatusCode,
    web::{Path, Query},
    FromRequest, HttpResponse,
};
use bigneon_api::controllers::organization_invites::{self, *};
use bigneon_api::extractors::*;
use bigneon_api::models::*;
use bigneon_db::models::*;
use chrono::Duration;
use serde_json;
use std::collections::HashMap;

pub async fn create(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);

    let email = "jeff2@tari.com";
    let _invited_user = database.create_user().with_email(email.to_string()).finish();

    let test_request = TestRequest::create();
    let state = test_request.extract_state().await;
    let json = Json(NewOrgInviteRequest {
        user_email: email.into(),
        roles: vec![Roles::OrgMember],
        event_ids: None,
    });
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = organization.id;

    let response: HttpResponse =
        organization_invites::create((state, database.connection.into(), json, path, auth_user.clone()))
            .await
            .into();
    let body = support::unwrap_body_to_string(&response).unwrap();

    if should_test_succeed {
        assert_eq!(response.status(), StatusCode::CREATED);
        let org_in: OrganizationInvite = serde_json::from_str(&body).unwrap();
        assert_eq!(org_in.organization_id, organization.id);
        assert_eq!(org_in.inviter_id, user.id);
    } else {
        support::expects_unauthorized(&response);
    }
}

pub async fn create_for_new_user(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);

    let test_request = TestRequest::create();
    let state = test_request.extract_state().await;
    let email = "jeff2@tari.com";
    let json = Json(NewOrgInviteRequest {
        user_email: email.into(),
        roles: vec![Roles::OrgMember],
        event_ids: None,
    });
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = organization.id;

    let response: HttpResponse =
        organization_invites::create((state, database.connection.into(), json, path, auth_user))
            .await
            .into();
    let body = support::unwrap_body_to_string(&response).unwrap();

    if should_test_succeed {
        assert_eq!(response.status(), StatusCode::CREATED);
        let org_in: OrganizationInvite = serde_json::from_str(&body).unwrap();
        assert_eq!(org_in.organization_id, organization.id);
        assert_eq!(org_in.inviter_id, user.id);
    } else {
        support::expects_unauthorized(&response);
    }
}

pub async fn destroy(role: Roles, should_succeed: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);

    let invite = database
        .create_organization_invite()
        .with_org(&organization)
        .with_invitee(&user)
        .finish();

    let test_request = TestRequest::create_with_uri_custom_params("/", vec!["id", "invite_id"]);
    let mut path = Path::<OrganizationInvitePathParameters>::extract(&test_request.request)
        .await
        .unwrap();
    path.id = organization.id;
    path.invite_id = invite.id;

    let response: HttpResponse = organization_invites::destroy((database.connection.clone().into(), path, auth_user))
        .await
        .into();

    if should_succeed {
        assert_eq!(response.status(), StatusCode::OK);
        let invite = OrganizationInvite::find(invite.id, connection);
        assert!(invite.is_err());
    } else {
        support::expects_unauthorized(&response);
    }
}

pub async fn index(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user1 = database.create_user().finish();
    let organization = database.create_organization().finish();
    let auth_user = support::create_auth_user_from_user(&user1, role, Some(&organization), &database);

    let user2 = database.create_user().finish();
    let user3 = database.create_user().finish();
    let user4 = database.create_user().finish();

    let mut org_invite1 = database
        .create_organization_invite()
        .with_org(&organization)
        .with_inviter(&user1)
        .with_invitee(&user2)
        .finish();

    let mut org_invite2 = database
        .create_organization_invite()
        .with_org(&organization)
        .with_inviter(&user1)
        .with_invitee(&user3)
        .finish();

    let org_invite3 = database
        .create_organization_invite()
        .with_org(&organization)
        .with_inviter(&user1)
        .with_invitee(&user4)
        .finish();

    // Decline first invite
    org_invite1.change_invite_status(0, connection).unwrap();

    // Accept second invite
    org_invite2.change_invite_status(1, connection).unwrap();

    let test_request = TestRequest::create();
    let query_parameters = Query::<PagingParameters>::extract(&test_request.request).await.unwrap();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = organization.id;

    let response =
        organization_invites::index((database.connection.clone().into(), path, query_parameters, auth_user)).await;

    let wrapped_expected_invites = Payload {
        data: vec![DisplayInvite {
            id: org_invite3.id,
            organization_name: organization.name,
            inviter_name: format!("{} {}", user4.first_name.unwrap(), user4.last_name.unwrap()).into(),
            expires_at: org_invite3.created_at + Duration::days(INVITE_EXPIRATION_PERIOD_IN_DAYS),
        }],
        paging: Paging {
            page: 0,
            limit: 100,
            sort: "".to_string(),
            dir: SortingDir::Asc,
            total: 1 as u64,
            tags: HashMap::new(),
        },
    };

    if should_test_succeed {
        let response = response.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(wrapped_expected_invites, *response.payload());
    } else {
        assert_eq!(
            response.err().unwrap().to_string(),
            "User does not have the required permissions"
        );
    }
}

pub async fn accept_invite_status_of_invite(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);
    database.create_user().finish();

    let invite = database
        .create_organization_invite()
        .with_org(&organization)
        .with_invitee(&user)
        .with_email(&user.email.clone().unwrap())
        .with_security_token(None)
        .finish();

    OrganizationInvite::find_by_token(invite.security_token.unwrap(), database.connection.get()).unwrap();

    let test_request = TestRequest::create_with_uri(
        format!(
            "/accept_invite?security_token={}",
            &invite.security_token.unwrap().to_string()
        )
        .as_str(),
    );
    let parameters = Query::<InviteResponseQuery>::extract(&test_request.request)
        .await
        .unwrap();

    let response: HttpResponse =
        organization_invites::accept_request((database.connection.into(), parameters, OptionalUser(Some(auth_user))))
            .await
            .into();
    if should_test_succeed {
        assert_eq!(response.status(), StatusCode::OK);
    } else {
        support::expects_unauthorized(&response);
    }
}
