use crate::auth::user::User as AuthUser;
use crate::controllers::events::{self, *};
use crate::db::{Connection, ReadonlyConnection};
use crate::errors::*;
use crate::extractors::*;
use crate::helpers::application;
use crate::models::*;
use crate::server::AppState;
use actix_web::{
    web::{Data, Path, Query},
    HttpResponse,
};
use bigneon_db::prelude::*;
use reqwest::StatusCode;

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct CityData {
    pub city: String,
    pub state: String,
    pub country: String,
    pub google_place_id: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub timezone: String,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct SlugMetaData {
    pub title: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, PartialEq, Serialize)]
#[serde(tag = "type")]
pub enum SlugResponse {
    Organization {
        organization: DisplayOrganization,
        events: Vec<EventVenueEntry>,
        meta: SlugMetaData,
    },
    City {
        city: CityData,
        events: Vec<EventVenueEntry>,
        meta: SlugMetaData,
    },
    Venue {
        venue: DisplayVenue,
        events: Vec<EventVenueEntry>,
        meta: SlugMetaData,
    },
    Genre {
        genre: String,
        events: Vec<EventVenueEntry>,
        meta: SlugMetaData,
    },
}

pub async fn index(
    (connection, query, user): (ReadonlyConnection, Query<PagingParameters>, AuthUser),
) -> Result<WebPayload<Slug>, BigNeonError> {
    let connection = connection.get();
    user.requires_scope(Scopes::OrgAdmin)?;
    let slug_type = query
        .get_tag("type")
        .and_then(|s| Some(s.parse::<SlugTypes>().unwrap_or(SlugTypes::Genre)));
    let query_string = query.get_tag("query");
    let (slugs, slug_total) = Slug::search(query_string, slug_type, query.page(), query.limit(), connection)?;
    let mut payload = Payload::from_data(slugs, query.page(), query.limit(), Some(slug_total as u64));

    payload.paging.tags = query.tags.clone();
    Ok(WebPayload::new(StatusCode::OK, payload))
}

pub async fn update(
    (connection, parameters, slug_parameters, user): (
        Connection,
        Path<PathParameters>,
        Json<SlugEditableAttributes>,
        AuthUser,
    ),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    user.requires_scope(Scopes::OrgAdmin)?;
    let slug = Slug::find(parameters.id, connection)?;
    let updated_slug = slug.update(slug_parameters.into_inner(), connection)?;
    Ok(HttpResponse::Ok().json(updated_slug))
}

pub async fn show(
    (state, conn, mut parameters, query, auth_user, request): (
        Data<AppState>,
        ReadonlyConnection,
        Path<StringPathParameters>,
        Query<EventParameters>,
        OptionalUser,
        RequestInfo,
    ),
) -> Result<HttpResponse, BigNeonError> {
    let user = auth_user
        .clone()
        .into_inner()
        .and_then(|auth_user| Some(auth_user.user));
    let connection = conn.clone();
    let connection = connection.get();
    let slugs = Slug::find_by_slug(&parameters.id, connection)?;

    if slugs.is_empty() {
        return application::not_found();
    }

    let slug = &slugs[0];
    match slug.slug_type {
        SlugTypes::Organization | SlugTypes::Venue | SlugTypes::Event => {
            let primary_slug = Slug::primary_slug(slug.main_table_id, slug.main_table, connection)?;
            if primary_slug.slug != slug.slug {
                return redirection_json(&primary_slug, state);
            }
        }
        _ => (),
    }

    let meta = SlugMetaData {
        title: slug.title.clone(),
        description: slug.description.clone(),
    };

    let response = match slug.slug_type {
        SlugTypes::Event => {
            parameters.id = slug.main_table_id.to_string();
            return events::show((state, conn, parameters, query, auth_user, request)).await;
        }
        SlugTypes::Organization => {
            let organization = Organization::find(slug.main_table_id, connection)?;

            let (events, _) = Event::search(
                None,
                None,
                Some(organization.id),
                None,
                None,
                None,
                None,
                None,
                EventSearchSortField::EventStart,
                SortingDir::Asc,
                user.clone(),
                PastOrUpcoming::Upcoming,
                None,
                &Paging::new(0, std::u32::MAX),
                state.service_locator.country_lookup_service(),
                connection,
            )?;

            let events = EventVenueEntry::event_venues_from_events(events, user, &state, connection)?;
            SlugResponse::Organization {
                organization: organization.for_display(connection)?,
                events,
                meta,
            }
        }
        SlugTypes::Venue => {
            let venue = Venue::find(slug.main_table_id, connection)?;

            let (events, _) = Event::search(
                None,
                None,
                None,
                Some(vec![venue.id]),
                None,
                None,
                None,
                None,
                EventSearchSortField::EventStart,
                SortingDir::Asc,
                user.clone(),
                PastOrUpcoming::Upcoming,
                None,
                &Paging::new(0, std::u32::MAX),
                state.service_locator.country_lookup_service(),
                connection,
            )?;

            let events = EventVenueEntry::event_venues_from_events(events, user, &state, connection)?;
            SlugResponse::Venue {
                venue: venue.for_display(connection)?,
                events,
                meta,
            }
        }
        SlugTypes::City => {
            let venue = Venue::find(slug.main_table_id, connection)?;
            let city = CityData {
                city: venue.city,
                state: venue.state,
                country: venue.country,
                google_place_id: venue.google_place_id,
                latitude: venue.latitude,
                longitude: venue.longitude,
                timezone: venue.timezone,
            };

            let (events, _) = Event::search(
                None,
                None,
                None,
                Some(slugs.iter().map(|s| s.main_table_id).collect()),
                None,
                None,
                None,
                None,
                EventSearchSortField::EventStart,
                SortingDir::Asc,
                user.clone(),
                PastOrUpcoming::Upcoming,
                None,
                &Paging::new(0, std::u32::MAX),
                state.service_locator.country_lookup_service(),
                connection,
            )?;

            let events = EventVenueEntry::event_venues_from_events(events, user, &state, connection)?;
            SlugResponse::City { city, events, meta }
        }
        SlugTypes::Genre => {
            let genre = Genre::find(slug.main_table_id, connection)?;

            let (events, _) = Event::search(
                None,
                None,
                None,
                None,
                Some(vec![genre.name.clone()]),
                None,
                None,
                None,
                EventSearchSortField::EventStart,
                SortingDir::Asc,
                user.clone(),
                PastOrUpcoming::Upcoming,
                None,
                &Paging::new(0, std::u32::MAX),
                state.service_locator.country_lookup_service(),
                connection,
            )?;

            let events = EventVenueEntry::event_venues_from_events(events, user, &state, connection)?;
            SlugResponse::Genre {
                genre: genre.name,
                events,
                meta,
            }
        }
        SlugTypes::CityGenre => {
            return application::not_found();
        }
    };

    Ok(HttpResponse::Ok().json(&response))
}

fn redirection_json(slug: &Slug, state: Data<AppState>) -> Result<HttpResponse, BigNeonError> {
    let path = match slug.slug_type {
        SlugTypes::Event => "tickets",
        SlugTypes::Venue => "venues",
        SlugTypes::Organization => "organizations",
        _ => return application::bad_request("Slug type is not valid for redirection"),
    };

    Ok(HttpResponse::Ok().json(json!({
        "redirect": format!("{}/{}/{}", &state.config.front_end_url, path, &slug.slug)
    })))
}
