use actix_web::{HttpResponse, State};
use auth::user::User as AuthUser;
use bigneon_db::prelude::*;
use chrono::Duration;
use db::Connection;
use errors::BigNeonError;
use extractors::{Json, OptionalUser};
use server::AppState;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct SendDownloadLinkRequest {
    phone: String,
}

#[derive(Deserialize)]
pub struct ResendDownloadLinkRequest {
    user_id: Uuid,
}

pub fn create(
    (state, connection, auth_user, data): (State<AppState>, Connection, AuthUser, Json<SendDownloadLinkRequest>),
) -> Result<HttpResponse, BigNeonError> {
    let conn = connection.get();
    let user = auth_user.user;
    let token = user.create_magic_link_token(state.service_locator.token_issuer(), Duration::minutes(120))?;
    let linker = state.service_locator.create_deep_linker()?;
    let mut link_data = HashMap::new();
    link_data.insert("refresh_token".to_string(), json!(&token));
    let link = linker.create_with_custom_data(
        &format!("{}?refresh_token={}", &state.config.front_end_url, &token),
        link_data,
    )?;
    Communication::new(
        CommunicationType::Sms,
        format!(
            "Hey {}, here's your link to download Big Neon and view your tickets: {}",
            &user.full_name(),
            &link
        ),
        None,
        Some(CommAddress::from(
            state.config.communication_default_source_phone.clone(),
        )),
        CommAddress::from(data.into_inner().phone),
        None,
        None,
        Some(vec!["download"]),
        None,
    )
    .queue(conn)?;

    Ok(HttpResponse::Created().finish())
}

pub fn resend(
    (state, connection, auth_user, data): (
        State<AppState>,
        Connection,
        OptionalUser,
        Json<ResendDownloadLinkRequest>,
    ),
) -> Result<HttpResponse, BigNeonError> {
    let conn = connection.get();
    let user = if let Some(user) = auth_user.0 {
        user.user
    } else {
        User::find(data.user_id, conn)?
    };

    let token = user.create_magic_link_token(state.service_locator.token_issuer(), Duration::minutes(120))?;
    let linker = state.service_locator.create_deep_linker()?;
    let mut link_data = HashMap::new();
    link_data.insert("refresh_token".to_string(), json!(&token));
    let link = linker.create_with_custom_data(
        &format!(
            "/send_download_link/{}?refresh_token={}",
            &state.config.front_end_url, &token
        ),
        link_data,
    )?;
    if user.email.is_none() {
        // No action...
        return Ok(HttpResponse::Ok().finish());
    }
    let mut extra_data = HashMap::new();
    extra_data.insert("first_name".to_string(), json!(user.first_name.clone()));

    Communication::new(
        CommunicationType::EmailTemplate,
        "".to_string(),
        None,
        Some(CommAddress::from(
            state.config.communication_default_source_email.clone(),
        )),
        CommAddress::from(user.email.unwrap()),
        Some(state.config.email_templates.resend_download_link.to_string()),
        None,
        Some(vec!["download"]),
        Some(extra_data),
    )
    .queue(conn)?;

    Ok(HttpResponse::Created().finish())
}
