use actix_web::{ws, HttpRequest, HttpResponse, Path};
use crate::auth::user::User;
use bigneon_db::prelude::*;
use crate::db::Connection;
use crate::errors::*;
use crate::models::*;
use crate::server::AppState;

pub fn initate(
    (conn, path, request, user): (Connection, Path<PathParameters>, HttpRequest<AppState>, User),
) -> Result<HttpResponse, BigNeonError> {
    let conn = conn.get();
    let event = Event::find(path.id, conn)?;
    user.requires_scope_for_organization_event(Scopes::WebSocketInitiate, &event.organization(conn)?, &event, conn)?;
    Ok(ws::start(&request, EventWebSocket::new(event.id))
        .map_err(|err| ApplicationError::new(format!("Websocket error: {:?}", err)))?)
}
