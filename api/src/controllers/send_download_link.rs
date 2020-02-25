use actix_web::{HttpResponse, State};
use auth::user::User as AuthUser;
use bigneon_db::prelude::*;
use chrono::Duration;
use db::Connection;
use errors::BigNeonError;
use extractors::Json;
use server::AppState;
use std::collections::HashMap;

#[derive(Deserialize)]
pub struct SendDownloadLinkRequest {
    phone: String,
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
        None,
        CommAddress::from(data.into_inner().phone),
        None,
        None,
        Some(vec!["download"]),
        None,
    )
    .queue(conn)?;

    Ok(HttpResponse::Created().finish())
}
