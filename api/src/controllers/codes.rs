use crate::auth::user::User;
use crate::database::Connection;
use crate::errors::{ApiError, ApplicationError};
use crate::extractors::*;
use crate::helpers::application;
use crate::models::PathParameters;
use crate::server::AppState;
use actix_web::{
    web::{Data, Path},
    HttpResponse,
};
use chrono::prelude::*;
use db::dev::times;
use db::models::*;
use serde_with::rust::double_option;
use uuid::Uuid;

#[derive(Deserialize, Serialize)]
pub struct CreateCodeRequest {
    pub name: String,
    pub redemption_codes: Vec<String>,
    pub code_type: CodeTypes,
    pub max_uses: u32,
    pub discount_in_cents: Option<u32>,
    pub discount_as_percentage: Option<u32>,
    pub start_date: Option<NaiveDateTime>,
    pub end_date: Option<NaiveDateTime>,
    pub max_tickets_per_user: Option<u32>,
    pub ticket_type_ids: Vec<Uuid>,
}

#[derive(Clone, Default, Deserialize, Serialize)]
pub struct UpdateCodeRequest {
    pub name: Option<String>,
    pub redemption_codes: Option<Vec<String>>,
    pub max_uses: Option<i64>,
    #[serde(default, deserialize_with = "double_option::deserialize")]
    pub discount_in_cents: Option<Option<u32>>,
    #[serde(default, deserialize_with = "double_option::deserialize")]
    pub discount_as_percentage: Option<Option<u32>>,
    #[serde(default, deserialize_with = "double_option::deserialize")]
    pub start_date: Option<Option<NaiveDateTime>>,
    #[serde(default, deserialize_with = "double_option::deserialize")]
    pub end_date: Option<Option<NaiveDateTime>>,
    #[serde(default, deserialize_with = "double_option::deserialize")]
    pub max_tickets_per_user: Option<Option<u32>>,
    pub ticket_type_ids: Option<Vec<Uuid>>,
}

impl From<UpdateCodeRequest> for UpdateCodeAttributes {
    fn from(attributes: UpdateCodeRequest) -> Self {
        let start_date = match attributes.start_date {
            None => None,
            Some(s) => match s {
                None => Some(times::zero()),
                Some(v) => Some(v),
            },
        };

        let end_date = match attributes.end_date {
            None => None,
            Some(s) => match s {
                None => Some(times::infinity()),
                Some(v) => Some(v),
            },
        };

        let redemption_code = if let Some(s) = attributes.redemption_codes {
            Some(s.concat())
        } else {
            None
        };

        UpdateCodeAttributes {
            name: attributes.name,
            redemption_code,
            max_uses: attributes.max_uses.map(|m| m as i64),
            discount_in_cents: attributes.discount_in_cents.map(|d| d.map(|d2| d2 as i64)),
            discount_as_percentage: attributes.discount_as_percentage.map(|d| d.map(|d2| d2 as i64)),
            start_date,
            end_date,
            max_tickets_per_user: attributes.max_tickets_per_user.map(|m| m.map(|m2| m2 as i64)),
        }
    }
}

pub async fn show((conn, path, user): (Connection, Path<PathParameters>, User)) -> Result<HttpResponse, ApiError> {
    let conn = conn.get();
    let code = Code::find(path.id, conn)?;
    user.requires_scope_for_organization_event(Scopes::CodeRead, &code.organization(conn)?, &code.event(conn)?, conn)?;

    Ok(HttpResponse::Ok().json(code.for_display(conn)?))
}

pub async fn link(
    (conn, path, user, state): (Connection, Path<PathParameters>, User, Data<AppState>),
) -> Result<HttpResponse, ApiError> {
    let conn = conn.get();
    let code = Code::find(path.id, conn)?;
    let event = code.event(conn)?;
    user.requires_scope_for_organization_event(Scopes::CodeRead, &code.organization(conn)?, &event, conn)?;
    let linker = state.service_locator.create_deep_linker()?;
    let raw_url = format!(
        "{}/events/{}/tickets?code={}",
        &state.config.front_end_url,
        event.slug(conn)?,
        &code.redemption_code
    );
    let link = match linker.create_deep_link_with_alias(&raw_url, &code.redemption_code) {
        Ok(l) => l,
        Err(_) => {
            // Alias might not be unique, create without
            linker.create_deep_link_with_fallback(&raw_url)
        }
    };
    Ok(HttpResponse::Ok().json(json!({ "link": link })))
}

pub async fn create(
    (conn, req, path, user): (Connection, Json<CreateCodeRequest>, Path<PathParameters>, User),
) -> Result<HttpResponse, ApiError> {
    let conn = conn.get();
    let event = Event::find(path.id, conn)?;
    user.requires_scope_for_organization_event(Scopes::CodeWrite, &event.organization(conn)?, &event, conn)?;

    if req.redemption_codes.len() != 1 {
        return application::unprocessable("Only one code allowed at this time");
    }

    let code = Code::create(
        req.name.clone(),
        path.id,
        req.code_type,
        req.redemption_codes
            .iter()
            .map(|s| s.to_uppercase())
            .next()
            .ok_or_else(|| ApplicationError::new("Code is required".to_string()))?
            .to_string(),
        req.max_uses,
        req.discount_in_cents,
        req.discount_as_percentage,
        req.start_date.unwrap_or(times::zero()),
        req.end_date.unwrap_or(times::infinity()),
        req.max_tickets_per_user,
    )
    .commit(Some(user.id()), conn)?;

    code.update_ticket_types(req.ticket_type_ids.clone(), conn)?;
    application::created(json!(code.for_display(conn)?))
}

pub async fn update(
    (conn, req, path, user): (Connection, Json<UpdateCodeRequest>, Path<PathParameters>, User),
) -> Result<HttpResponse, ApiError> {
    let conn = conn.get();

    let code = Code::find(path.id, conn)?;
    user.requires_scope_for_organization_event(Scopes::CodeWrite, &code.organization(conn)?, &code.event(conn)?, conn)?;

    let code = code.update(req.clone().into(), Some(user.id()), conn)?;

    if let Some(ref ticket_type_ids) = req.ticket_type_ids {
        code.update_ticket_types(ticket_type_ids.clone(), conn)?;
    }

    Ok(HttpResponse::Ok().json(code.for_display(conn)?))
}

pub async fn destroy((conn, path, user): (Connection, Path<PathParameters>, User)) -> Result<HttpResponse, ApiError> {
    let conn = conn.get();
    let code = Code::find(path.id, conn)?;
    user.requires_scope_for_organization_event(Scopes::CodeWrite, &code.organization(conn)?, &code.event(conn)?, conn)?;

    code.destroy(Some(user.id()), &*conn)?;
    Ok(HttpResponse::Ok().json(json!({})))
}
