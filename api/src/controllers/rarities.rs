use actix_web::{HttpResponse, Path};
use animo_db::models::*;
use auth::user::User;
use db::Connection;
use errors::*;
use extractors::*;
use models::PathParameters;

pub fn create(
    (connection, new_rarity, path, user): (Connection, Json<NewRarity>, Path<PathParameters>, User),
) -> Result<HttpResponse, AnimoError> {
    let connection = connection.get();
    let event = Event::find(path.id, connection)?;
    let org = event.organization(connection)?;
    user.requires_scope_for_organization_event(Scopes::RarityWrite, &org, &event, connection)?;
    let mut new_rarity = new_rarity.into_inner();
    new_rarity.event_id = Some(path.id);
    let rarity = new_rarity.commit(connection)?;
    Ok(HttpResponse::Created().json(&rarity))
}
