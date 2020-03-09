use actix_web::{http::StatusCode, HttpResponse, web::{Path, Query, Data}};
use crate::auth::user::User;
use bigneon_db::models::*;
use chrono::prelude::*;
use crate::db::Connection;
use crate::errors::BigNeonError;
use crate::extractors::*;
use crate::helpers::application;
use log::Level::Warn;
use crate::models::{PathParameters, WebPayload};
use serde_with::rust::double_option;
use crate::server::AppState;
use std::error::Error;
use uuid::Uuid;

#[derive(Deserialize, Serialize)]
pub struct CreateHoldRequest {
    pub name: String,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub redemption_code: Option<String>,
    pub discount_in_cents: Option<u32>,
    pub hold_type: HoldTypes,
    pub quantity: u32,
    pub ticket_type_id: Uuid,
    pub end_at: Option<NaiveDateTime>,
    pub max_per_user: Option<u32>,
}

#[derive(Default, Deserialize, Serialize)]
pub struct UpdateHoldRequest {
    pub name: Option<String>,
    #[serde(default, deserialize_with = "double_option_deserialize_unless_blank")]
    pub redemption_code: Option<Option<String>>,
    pub hold_type: Option<HoldTypes>,
    pub quantity: Option<u32>,
    #[serde(default, deserialize_with = "double_option::deserialize")]
    pub discount_in_cents: Option<Option<i64>>,
    #[serde(default, deserialize_with = "double_option_deserialize_unless_blank")]
    pub email: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option_deserialize_unless_blank")]
    pub phone: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option::deserialize")]
    pub end_at: Option<Option<NaiveDateTime>>,
    #[serde(default, deserialize_with = "double_option::deserialize")]
    pub max_per_user: Option<Option<i64>>,
}

impl From<UpdateHoldRequest> for UpdateHoldAttributes {
    fn from(attributes: UpdateHoldRequest) -> Self {
        UpdateHoldAttributes {
            name: attributes.name,
            hold_type: attributes.hold_type.and_then(|hold_type| Some(hold_type)),
            discount_in_cents: attributes.discount_in_cents,
            email: attributes.email,
            phone: attributes.phone,
            end_at: attributes.end_at,
            max_per_user: attributes.max_per_user,
            redemption_code: attributes.redemption_code,
        }
    }
}

// add update fields in here as well

pub fn create(
    (conn, req, path, user): (Connection, Json<CreateHoldRequest>, Path<PathParameters>, User),
) -> Result<HttpResponse, BigNeonError> {
    let conn = conn.get();
    let event = Event::find(path.id, conn)?;
    user.requires_scope_for_organization_event(Scopes::HoldWrite, &event.organization(conn)?, &event, conn)?;

    let hold = Hold::create_hold(
        req.name.clone(),
        path.id,
        req.redemption_code.clone(),
        req.discount_in_cents,
        req.end_at,
        req.max_per_user,
        req.hold_type,
        req.ticket_type_id,
    )
    .commit(Some(user.id()), conn)?;

    hold.set_quantity(Some(user.id()), req.quantity, conn)?;

    #[derive(Serialize)]
    struct R {
        pub id: Uuid,
        pub name: String,
        pub event_id: Uuid,
        pub redemption_code: Option<String>,
        pub discount_in_cents: Option<i64>,
        pub end_at: Option<NaiveDateTime>,
        pub max_per_user: Option<i64>,
        pub hold_type: HoldTypes,
        pub ticket_type_id: Uuid,
        pub available: u32,
        pub quantity: u32,
    }

    let (quantity, available) = hold.quantity(conn)?;

    let r = R {
        id: hold.id,
        name: hold.name,
        event_id: hold.event_id,
        redemption_code: hold.redemption_code,
        discount_in_cents: hold.discount_in_cents,
        end_at: hold.end_at,
        max_per_user: hold.max_per_user,
        hold_type: hold.hold_type,
        ticket_type_id: hold.ticket_type_id,
        available,
        quantity,
    };

    application::created(json!(r))
}

pub fn update(
    (conn, req, path, user): (Connection, Json<UpdateHoldRequest>, Path<PathParameters>, User),
) -> Result<HttpResponse, BigNeonError> {
    let conn = conn.get();

    let hold = Hold::find(path.id, conn)?;
    user.requires_scope_for_organization_event(Scopes::HoldWrite, &hold.organization(conn)?, &hold.event(conn)?, conn)?;
    let quantity = req.quantity;
    let hold = hold.update(req.into_inner().into(), conn)?;
    if let Some(quantity) = quantity {
        hold.set_quantity(Some(user.id()), quantity, conn)?;
    }

    Ok(HttpResponse::Ok().json(hold))
}

