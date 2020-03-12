use crate::auth::{claims::RefreshToken, TokenResponse};
use crate::db::Connection;
use crate::errors::*;
use crate::extractors::*;
use crate::helpers::application;
use crate::jwt::{decode, Validation};
use crate::models::*;
use crate::server::{AppState, GetAppState};
use crate::utils::google_recaptcha;
use actix_web::{web::Data, HttpRequest, HttpResponse};
use bigneon_db::prelude::*;
use log::Level::Info;
use std::collections::HashMap;

#[derive(Deserialize)]
pub struct LoginRequest {
    email: String,
    password: String,
    #[serde(rename = "g-recaptcha-response")]
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    captcha_response: Option<String>,
}

#[derive(Deserialize)]
pub struct RefreshRequest {
    refresh_token: String,
}

impl LoginRequest {
    pub fn new(email: &str, password: &str) -> Self {
        LoginRequest {
            email: String::from(email),
            password: String::from(password),
            captcha_response: None,
        }
    }
}

impl RefreshRequest {
    pub fn new(refresh_token: &str) -> Self {
        RefreshRequest {
            refresh_token: String::from(refresh_token),
        }
    }
}

pub async fn token(
    (http_request, connection, login_request, request_info): (HttpRequest, Connection, Json<LoginRequest>, RequestInfo),
) -> Result<TokenResponse, BigNeonError> {
    let state = http_request.state();
    let connection_info = http_request.connection_info();
    let remote_ip = connection_info.remote();
    let mut login_log_data = HashMap::new();
    login_log_data.insert("email", login_request.email.clone().into());

    if let Some(ref google_recaptcha_secret_key) = state.config.google_recaptcha_secret_key {
        match login_request.captcha_response {
            Some(ref captcha_response) => {
                let captcha_response = google_recaptcha::verify_response(
                    google_recaptcha_secret_key,
                    captcha_response.to_owned(),
                    remote_ip,
                )?;
                if !captcha_response.success {
                    return application::unauthorized_with_message("Captcha value invalid", None, Some(login_log_data));
                }
            }
            None => {
                return application::unauthorized_with_message("Captcha required", None, Some(login_log_data));
            }
        }
    }

    // Generic messaging to prevent exposing user is member of system
    let login_failure_messaging = "Email or password incorrect";

    let user = match User::find_by_email(&login_request.email, false, connection.get()).optional() {
        Ok(u) => match u {
            Some(usr) => usr,
            None => return application::unauthorized_with_message(login_failure_messaging, None, Some(login_log_data)),
        },
        Err(_e) => {
            return application::unauthorized_with_message(login_failure_messaging, None, Some(login_log_data));
        }
    };

    if !user.check_password(&login_request.password) {
        return application::unauthorized_with_message(login_failure_messaging, None, Some(login_log_data));
    }

    user.login_domain_event(json!(request_info), connection.get())?;
    jlog!(Info, "User logged in via email and password", {"id": user.id, "email": user.email.clone()});
    let response = TokenResponse::create_from_user(
        &state.config.token_secret,
        &state.config.token_issuer,
        &state.config.jwt_expiry_time,
        &user,
    )?;
    Ok(response)
}

pub async fn token_refresh(
    (state, connection, refresh_request): (Data<AppState>, Connection, Json<RefreshRequest>),
) -> Result<HttpResponse, BigNeonError> {
    let mut validation = Validation::default();
    validation.validate_exp = false;
    let token = decode::<RefreshToken>(
        &refresh_request.refresh_token,
        state.config.token_secret.as_bytes(),
        &validation,
    )?;
    let user = User::find(token.claims.get_id()?, connection.get())?;

    // If the user changes their password invalidate all refresh tokens
    let password_modified_timestamp = user.password_modified_at.timestamp() as u64;
    if password_modified_timestamp > token.claims.issued {
        return application::unauthorized_with_message("Invalid token", None, None);
    }

    let response = TokenResponse::create_from_refresh_token(
        &state.config.token_secret,
        &state.config.token_issuer,
        &state.config.jwt_expiry_time,
        &user.id,
        &refresh_request.refresh_token,
    )?;

    Ok(HttpResponse::Ok().json(response))
}
