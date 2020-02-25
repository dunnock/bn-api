use actix_web::Path;
use actix_web::{HttpResponse, Query};
use auth::user::User;
use bigneon_db::models::scopes::Scopes;
use bigneon_db::models::*;
use bigneon_db::prelude::Optional;
use db::Connection;
use errors::BigNeonError;
use extractors::Json;
use models::{PathParameters, WebPayload};
use reqwest::StatusCode;

pub fn create(
    (conn, json, user): (Connection, Json<NewOrganizationVenue>, User),
) -> Result<HttpResponse, BigNeonError> {
    user.requires_scope(Scopes::OrgVenueWrite)?;
    let connection = conn.get();

    let organization_venue = json.into_inner().commit(connection)?;
    Ok(HttpResponse::Created().json(json!(organization_venue)))
}

pub fn index(
    (conn, path, query, user): (Connection, Path<PathParameters>, Query<PagingParameters>, User),
) -> Result<WebPayload<OrganizationVenue>, BigNeonError> {
    user.requires_scope(Scopes::OrgVenueRead)?;
    let connection = conn.get();

    // Route is used for venue and organization lookups
    let organization_venues = if Organization::find(path.id, connection).optional()?.is_some() {
        OrganizationVenue::find_by_organization(path.id, Some(query.page()), Some(query.limit()), connection)?
    } else {
        OrganizationVenue::find_by_venue(path.id, Some(query.page()), Some(query.limit()), connection)?
    };

    Ok(WebPayload::new(StatusCode::OK, organization_venues))
}

pub fn show((conn, path, user): (Connection, Path<PathParameters>, User)) -> Result<HttpResponse, BigNeonError> {
    user.requires_scope(Scopes::OrgVenueRead)?;
    let connection = conn.get();
    let organization_venue = OrganizationVenue::find(path.id, connection)?;
    Ok(HttpResponse::Ok().json(organization_venue))
}

pub fn destroy((conn, path, user): (Connection, Path<PathParameters>, User)) -> Result<HttpResponse, BigNeonError> {
    user.requires_scope(Scopes::OrgVenueDelete)?;
    let connection = conn.get();
    let organization_venue = OrganizationVenue::find(path.id, connection)?;
    let organization_venue = organization_venue.destroy(connection)?;
    Ok(HttpResponse::Ok().json(organization_venue))
}
