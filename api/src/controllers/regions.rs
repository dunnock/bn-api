use crate::auth::user::User;
use crate::database::Connection;
use crate::errors::*;
use crate::extractors::*;
use crate::models::PathParameters;
use crate::models::WebPayload;
use actix_web::{
    http::StatusCode,
    web::{Path, Query},
    HttpResponse,
};
use db::models::*;

pub async fn index(
    (connection, query_parameters): (Connection, Query<PagingParameters>),
) -> Result<WebPayload<Region>, ApiError> {
    //TODO refactor query using paging parameters
    let regions = Region::all(connection.get())?;

    Ok(WebPayload::new(
        StatusCode::OK,
        Payload::from_data(regions, query_parameters.page(), query_parameters.limit(), None),
    ))
}

pub async fn show((connection, parameters): (Connection, Path<PathParameters>)) -> Result<HttpResponse, ApiError> {
    let region = Region::find(parameters.id, connection.get())?;
    Ok(HttpResponse::Ok().json(&region))
}

pub async fn create(
    (connection, new_region, user): (Connection, Json<NewRegion>, User),
) -> Result<HttpResponse, ApiError> {
    user.requires_scope(Scopes::RegionWrite)?;
    let connection = connection.get();
    let region = new_region.into_inner().commit(connection)?;
    Ok(HttpResponse::Created().json(&region))
}

pub async fn update(
    (connection, parameters, region_parameters, user): (
        Connection,
        Path<PathParameters>,
        Json<RegionEditableAttributes>,
        User,
    ),
) -> Result<HttpResponse, ApiError> {
    user.requires_scope(Scopes::RegionWrite)?;
    let connection = connection.get();
    let region = Region::find(parameters.id, connection)?;
    let updated_region = region.update(region_parameters.into_inner(), connection)?;
    Ok(HttpResponse::Ok().json(updated_region))
}
