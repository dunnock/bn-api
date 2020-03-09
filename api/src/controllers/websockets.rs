use crate::auth::user::User;
use crate::db::Connection;
use crate::errors::*;
use crate::models::*;
use crate::server::AppState;
use actix_web::{HttpRequest, HttpResponse, web::{Path, Payload}};
use actix_web_actors::ws;
use bigneon_db::prelude::*;

pub fn initate(
    (conn, path, request, user): (Connection, Path<PathParameters>, HttpRequest, User),
    stream: Payload
) -> Result<HttpResponse, BigNeonError> {
    let conn = conn.get();
    let event = Event::find(path.id, conn)?;
    user.requires_scope_for_organization_event(Scopes::WebSocketInitiate, &event.organization(conn)?, &event, conn)?;
    Ok(ws::start(EventWebSocket::new(event.id), &request, stream)
        .map_err(|err| ApplicationError::new(format!("Websocket error: {:?}", err)))?)
}
