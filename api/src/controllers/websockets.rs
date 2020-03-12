use crate::auth::user::User;
use crate::db::Connection;
use crate::errors::*;
use crate::models::*;
use crate::server::AppState;
use actix_web::{
    web::{Data, Path, Payload},
    HttpRequest, HttpResponse,
};
use actix_web_actors::ws;
use bigneon_db::prelude::*;

pub async fn initate(
    (conn, path, request, user, state): (Connection, Path<PathParameters>, HttpRequest, User, Data<AppState>),
    stream: Payload,
) -> Result<HttpResponse, BigNeonError> {
    let conn = conn.get();
    let event = Event::find(path.id, conn)?;
    user.requires_scope_for_organization_event(Scopes::WebSocketInitiate, &event.organization(conn)?, &event, conn)?;
    Ok(
        ws::start(EventWebSocket::new(event.id, state.clients.clone()), &request, stream)
            .map_err(|err| ApplicationError::new(format!("Websocket error: {:?}", err)))?,
    )
}
