use actix_web::{http::StatusCode, HttpResponse, web::{Path, Query}};
use crate::auth::user::User;
use bigneon_db::models::*;
use chrono::prelude::*;
use crate::controllers::holds::UpdateHoldRequest;
use crate::db::Connection;
use crate::errors::BigNeonError;
use crate::extractors::*;
use crate::models::{PathParameters, WebPayload, WebResult};

pub fn index(
    (conn, path, query_parameters, user): (Connection, Path<PathParameters>, Query<PagingParameters>, User),
) -> Result<WebPayload<DisplayHold>, BigNeonError> {
    let conn = conn.get();
    let hold = Hold::find(path.id, conn)?;
    user.requires_scope_for_organization(Scopes::CompRead, &hold.organization(conn)?, conn)?;

    let comps = Hold::find_by_parent_id(
        path.id,
        Some(HoldTypes::Comp),
        query_parameters.page(),
        query_parameters.limit(),
        conn,
    )?;

    let mut list = Vec::<DisplayHold>::new();
    for hold in comps.data {
        if hold.deleted_at.is_some() {
            continue;
        }
        let r = hold.into_display(conn)?;

        list.push(r);
    }

    let payload = Payload::new(list, comps.paging);

    Ok(WebPayload::new(StatusCode::OK, payload))
}

pub fn show((conn, path, user): (Connection, Path<PathParameters>, User)) -> Result<HttpResponse, BigNeonError> {
    let conn = conn.get();
    let hold = Hold::find(path.id, conn)?;
    user.requires_scope_for_organization(Scopes::CompRead, &hold.organization(conn)?, conn)?;
    let comp = hold.into_display(conn)?;
    Ok(HttpResponse::Ok().json(&comp))
}

#[derive(Default, Deserialize, Serialize)]
pub struct NewCompRequest {
    pub name: String,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub email: Option<String>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub phone: Option<String>,
    pub quantity: u32,
    pub redemption_code: String,
    pub end_at: Option<NaiveDateTime>,
    pub max_per_user: Option<u32>,
}

pub fn create(
    (conn, new_comp, path, user): (Connection, Json<NewCompRequest>, Path<PathParameters>, User),
) -> Result<WebResult<DisplayHold>, BigNeonError> {
    let conn = conn.get();
    let hold = Hold::find(path.id, conn)?;
    user.requires_scope_for_organization(Scopes::CompWrite, &hold.organization(conn)?, conn)?;
    let new_comp = new_comp.into_inner();
    let comp = Hold::create_comp_for_person(
        new_comp.name,
        Some(user.id()),
        hold.id,
        new_comp.email,
        new_comp.phone,
        new_comp.redemption_code,
        new_comp.end_at,
        new_comp.max_per_user,
        new_comp.quantity,
        conn,
    )?;

    Ok(WebResult::new(StatusCode::CREATED, comp.into_display(conn)?))
}

pub fn update(
    (conn, req, path, user): (Connection, Json<UpdateHoldRequest>, Path<PathParameters>, User),
) -> Result<HttpResponse, BigNeonError> {
    let conn = conn.get();

    let comp = Hold::find(path.id, conn)?;
    user.requires_scope_for_organization(Scopes::CompWrite, &comp.organization(conn)?, conn)?;
    let req = req.into_inner();
    let quantity = req.quantity;
    let hold = comp.update(req.into(), conn)?;
    if quantity.is_some() {
        hold.set_quantity(Some(user.id()), quantity.unwrap(), conn)?;
    }

    let comp = hold.into_display(conn)?;
    Ok(HttpResponse::Ok().json(comp))
}

pub fn destroy((conn, path, user): (Connection, Path<PathParameters>, User)) -> Result<HttpResponse, BigNeonError> {
    let conn = conn.get();
    let hold = Hold::find(path.id, conn)?;
    user.requires_scope_for_organization(Scopes::CompWrite, &hold.organization(conn)?, conn)?;

    let comp = Hold::find(path.id, conn)?;
    comp.destroy(Some(user.id()), &*conn)?;
    Ok(HttpResponse::Ok().json(json!({})))
}
