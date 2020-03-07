use actix_web::{http::StatusCode, HttpResponse, Json};
use bigneon_api::controllers::user_invites::{self, UserInviteRequest};
use bigneon_db::models::Roles;
use crate::support;
use crate::support::database::TestDatabase;
use crate::support::test_request::TestRequest;
use uuid::Uuid;

#[test]
fn create_fails_user_exists() {
    let database = TestDatabase::new();
    let email = format!("test-{}@example.com", Uuid::new_v4());

    database.create_user().with_email(email.to_string()).finish();

    let test_request = TestRequest::create();
    let state = test_request.extract_state();
    let json = Json(UserInviteRequest {
        first_name: Some("firsty".to_string()),
        last_name: Some("lasty".to_string()),
        email: email.to_string(),
    });
    let auth_user = support::create_auth_user(Roles::Admin, None, &database);
    let response: HttpResponse = user_invites::create((state, database.connection.clone(), json, auth_user)).into();

    assert_eq!(response.status(), StatusCode::CREATED);
}

#[test]
fn create() {
    let database = TestDatabase::new();
    let email = format!("test-{}@example.com", Uuid::new_v4());

    let test_request = TestRequest::create();
    let state = test_request.extract_state();
    let json = Json(UserInviteRequest {
        first_name: Some("firsty".to_string()),
        last_name: Some("lasty".to_string()),
        email: email.to_string(),
    });
    let auth_user = support::create_auth_user(Roles::Admin, None, &database);
    let response: HttpResponse = user_invites::create((state, database.connection.clone(), json, auth_user)).into();

    assert_eq!(response.status(), StatusCode::CREATED);
}
