use crate::auth::user::User;
use crate::db::Connection;
use crate::errors::BigNeonError;
use crate::extractors::Json;
use crate::models::{PathParameters, WebPayload};
use actix_web::{HttpResponse, web::{Query, Path}};
use bigneon_db::models::scopes::Scopes;
use bigneon_db::models::*;
use reqwest::StatusCode;

pub async fn create(
    (conn, json, user): (Connection, Json<NewOrganizationVenue>, User),
) -> Result<HttpResponse, BigNeonError> {
    user.requires_scope(Scopes::OrgVenueWrite)?;
    let connection = conn.get();

    let organization_venue = json.into_inner().commit(connection)?;
    Ok(HttpResponse::Created().json(json!(organization_venue)))
}

pub async fn organizations_index(
    (conn, path, query, user): (Connection, Path<PathParameters>, Query<PagingParameters>, User),
) -> Result<WebPayload<OrganizationVenue>, BigNeonError> {
    user.requires_scope(Scopes::OrgVenueRead)?;
    let connection = conn.get();
    let organization = Organization::find(path.id, connection)?;
    let organization_venues =
        OrganizationVenue::find_by_organization(organization.id, Some(query.page()), Some(query.limit()), connection)?;
    Ok(WebPayload::new(StatusCode::OK, organization_venues))
}

pub async fn venues_index(
    (conn, path, query, user): (Connection, Path<PathParameters>, Query<PagingParameters>, User),
) -> Result<WebPayload<OrganizationVenue>, BigNeonError> {
    user.requires_scope(Scopes::OrgVenueRead)?;
    let connection = conn.get();
    let venue = Venue::find(path.id, connection)?;
    let organization_venues =
        OrganizationVenue::find_by_venue(venue.id, Some(query.page()), Some(query.limit()), connection)?;
    Ok(WebPayload::new(StatusCode::OK, organization_venues))
}

pub async fn show((conn, path, user): (Connection, Path<PathParameters>, User)) -> Result<HttpResponse, BigNeonError> {
    user.requires_scope(Scopes::OrgVenueRead)?;
    let connection = conn.get();
    let organization_venue = OrganizationVenue::find(path.id, connection)?;
    Ok(HttpResponse::Ok().json(organization_venue))
}

pub async fn destroy((conn, path, user): (Connection, Path<PathParameters>, User)) -> Result<HttpResponse, BigNeonError> {
    user.requires_scope(Scopes::OrgVenueDelete)?;
    let connection = conn.get();
    let organization_venue = OrganizationVenue::find(path.id, connection)?;
    let organization_venue = organization_venue.destroy(connection)?;
    Ok(HttpResponse::Ok().json(organization_venue))
}
