use crate::jwt::{encode, Header, Validation};
use crate::support;
use crate::support::database::TestDatabase;
use crate::support::test_request::TestRequest;
use actix_web::{http::StatusCode, HttpResponse};
use api::auth::TokenResponse;
use api::controllers::auth;
use api::controllers::auth::{LoginRequest, RefreshRequest};
use api::extractors::*;
use api::models::*;
use chrono::Duration;
use db::models::TokenIssuer;
use db::prelude::{AccessToken, Scopes};
use serde_json;
use uuid::Uuid;

#[actix_rt::test]
async fn token() {
    let database = TestDatabase::new();
    let email = "fake@localhost";
    let password = "strong_password";
    let user = database
        .create_user()
        .with_email(email.to_string())
        .with_password(password.to_string())
        .finish();

    let test_request = TestRequest::create();
    let state = test_request.extract_state().await;
    let json = Json(LoginRequest::new("fake@localhost", "strong_password"));

    let response: TokenResponse = auth::token((
        test_request.request,
        database.connection.into(),
        json,
        RequestInfo { user_agent: None },
    ))
    .await
    .unwrap();

    let access_token = state.config.token_issuer.decode(&response.access_token).unwrap();

    let mut validation = Validation::default();
    validation.validate_exp = false;
    let refresh_token = state.config.token_issuer.decode(&response.refresh_token).unwrap();

    assert_eq!(access_token.claims.get_id().unwrap(), user.id);
    assert_eq!(refresh_token.claims.get_id().unwrap(), user.id);
}

#[actix_rt::test]
async fn token_invalid_email() {
    let database = TestDatabase::new();
    database.create_user().finish();

    let test_request = TestRequest::create();
    let json = Json(LoginRequest::new("incorrect@localhost", "strong_password"));

    let response = auth::token((
        test_request.request,
        database.connection.into(),
        json,
        RequestInfo { user_agent: None },
    ))
    .await;

    assert!(response.is_err());
    assert_eq!("Email or password incorrect", response.err().unwrap().to_string());
}

#[actix_rt::test]
async fn token_incorrect_password() {
    let database = TestDatabase::new();
    let user = database.create_user().with_email("fake@localhost".to_string()).finish();

    let test_request = TestRequest::create();
    let json = Json(LoginRequest::new(&user.email.unwrap(), "incorrect"));

    let response = auth::token((
        test_request.request,
        database.connection.into(),
        json,
        RequestInfo { user_agent: None },
    ))
    .await;

    assert!(response.is_err());
    assert_eq!("Email or password incorrect", response.err().unwrap().to_string());
}

#[actix_rt::test]
async fn token_refresh() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();

    let test_request = TestRequest::create();
    let state = test_request.extract_state().await;
    let token_issuer = state.config.token_issuer.clone();
    let refresh_token = token_issuer
        .issue_with_limited_scopes(user.id, vec![Scopes::TokenRefresh], Duration::minutes(30))
        .unwrap();
    let json = Json(RefreshRequest::new(&refresh_token));

    let response: HttpResponse = auth::token_refresh((state, database.connection.into(), json))
        .await
        .into();

    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    let response: TokenResponse = serde_json::from_str(&body).unwrap();
    let access_token = token_issuer.decode(&response.access_token).unwrap();
    assert_eq!(access_token.claims.get_id().unwrap(), user.id);
}

#[actix_rt::test]
async fn token_refresh_invalid_refresh_token_secret() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();

    let test_request = TestRequest::create();
    let state = test_request.extract_state().await;
    let refresh_token_claims =
        AccessToken::new_limited_scope(user.id, "iss".to_string(), 30, vec![Scopes::TokenRefresh]);
    let refresh_token = encode(&Header::default(), &refresh_token_claims, b"incorrect-secret").unwrap();

    let json = Json(RefreshRequest::new(&refresh_token));

    let response: HttpResponse = auth::token_refresh((state, database.connection.into(), json))
        .await
        .into();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(body, json!({"error": "Invalid token"}).to_string());
}

#[actix_rt::test]
async fn token_refresh_invalid_refresh_token() {
    let database = TestDatabase::new();
    database.create_user().finish();

    let test_request = TestRequest::create();

    let state = test_request.extract_state().await;
    let json = Json(RefreshRequest::new(&"not.a.real.token"));

    let response: HttpResponse = auth::token_refresh((state, database.connection.into(), json))
        .await
        .into();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(body, json!({"error": "Invalid token"}).to_string());
}

#[actix_rt::test]
async fn token_refresh_user_does_not_exist() {
    let database = TestDatabase::new();

    let user = database.create_user().finish();
    let test_request = TestRequest::create();

    let state = test_request.extract_state().await;
    let mut refresh_token_claims =
        AccessToken::new_limited_scope(user.id, "iss".to_string(), 30, vec![Scopes::TokenRefresh]);
    refresh_token_claims.sub = Uuid::new_v4().to_string();

    let refresh_token = state.config.token_issuer.encode(&refresh_token_claims).unwrap();
    let json = Json(RefreshRequest::new(&refresh_token));

    let response: HttpResponse = auth::token_refresh((state, database.connection.into(), json))
        .await
        .into();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[actix_rt::test]
async fn token_refresh_password_reset_since_issued() {
    let database = TestDatabase::new();

    let user = database.create_user().finish();
    let password_modified_timestamp = user.password_modified_at.timestamp() as u64;

    let test_request = TestRequest::create();

    let state = test_request.extract_state().await;
    let mut refresh_token_claims =
        AccessToken::new_limited_scope(user.id, "iss".to_string(), 30, vec![Scopes::TokenRefresh]);

    // Issued a second after the latest password
    refresh_token_claims.issued = password_modified_timestamp - 1;
    let refresh_token = state.config.token_issuer.encode(&refresh_token_claims).unwrap();

    let json = Json(RefreshRequest::new(&refresh_token));

    let response: HttpResponse = auth::token_refresh((state, database.connection.into(), json))
        .await
        .into();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(body, json!({"error": "Token no longer valid"}).to_string());
}

#[actix_rt::test]
async fn token_refreshed_after_password_change() {
    let database = TestDatabase::new();

    let user = database.create_user().finish();
    let password_modified_timestamp = user.password_modified_at.timestamp() as u64;

    let test_request = TestRequest::create();

    let state = test_request.extract_state().await;
    let mut refresh_token_claims =
        AccessToken::new_limited_scope(user.id, "iss".to_string(), 30, vec![Scopes::TokenRefresh]);

    // Issued a second after the latest password
    refresh_token_claims.issued = password_modified_timestamp + 1;
    let token_issuer = state.config.token_issuer.clone();
    let refresh_token = token_issuer.encode(&refresh_token_claims).unwrap();
    let json = Json(RefreshRequest::new(&refresh_token));

    let response: HttpResponse = auth::token_refresh((state, database.connection.into(), json))
        .await
        .into();

    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    let response: TokenResponse = serde_json::from_str(&body).unwrap();
    let access_token = token_issuer.decode(&response.access_token).unwrap();

    assert_eq!(access_token.claims.get_id().unwrap(), user.id);
}
