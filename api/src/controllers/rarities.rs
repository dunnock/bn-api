use crate::auth::user::User;
use crate::database::Connection;
use crate::errors::*;
use crate::extractors::*;
use crate::models::PathParameters;
use actix_web::{
    web::{Data, Path, Query},
    HttpResponse,
};
use db::models::*;

pub fn create(
    (connection, new_rarity, path, user): (Connection, Json<NewRarity>, Path<PathParameters>, User),
) -> Result<HttpResponse, ApiError> {
    let connection = connection.get();
    let event = Event::find(path.id, connection)?;
    let org = event.organization(connection)?;
    user.requires_scope_for_organization_event(Scopes::RarityWrite, &org, &event, connection)?;
    let mut new_rarity = new_rarity.into_inner();
    new_rarity.event_id = Some(path.id);
    let rarity = new_rarity.commit(connection)?;
    Ok(HttpResponse::Created().json(&rarity))
}
