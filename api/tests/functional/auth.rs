use crate::jwt::{decode, encode, Header, Validation};
use crate::support;
use crate::support::database::TestDatabase;
use crate::support::test_request::TestRequest;
use actix_web::{FromRequest, http::StatusCode, HttpResponse};
use bigneon_api::auth::{claims::AccessToken, claims::RefreshToken, TokenResponse};
use bigneon_api::controllers::auth;
use bigneon_api::controllers::auth::{LoginRequest, RefreshRequest};
use bigneon_api::extractors::*;
use bigneon_api::models::*;
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
    .unwrap();

    let access_token = decode::<AccessToken>(
        &response.access_token,
        state.config.token_secret.as_bytes(),
        &Validation::default(),
    )
    .unwrap();

    let mut validation = Validation::default();
    validation.validate_exp = false;
    let refresh_token = decode::<RefreshToken>(
        &response.refresh_token,
        state.config.token_secret.as_bytes(),
        &validation,
    )
    .unwrap();

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
    ));

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
    ));

    assert!(response.is_err());
    assert_eq!("Email or password incorrect", response.err().unwrap().to_string());
}

#[actix_rt::test]
async fn token_refresh() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();

    let test_request = TestRequest::create();
    let state = test_request.extract_state().await;
    let token_secret = &state.config.token_secret.clone();
    let refresh_token_claims = RefreshToken::new(&user.id, state.config.token_issuer.clone());
    let refresh_token = encode(&Header::default(), &refresh_token_claims, token_secret.as_bytes()).unwrap();

    let json = Json(RefreshRequest::new(&refresh_token));

    let response: HttpResponse = auth::token_refresh((state, database.connection.into(), json)).await.into();
    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    let response: TokenResponse = serde_json::from_str(&body).unwrap();

    let access_token =
        decode::<AccessToken>(&response.access_token, token_secret.as_bytes(), &Validation::default()).unwrap();
    assert_eq!(response.refresh_token, refresh_token);
    assert_eq!(access_token.claims.get_id().unwrap(), user.id);
}

#[actix_rt::test]
async fn token_refresh_invalid_refresh_token_secret() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();

    let test_request = TestRequest::create();
    let state = test_request.extract_state().await;
    let refresh_token_claims = RefreshToken::new(&user.id, state.config.token_issuer.clone());
    let refresh_token = encode(&Header::default(), &refresh_token_claims, b"incorrect-secret").unwrap();

    let json = Json(RefreshRequest::new(&refresh_token));

    let response: HttpResponse = auth::token_refresh((state, database.connection.into(), json)).await.into();

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

    let response: HttpResponse = auth::token_refresh((state, database.connection.into(), json)).await.into();

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
    let mut refresh_token_claims = RefreshToken::new(&user.id, state.config.token_issuer.clone());
    refresh_token_claims.sub = Uuid::new_v4().to_string();

    let refresh_token = encode(
        &Header::default(),
        &refresh_token_claims,
        state.config.token_secret.as_bytes(),
    )
    .unwrap();
    let json = Json(RefreshRequest::new(&refresh_token));

    let response: HttpResponse = auth::token_refresh((state, database.connection.into(), json)).await.into();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[actix_rt::test]
async fn token_refresh_password_reset_since_issued() {
    let database = TestDatabase::new();

    let user = database.create_user().finish();
    let password_modified_timestamp = user.password_modified_at.timestamp() as u64;

    let test_request = TestRequest::create();

    let state = test_request.extract_state().await;
    let mut refresh_token_claims = RefreshToken::new(&user.id, state.config.token_issuer.clone());

    // Issued a second prior to the latest password
    refresh_token_claims.issued = password_modified_timestamp - 1;
    let refresh_token = encode(
        &Header::default(),
        &refresh_token_claims,
        state.config.token_secret.as_bytes(),
    )
    .unwrap();
    let json = Json(RefreshRequest::new(&refresh_token));

    let response: HttpResponse = auth::token_refresh((state, database.connection.into(), json)).await.into();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(body, json!({"error": "Invalid token"}).to_string());
}

#[actix_rt::test]
async fn token_refreshed_after_password_change() {
    let database = TestDatabase::new();

    let user = database.create_user().finish();
    let password_modified_timestamp = user.password_modified_at.timestamp() as u64;

    let test_request = TestRequest::create();

    let state = test_request.extract_state().await;
    let token_secret = &state.config.token_secret.clone();
    let mut refresh_token_claims = RefreshToken::new(&user.id, state.config.token_issuer.clone());

    // Issued a second after the latest password
    refresh_token_claims.issued = password_modified_timestamp + 1;
    let refresh_token = encode(&Header::default(), &refresh_token_claims, token_secret.as_bytes()).unwrap();
    let json = Json(RefreshRequest::new(&refresh_token));

    let response: HttpResponse = auth::token_refresh((state, database.connection.into(), json)).await.into();

    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    let response: TokenResponse = serde_json::from_str(&body).unwrap();

    let access_token =
        decode::<AccessToken>(&response.access_token, token_secret.as_bytes(), &Validation::default()).unwrap();
    assert_eq!(response.refresh_token, refresh_token);
    assert_eq!(access_token.claims.get_id().unwrap(), user.id);
}
