use actix_web::{http::StatusCode, HttpResponse, web::Query};
use crate::auth::user::{User as AuthUser, User};
use bigneon_db::models::{DomainAction, Report, Scopes};
use bigneon_db::prelude::{DisplayOrder, Event, Order, Paging, PagingParameters, Payload};
use crate::db::Connection;
use crate::errors::*;
use crate::models::WebPayload;

pub fn admin_ticket_count((connection, user): (Connection, AuthUser)) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    //Check if they have org admin permissions
    user.requires_scope(Scopes::OrgAdmin)?;
    let result = Report::ticket_sales_and_counts(None, None, None, None, false, false, false, false, connection)?;
    Ok(HttpResponse::Ok().json(result))
}

pub fn admin_stuck_domain_actions((connection, user): (Connection, AuthUser)) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    //Check if they have org admin permissions
    user.requires_scope(Scopes::OrgAdmin)?;
    let result = DomainAction::find_stuck(connection)?;
    Ok(HttpResponse::Ok().json(result))
}

pub fn orders(
    (conn, query, user): (Connection, Query<PagingParameters>, User),
) -> Result<WebPayload<DisplayOrder>, BigNeonError> {
    let conn = conn.get();
    let event_id = match query.get_tag_as_str("event_id") {
        Some(e) => Some(e.parse()?),
        None => None,
    };
    if let Some(ref event_id) = event_id {
        let event = Event::find(*event_id, conn)?;
        let org = event.organization(conn)?;
        user.requires_scope_for_organization_event(Scopes::OrderRead, &org, &event, conn)?;
    } else {
        // this should only be admins
        user.requires_scope(Scopes::OrderRead)?;
    };

    let ticket_type_id = match query.get_tag_as_str("ticket_type_id") {
        Some(tt) => Some(tt.parse()?),
        None => None,
    };
    let platform = query.get_tag_as_str("platform").map(|s| s.to_lowercase());
    let mut paging: Paging = query.clone().into();
    let orders = Order::search(
        event_id,
        None,
        query.query(),
        query.get_tag_as_str("order_no"),
        query.get_tag_as_str("ticket_no"),
        query.get_tag_as_str("email"),
        query.get_tag_as_str("phone"),
        query.get_tag_as_str("name"),
        ticket_type_id,
        query.get_tag_as_str("promo_code"),
        platform.is_none() || platform == Some("boxoffice".to_string()),
        platform.is_none() || platform != Some("boxoffice".to_string()),
        platform.is_none() || platform != Some("app".to_string()),
        platform.is_none() || platform == Some("app".to_string()),
        //        match query.get_tag_as_str("payment_method") {
        //            Some(pm) => Some(pm.parse()?),
        //            None => None,
        //        },
        match query.get_tag_as_str("start_date") {
            Some(sd) => Some(sd.parse()?),
            None => None,
        },
        match query.get_tag_as_str("end_date") {
            Some(ed) => Some(ed.parse()?),
            None => None,
        },
        user.id(),
        &query,
        conn,
    )?;
    paging.total = orders.1 as u64;
    Ok(WebPayload::new(StatusCode::OK, Payload::new(orders.0, paging)))
}
