use crate::auth::user::User as AuthUser;
use crate::database::Connection;
use crate::errors::ApiError;
use crate::extractors::Json;
use crate::server::AppState;
use crate::SITE_NAME;
use actix_web::{web::Data, HttpResponse};
use chrono::Duration;
use db::prelude::*;
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

pub async fn create(
    (state, connection, auth_user, data): (Data<AppState>, Connection, AuthUser, Json<SendDownloadLinkRequest>),
) -> Result<HttpResponse, ApiError> {
    let conn = connection.get();
    let user = auth_user.user;
    let token = user.create_magic_link_token(
        state.service_locator.token_issuer(),
        Duration::minutes(120),
        false,
        conn,
    )?;
    let linker = state.service_locator.create_deep_linker()?;
    let mut link_data = HashMap::new();
    link_data.insert("refresh_token".to_string(), json!(&token));
    let link = linker.create_with_custom_data(
        &format!(
            "{}?refresh_token={}",
            &state.config.front_end_url,
            &token.unwrap_or("".to_string())
        ),
        link_data,
    )?;
    Communication::new(
        CommunicationType::Sms,
        format!(
            "Hey {}, here's your link to download {} and view your tickets: {}",
            &user.full_name(),
            SITE_NAME,
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

pub async fn resend(
    (state, connection, data): (Data<AppState>, Connection, Json<ResendDownloadLinkRequest>),
) -> Result<HttpResponse, ApiError> {
    let conn = connection.get();

    let user = User::find(data.user_id, conn)?;

    let token =
        user.create_magic_link_token(state.service_locator.token_issuer(), Duration::minutes(120), true, conn)?;
    let linker = state.service_locator.create_deep_linker()?;
    let mut link_data = HashMap::new();
    link_data.insert("refresh_token".to_string(), json!(&token));
    let link = linker.create_with_custom_data(
        &format!(
            "{}/send-download-link?refresh_token={}",
            &state.config.front_end_url,
            &token.unwrap_or("".to_string())
        ),
        link_data,
    )?;
    if user.email.is_none() {
        // No action...
        return Ok(HttpResponse::Ok().finish());
    }
    let mut extra_data = HashMap::new();
    extra_data.insert("download_app_link".to_string(), json!(link));

    Communication::new(
        CommunicationType::EmailTemplate,
        "Your link has arrived!".to_string(),
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
