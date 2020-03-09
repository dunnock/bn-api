use crate::auth::user::User;
use crate::db::Connection;
use crate::errors::BigNeonError;
use crate::extractors::*;
use crate::models::{PathParameters, WebPayload, WebResult};
use actix_web::{http::StatusCode, HttpResponse, web::Path};
use bigneon_db::models::*;

#[derive(Deserialize, Serialize)]
pub struct NewEventReportSubscriberRequest {
    pub email: String,
    pub report_type: ReportTypes,
}

pub fn index(
    (conn, path, user): (Connection, Path<PathParameters>, User),
) -> Result<WebPayload<EventReportSubscriber>, BigNeonError> {
    let conn = conn.get();
    let event = Event::find(path.id, conn)?;
    user.requires_scope_for_organization_event(
        Scopes::EventReportSubscriberRead,
        &event.organization(conn)?,
        &event,
        conn,
    )?;

    let report_subscribers = EventReportSubscriber::find_all(event.id, ReportTypes::TicketCounts, conn)?;
    let payload: Payload<EventReportSubscriber> = report_subscribers.into();
    Ok(WebPayload::new(StatusCode::OK, payload))
}

pub fn create(
    (conn, subscriber_request, path, user): (
        Connection,
        Json<NewEventReportSubscriberRequest>,
        Path<PathParameters>,
        User,
    ),
) -> Result<WebResult<EventReportSubscriber>, BigNeonError> {
    let conn = conn.get();
    let event = Event::find(path.id, conn)?;
    user.requires_scope_for_organization_event(
        Scopes::EventReportSubscriberWrite,
        &event.organization(conn)?,
        &event,
        conn,
    )?;
    let event_subscriber = EventReportSubscriber::create(
        event.id,
        subscriber_request.report_type,
        subscriber_request.email.clone(),
    )
    .commit(Some(user.id()), conn)?;

    Ok(WebResult::new(StatusCode::CREATED, event_subscriber))
}

pub fn destroy((conn, path, user): (Connection, Path<PathParameters>, User)) -> Result<HttpResponse, BigNeonError> {
    let conn = conn.get();
    let event_report_subscriber = EventReportSubscriber::find(path.id, conn)?;
    let event = Event::find(event_report_subscriber.event_id, conn)?;
    user.requires_scope_for_organization_event(
        Scopes::EventReportSubscriberDelete,
        &event.organization(conn)?,
        &event,
        conn,
    )?;

    event_report_subscriber.destroy(Some(user.id()), &*conn)?;
    Ok(HttpResponse::Ok().json(json!({})))
}