pub fn show((conn, path, user): (Connection, Path<PathParameters>, User)) -> Result<HttpResponse, BigNeonError> {
    let conn = conn.get();
    let hold = Hold::find(path.id, conn)?;
    user.requires_scope_for_organization_event(Scopes::HoldRead, &hold.organization(conn)?, &hold.event(conn)?, conn)?;

    #[derive(Serialize)]
    struct R {
        pub id: Uuid,
        pub name: String,
        pub event_id: Uuid,
        pub redemption_code: Option<String>,
        pub discount_in_cents: Option<i64>,
        pub end_at: Option<NaiveDateTime>,
        pub max_per_user: Option<i64>,
        pub hold_type: HoldTypes,
        pub ticket_type_id: Uuid,
        pub available: u32,
        pub quantity: u32,
    }

    let (quantity, available) = hold.quantity(conn)?;

    let r = R {
        id: hold.id,
        name: hold.name,
        event_id: hold.event_id,
        redemption_code: hold.redemption_code,
        discount_in_cents: hold.discount_in_cents,
        end_at: hold.end_at,
        max_per_user: hold.max_per_user,
        hold_type: hold.hold_type,
        ticket_type_id: hold.ticket_type_id,
        available,
        quantity,
    };

    Ok(HttpResponse::Ok().json(r))
}

pub fn link(
    (conn, path, user, state): (Connection, Path<PathParameters>, User, Data<AppState>),
) -> Result<HttpResponse, BigNeonError> {
    let conn = conn.get();
    let hold = Hold::find(path.id, conn)?;
    let event = hold.event(conn)?;
    user.requires_scope_for_organization_event(Scopes::HoldRead, &hold.organization(conn)?, &event, conn)?;
    if hold.redemption_code.is_none() {
        return application::not_found();
    }

    let linker = state.service_locator.create_deep_linker()?;
    let raw_url = format!(
        "{}/{}/tickets?code={}",
        &state.config.front_end_url,
        event.slug(conn)?,
        hold.redemption_code.as_ref().unwrap()
    );
    let link = match linker.create_deep_link_with_alias(&raw_url, hold.redemption_code.as_ref().unwrap()) {
        Ok(l) => l,
        Err(e) => {
            jlog!(Warn, "Error when creating an aliased link",
            {"error": e.description(), "raw_url": &raw_url, "alias": hold.redemption_code.as_ref().unwrap()});
            // Alias might not be unique, create without
            linker.create_deep_link_with_fallback(&raw_url)
        }
    };
    Ok(HttpResponse::Ok().json(json!({ "link": link })))
}

#[derive(Deserialize)]
pub struct SetQuantityRequest {
    pub quantity: u32,
}

#[derive(Deserialize, Serialize)]
pub struct SplitHoldRequest {
    pub name: String,
    pub redemption_code: String,
    pub discount_in_cents: Option<u32>,
    pub hold_type: HoldTypes,
    pub quantity: u32,
    pub end_at: Option<NaiveDateTime>,
    pub max_per_user: Option<u32>,
    pub child: Option<bool>,
    pub email: Option<String>,
    pub phone: Option<String>,
}

pub fn children(
    (conn, path, query_parameters, user): (Connection, Path<PathParameters>, Query<PagingParameters>, User),
) -> Result<WebPayload<DisplayHold>, BigNeonError> {
    let conn = conn.get();
    let hold = Hold::find(path.id, conn)?;
    user.requires_scope_for_organization(Scopes::HoldRead, &hold.organization(conn)?, conn)?;

    let holds = Hold::find_by_parent_id(path.id, None, query_parameters.page(), query_parameters.limit(), conn)?;

    let mut list = Vec::<DisplayHold>::new();
    for hold in holds.data {
        let r = hold.into_display(conn)?;

        list.push(r);
    }

    let payload = Payload::new(list, holds.paging);

    Ok(WebPayload::new(StatusCode::OK, payload))
}

pub fn split(
    (conn, req, path, user): (Connection, Json<SplitHoldRequest>, Path<PathParameters>, User),
) -> Result<HttpResponse, BigNeonError> {
    let conn = conn.get();
    let hold = Hold::find(path.id, conn)?;
    user.requires_scope_for_organization_event(Scopes::HoldWrite, &hold.organization(conn)?, &hold.event(conn)?, conn)?;

    let new_hold = hold.split(
        Some(user.id()),
        req.name.clone(),
        req.email.clone(),
        req.phone.clone(),
        req.redemption_code.clone(),
        req.quantity,
        req.discount_in_cents,
        req.hold_type,
        req.end_at,
        req.max_per_user,
        req.child.unwrap_or(false),
        conn,
    )?;
    Ok(HttpResponse::Created().json(new_hold))
}

pub fn destroy((conn, path, user): (Connection, Path<PathParameters>, User)) -> Result<HttpResponse, BigNeonError> {
    let conn = conn.get();
    let hold = Hold::find(path.id, conn)?;
    user.requires_scope_for_organization_event(Scopes::HoldWrite, &hold.organization(conn)?, &hold.event(conn)?, conn)?;
    hold.destroy(Some(user.id()), conn)?;
    Ok(HttpResponse::Ok().finish())
}
