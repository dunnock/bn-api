use crate::auth::user::User;
use crate::database::Connection;
use crate::errors::*;
use crate::extractors::*;
use crate::models::PathParameters;
use actix_web::{
    web::{Path, Query},
    HttpResponse,
};
use db::models::*;
use uuid::Uuid;

pub async fn index(
    (connection, query_parameters, user): (Connection, Query<PagingParameters>, OptionalUser),
) -> Result<HttpResponse, ApiError> {
    //TODO implement proper paging on db
    let venues = match user.into_inner() {
        Some(u) => Venue::all(Some(&u.user), connection.get())?,
        None => Venue::all(None, connection.get())?,
    };

    Ok(HttpResponse::Ok().json(&Payload::from_data(
        venues,
        query_parameters.page(),
        query_parameters.limit(),
        None,
    )))
}

pub async fn show((connection, parameters): (Connection, Path<PathParameters>)) -> Result<HttpResponse, ApiError> {
    let connection = connection.get();
    let venue = Venue::find(parameters.id, connection)?;

    Ok(HttpResponse::Ok().json(&venue))
}

pub async fn show_from_organizations(
    (connection, organization_id, query_parameters, user): (
        Connection,
        Path<PathParameters>,
        Query<PagingParameters>,
        OptionalUser,
    ),
) -> Result<HttpResponse, ApiError> {
    //TODO implement proper paging on db
    let venues = match user.into_inner() {
        Some(u) => Venue::find_for_organization(Some(&u.user), organization_id.id, connection.get())?,
        None => Venue::find_for_organization(None, organization_id.id, connection.get())?,
    };

    Ok(HttpResponse::Ok().json(&Payload::from_data(
        venues,
        query_parameters.page(),
        query_parameters.limit(),
        None,
    )))
}

#[derive(Serialize, Deserialize, Default, PartialEq, Debug, Clone)]
pub struct NewVenueData {
    pub name: String,
    pub region_id: Option<Uuid>,
    pub address: String,
    pub city: String,
    pub state: String,
    pub country: String,
    pub postal_code: String,
    pub phone: Option<String>,
    pub promo_image_url: Option<String>,
    pub google_place_id: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub timezone: String,
    pub organization_ids: Vec<Uuid>,
}

impl From<NewVenueData> for NewVenue {
    fn from(v: NewVenueData) -> NewVenue {
        NewVenue {
            name: v.name,
            region_id: v.region_id,
            address: v.address,
            city: v.city,
            state: v.state,
            country: v.country,
            postal_code: v.postal_code,
            phone: v.phone,
            promo_image_url: v.promo_image_url,
            google_place_id: v.google_place_id,
            latitude: v.latitude,
            longitude: v.longitude,
            timezone: v.timezone,
        }
    }
}

pub async fn create(
    (connection, new_venue, user): (Connection, Json<NewVenueData>, User),
) -> Result<HttpResponse, ApiError> {
    let connection = connection.get();

    let org_ids = new_venue.organization_ids.clone();
    for org_id in org_ids.iter() {
        let organization = Organization::find(*org_id, connection)?;
        user.requires_scope_for_organization(Scopes::VenueWrite, &organization, connection)?;
    }
    let venue: NewVenue = NewVenue::from(new_venue.into_inner());
    let mut venue: Venue = venue.commit(connection)?;
    // New venues belonging to an organization start private
    venue = venue.set_privacy(true, connection)?;
    for org_id in org_ids.iter() {
        OrganizationVenue::create(*org_id, venue.id).commit(connection)?;
    }
    Ok(HttpResponse::Created().json(&venue))
}

pub async fn toggle_privacy(
    (connection, parameters, user): (Connection, Path<PathParameters>, User),
) -> Result<HttpResponse, ApiError> {
    let connection = connection.get();
    user.requires_scope(Scopes::VenueWrite)?;

    let venue = Venue::find(parameters.id, connection)?;
    let updated_venue = venue.set_privacy(!venue.is_private, connection)?;
    Ok(HttpResponse::Ok().json(updated_venue))
}

pub async fn update(
    (connection, parameters, venue_parameters, user): (
        Connection,
        Path<PathParameters>,
        Json<VenueEditableAttributes>,
        User,
    ),
) -> Result<HttpResponse, ApiError> {
    let connection = connection.get();
    let venue = Venue::find(parameters.id, connection)?;
    if !venue.is_private {
        user.requires_scope(Scopes::VenueWrite)?;
    } else {
        let mut eligible_for_updating = false;
        for organization in venue.organizations(connection)? {
            eligible_for_updating = user.has_scope_for_organization(Scopes::VenueWrite, &organization, connection)?;
            if eligible_for_updating {
                break;
            }
        }

        if !eligible_for_updating {
            user.requires_scope(Scopes::VenueWrite)?;
        }
    }

    let updated_venue = venue.update(venue_parameters.into_inner(), connection)?;
    Ok(HttpResponse::Ok().json(updated_venue))
}
