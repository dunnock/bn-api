use crate::auth::user::User as AuthUser;
use crate::db::Connection;
use actix_web::{HttpResponse, web::{Path, Query}};
use bigneon_db::models::*;

use crate::errors::*;
use crate::extractors::*;
use crate::models::PathParameters;
use diesel::PgConnection;

pub fn index(
    (connection, path_parameters, query_parameters): (Connection, Path<PathParameters>, Query<PagingParameters>),
) -> Result<HttpResponse, BigNeonError> {
    let stages = Stage::find_by_venue_id(path_parameters.id, connection.get())?;

    Ok(HttpResponse::Ok().json(&Payload::from_data(
        stages,
        query_parameters.page(),
        query_parameters.limit(),
        None,
    )))
}

pub fn show((connection, parameters): (Connection, Path<PathParameters>)) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let stage = Stage::find(parameters.id, connection)?;

    Ok(HttpResponse::Ok().json(&stage))
}

#[derive(Deserialize)]
pub struct CreateStage {
    pub name: String,
    pub description: Option<String>,
    pub capacity: Option<i64>,
}

pub fn create(
    (connection, parameters, create_stage, user): (Connection, Path<PathParameters>, Json<CreateStage>, AuthUser),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let venue = Venue::find(parameters.id, connection)?;
    check_access(&venue, &user, connection)?;

    let new_stage = Stage::create(
        parameters.id,
        create_stage.name.clone(),
        create_stage.description.clone(),
        create_stage.capacity.clone(),
    );
    let stage = new_stage.commit(connection)?;

    Ok(HttpResponse::Created().json(&stage))
}

pub fn update(
    (connection, parameters, stage_parameters, user): (
        Connection,
        Path<PathParameters>,
        Json<StageEditableAttributes>,
        AuthUser,
    ),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let stage = Stage::find(parameters.id, connection)?;
    let venue = Venue::find(stage.venue_id, connection)?;
    check_access(&venue, &user, connection)?;

    let updated_stage = stage.update(stage_parameters.into_inner(), connection)?;
    Ok(HttpResponse::Ok().json(updated_stage))
}

pub fn delete(
    (connection, parameters, user): (Connection, Path<PathParameters>, AuthUser),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let stage = Stage::find(parameters.id, connection)?;
    let venue = Venue::find(stage.venue_id, connection)?;
    check_access(&venue, &user, connection)?;

    stage.destroy(connection)?;
    Ok(HttpResponse::Ok().json(json!({})))
}

fn check_access(venue: &Venue, user: &AuthUser, connection: &PgConnection) -> Result<(), BigNeonError> {
    let mut has_create_access = false;
    for organization in venue.organizations(connection)? {
        has_create_access =
            has_create_access || user.has_scope_for_organization(Scopes::VenueWrite, &organization, connection)?;
        if has_create_access {
            break;
        }
    }

    if !venue.is_private || !has_create_access {
        user.requires_scope(Scopes::VenueWrite)?;
    }

    Ok(())
}
