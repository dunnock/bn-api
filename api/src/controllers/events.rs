use crate::auth::user::User as AuthUser;
use crate::controllers::organizations::DisplayOrganizationUser;
use crate::controllers::ticket_types;
use crate::db::Connection;
use crate::db::{CacheDatabase, ReadonlyConnection};
use crate::domain_events::executors::UpdateGenresPayload;
use crate::errors::*;
use crate::extractors::*;
use crate::helpers::*;
use crate::jwt::{encode, Header};
use crate::models::*;
use crate::server::AppState;
use crate::utils::cloudinary::optimize_cloudinary;
use crate::utils::redis::*;
use crate::utils::ServiceLocator;
use actix_web::{http::StatusCode, HttpResponse, web::{Path, Query, Data}};
use bigneon_db::dev::times;
use bigneon_db::prelude::*;
use chrono::prelude::*;
use chrono::Duration;
use diesel::PgConnection;
use serde::Serialize;
use serde_json::Value;
use serde_with::{self, CommaSeparator};
use std::collections::HashMap;
use url::Url;
use uuid::Uuid;

#[derive(Deserialize, Clone, Serialize)]
pub struct SearchParameters {
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    query: Option<String>,
    region_id: Option<Uuid>,
    organization_id: Option<Uuid>,
    venue_id: Option<Uuid>,
    #[serde(default, with = "serde_with::rust::StringWithSeparator::<CommaSeparator>")]
    genres: Vec<String>,
    #[serde(default, with = "serde_with::rust::StringWithSeparator::<CommaSeparator>")]
    status: Vec<EventStatus>,
    start_utc: Option<NaiveDateTime>,
    end_utc: Option<NaiveDateTime>,
    page: Option<u32>,
    limit: Option<u32>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    sort: Option<String>,
    dir: Option<SortingDir>,
    past_or_upcoming: Option<String>,
    updated_at: Option<String>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    category: Option<EventTypes>,
}

impl From<SearchParameters> for Paging {
    fn from(s: SearchParameters) -> Paging {
        let mut default_tags: HashMap<String, Value> = HashMap::new();
        if let Some(ref i) = s.query {
            default_tags.insert("query".to_owned(), json!(i.clone()));
        }
        if let Some(ref i) = s.region_id {
            default_tags.insert("region_id".to_owned(), json!(i));
        }
        if let Some(ref i) = s.organization_id {
            default_tags.insert("organization_id".to_owned(), json!(i));
        }
        for event_status in s.status.clone().into_iter() {
            default_tags.insert("status".to_owned(), json!(event_status));
        }

        if let Some(ref i) = s.start_utc {
            default_tags.insert("start_utc".to_owned(), json!(i));
        }
        if let Some(ref i) = s.end_utc {
            default_tags.insert("end_utc".to_owned(), json!(i));
        }

        PagingParameters {
            page: s.page,
            limit: s.limit,
            sort: s.sort,
            dir: s.dir,
            tags: default_tags,
        }
        .into()
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct EventExportData {
    pub id: Uuid,
    pub name: String,
    pub organization_id: Uuid,
    pub venue: Option<VenueInfo>,
    pub created_at: NaiveDateTime,
    pub event_start: Option<NaiveDateTime>,
    pub door_time: Option<NaiveDateTime>,
    pub event_end: Option<NaiveDateTime>,
    pub status: EventStatus,
    pub promo_image_url: Option<String>,
    pub additional_info: Option<String>,
    pub top_line_info: Option<String>,
    pub age_limit: Option<String>,
    pub cancelled_at: Option<NaiveDateTime>,
    pub min_ticket_price: Option<u32>,
    pub max_ticket_price: Option<u32>,
    pub publish_date: Option<NaiveDateTime>,
    pub on_sale: Option<NaiveDateTime>,
    pub total_tickets: u32,
    pub sold_unreserved: Option<u32>,
    pub sold_held: Option<u32>,
    pub tickets_open: u32,
    pub tickets_held: u32,
    pub tickets_redeemed: u32,
    pub ticket_types: Vec<EventExportTicketType>,
    pub is_external: bool,
    pub external_url: Option<String>,
    pub localized_times: EventLocalizedTimeStrings,
    pub event_type: EventTypes,
    pub slug: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct EventExportTicketType {
    pub event_id: Uuid,
    pub name: String,
    pub status: TicketTypeStatus,
    pub min_price: i64,
    pub max_price: i64,
    pub total: i64,
    pub sold_unreserved: Option<i64>,
    pub sold_held: Option<i64>,
    pub open: i64,
    pub held: i64,
    pub redeemed: i64,
}

impl From<EventSummaryResult> for EventExportData {
    fn from(e: EventSummaryResult) -> EventExportData {
        EventExportData {
            id: e.id,
            name: e.name.clone(),
            organization_id: e.organization_id,
            venue: e.venue.clone(),
            created_at: e.created_at,
            event_start: e.event_start,
            door_time: e.door_time,
            event_end: e.event_end,
            status: e.status,
            promo_image_url: e.promo_image_url.clone(),
            additional_info: e.additional_info.clone(),
            top_line_info: e.top_line_info.clone(),
            age_limit: e.age_limit.clone(),
            cancelled_at: e.cancelled_at,
            min_ticket_price: e.min_ticket_price,
            max_ticket_price: e.max_ticket_price,
            publish_date: e.publish_date,
            on_sale: e.on_sale,
            total_tickets: e.total_tickets,
            sold_unreserved: e.sold_unreserved,
            sold_held: e.sold_held,
            tickets_open: e.tickets_open,
            tickets_held: e.tickets_held,
            tickets_redeemed: e.tickets_redeemed,
            ticket_types: e.ticket_types.clone().into_iter().map(|tt| tt.into()).collect(),
            is_external: e.is_external,
            external_url: e.external_url.clone(),
            localized_times: e.localized_times.clone(),
            event_type: e.event_type.clone(),
            slug: e.slug.clone(),
        }
    }
}

impl From<EventSummaryResultTicketType> for EventExportTicketType {
    fn from(tt: EventSummaryResultTicketType) -> EventExportTicketType {
        EventExportTicketType {
            event_id: tt.event_id,
            name: tt.name,
            status: tt.status,
            min_price: tt.min_price,
            max_price: tt.max_price,
            total: tt.total,
            sold_unreserved: tt.sold_unreserved,
            sold_held: tt.sold_held,
            open: tt.open,
            held: tt.held,
            redeemed: tt.redeemed,
        }
    }
}

pub async fn export_event_data(
    (connection, path, paging, user): (Connection, Path<PathParameters>, Query<PagingParameters>, AuthUser),
) -> Result<WebPayload<EventExportData>, BigNeonError> {
    let conn = connection.get();
    let organization = Organization::find(path.id, conn)?;
    user.requires_scope_for_organization(Scopes::EventDataRead, &organization, conn)?;

    let events = Event::find_all_events_for_organization(
        path.id,
        paging
            .get_tag("past_or_upcoming")
            .map(|past_or_upcoming| past_or_upcoming.parse().unwrap_or(PastOrUpcoming::Upcoming)),
        None,
        true,
        paging.page(),
        paging.limit(),
        conn,
    )?;

    let export_data: Vec<EventExportData> = events.data.into_iter().map(|e| e.into()).collect();
    Ok(WebPayload::new(
        StatusCode::OK,
        Payload::from_data(export_data, paging.page(), paging.limit(), None),
    ))
}

/**
 * What events does this user have authority to check in
**/
pub async fn checkins(
    (conn, query, auth_user, state): (Connection, Query<SearchParameters>, AuthUser, Data<AppState>),
) -> Result<HttpResponse, BigNeonError> {
    let events = auth_user.user.find_events_with_access_to_scan(conn.get())?;
    let mut payload = Payload::new(
        EventVenueEntry::event_venues_from_events(events, Some(auth_user.user), &state, conn.get())?,
        query.into_inner().into(),
    );
    payload.paging.total = payload.data.len() as u64;
    payload.paging.limit = 100;
    Ok(HttpResponse::Ok().json(&payload))
}

pub async fn index(
    (state, connection, query, auth_user): (
        Data<AppState>,
        ReadonlyConnection,
        Query<SearchParameters>,
        OptionalUser,
    ),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let query = query.into_inner();
    let paging = query.clone().into();
    let user = auth_user.into_inner().and_then(|auth_user| Some(auth_user.user));

    let past_or_upcoming = match query
        .past_or_upcoming
        .clone()
        .unwrap_or("upcoming".to_string())
        .as_str()
    {
        "past" => PastOrUpcoming::Past,
        _ => PastOrUpcoming::Upcoming,
    };

    let sort_field = match query.sort.clone().unwrap_or("event_start".to_string()).as_str() {
        "event_start" => EventSearchSortField::EventStart,
        "name" => EventSearchSortField::Name,
        _ => EventSearchSortField::EventStart,
    };

    let events_count = Event::search(
        query.query.clone(),
        query.region_id,
        query.organization_id,
        query.venue_id.map(|v| vec![v]),
        if query.genres.is_empty() {
            None
        } else {
            Some(query.genres.clone())
        },
        query.start_utc,
        query.end_utc,
        if query.status.is_empty() {
            None
        } else {
            Some(query.status.clone())
        },
        sort_field,
        query.dir.clone().unwrap_or(SortingDir::Asc),
        user.clone(),
        past_or_upcoming,
        query.category.clone(),
        &paging,
        state.service_locator.country_lookup_service(),
        connection,
    )?;
    let (events, count) = events_count;

    let mut payload = Payload::new(
        EventVenueEntry::event_venues_from_events(events, user, &state, connection)?,
        query.into(),
    );
    payload.paging.total = count as u64;
    payload.paging.limit = paging.limit;

    Ok(HttpResponse::Ok().json(&payload))
}

#[derive(Deserialize)]
pub struct EventParameters {
    pub box_office_pricing: Option<bool>,
    pub redemption_code: Option<String>,
    pub private_access_code: Option<String>,
}

pub async fn show(
    (state, connection, parameters, query, user, request): (
        Data<AppState>,
        ReadonlyConnection,
        Path<StringPathParameters>,
        Query<EventParameters>,
        OptionalUser,
        RequestInfo,
    ),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let user = user.into_inner();

    let event_id = match parameters.id.parse() {
        Ok(i) => i,
        Err(_) => {
            // Backwards compatibility for existing links
            let slugs = Slug::find_by_slug(&parameters.id, connection)?;
            if slugs.is_empty() || (slugs[0].main_table != Tables::Events || slugs[0].slug_type != SlugTypes::Event) {
                return application::not_found();
            }

            slugs[0].main_table_id
        }
    };

    let (event, organization, venue, fee_schedule) = Event::find_incl_org_venue_fees(event_id, connection)?;

    if event.private_access_code.is_some()
        && !(query.private_access_code.is_some()
            && event.private_access_code.clone().unwrap() == query.private_access_code.clone().unwrap().to_lowercase())
    {
        match user {
            Some(ref user) => user.requires_scope_for_organization(Scopes::OrgReadEvents, &organization, connection)?,
            None => {
                return application::unauthorized_with_message("Unauthorized access of private event", None, None);
            }
        }
    };

    let user_has_privileges = match user {
        Some(ref user) => {
            user.has_scope_for_organization_event(Scopes::EventWrite, &organization, event.id, connection)?
        }
        None => false,
    };

    let event_ended = event.event_end.unwrap_or(times::infinity()) < dates::now().finish();
    if !user_has_privileges
        && (event.publish_date.unwrap_or(times::infinity()) > dates::now().finish() || event.deleted_at.is_some())
    {
        return application::not_found();
    }

    let localized_times = event.get_all_localized_time_strings(venue.as_ref());
    let event_artists = EventArtist::find_all_from_event(event.id, connection)?;
    let total_interest = EventInterest::total_interest(event.id, connection)?;
    let user_interest = match user {
        Some(ref u) => EventInterest::user_interest(event.id, u.id(), connection)?,
        None => false,
    };

    let box_office_pricing = query.box_office_pricing.unwrap_or(false);
    if box_office_pricing {
        match user {
            Some(ref user) => {
                user.requires_scope_for_organization(Scopes::BoxOfficeTicketRead, &organization, connection)?
            }
            None => {
                return application::unauthorized_with_message("Cannot access box office pricing", None, None);
            }
        }
    }

    let ticket_types = TicketType::find_by_event_id(event.id, true, query.redemption_code.clone(), connection)?;
    let mut display_ticket_types = Vec::new();
    let mut sales_start_date = Some(times::infinity());
    let mut limited_tickets_remaining: Vec<TicketsRemaining> = Vec::new();

    let platform = if box_office_pricing {
        Platforms::BoxOffice
    } else {
        // If we can't determine the platform, then serve it as web
        if let Some(user_agent) = request.user_agent {
            Platforms::from_user_agent(user_agent.as_str()).unwrap_or(Platforms::Web)
        } else {
            Platforms::Web
        }
    };

    for ticket_type in ticket_types {
        match platform {
            Platforms::App => {
                if !ticket_type.app_sales_enabled {
                    continue;
                }
            }
            Platforms::Web => {
                if !ticket_type.web_sales_enabled {
                    continue;
                }
            }
            Platforms::BoxOffice => {
                if !ticket_type.box_office_sales_enabled {
                    continue;
                }
            }
        };

        if ticket_type.status != TicketTypeStatus::Cancelled {
            let display_ticket_type = UserDisplayTicketType::from_ticket_type(
                &ticket_type,
                &fee_schedule,
                box_office_pricing,
                query.redemption_code.clone(),
                connection,
            )?;

            // Only show private ticket types via holds
            if ticket_type.visibility == TicketTypeVisibility::Hidden && display_ticket_type.redemption_code.is_none() {
                continue;
            }

            if sales_start_date.unwrap() > ticket_type.start_date.clone().unwrap_or(times::infinity()) {
                sales_start_date = ticket_type.start_date.clone();
            }

            // If the ticket type is sold out, hide it if necessary
            if display_ticket_type.status != TicketTypeStatus::Published
                && ticket_type.visibility == TicketTypeVisibility::WhenAvailable
            {
                continue;
            };

            display_ticket_types.push(display_ticket_type);
        }
    }

    if let Some(ref u) = user {
        let tickets_bought = Order::quantity_for_user_for_event(u.id(), event.id, connection)?;
        for (tt_id, num) in tickets_bought {
            let limit = TicketType::find(tt_id, connection)?.limit_per_person;
            if limit > 0 {
                limited_tickets_remaining.push(TicketsRemaining {
                    ticket_type_id: tt_id,
                    tickets_remaining: limit - num,
                });
            }
        }
    }

    let mut tracking_keys =
        Organization::tracking_keys_for_ids(vec![organization.id], &state.config.api_keys_encryption_key, connection)?
            .get(&organization.id)
            .unwrap_or(&TrackingKeys { ..Default::default() })
            .clone();

    if let Some(ref pixel) = event.facebook_pixel_key {
        tracking_keys.facebook_pixel_key = Some(pixel.to_string());
    }

    let (min_ticket_price, max_ticket_price) =
        if event.publish_date.unwrap_or(times::infinity()) < dates::now().finish() || user_has_privileges {
            event.current_ticket_pricing_range(box_office_pricing, connection)?
        } else {
            (None, None)
        };
    // Show private access code to any admin with write access
    let show_private_access_code = if let Some(user) = user {
        user.has_scope_for_organization_event(Scopes::EventWrite, &organization, event.id, connection)?
    } else {
        false
    };

    let fee_in_cents = event
        .client_fee_in_cents
        .unwrap_or(organization.client_event_fee_in_cents)
        + event
            .company_fee_in_cents
            .unwrap_or(organization.company_event_fee_in_cents);
    let slug = event.slug(connection)?;

    let status = if event_ended {
        EventStatus::Closed
    } else {
        event.status.clone()
    };

    let payload = &EventShowResult {
        id: event.id,
        response_type: "Event".to_string(),
        private_access_code: if show_private_access_code {
            Some(event.private_access_code)
        } else {
            None
        },
        name: event.name,
        organization_id: event.organization_id,
        venue_id: event.venue_id,
        created_at: event.created_at,
        event_start: event.event_start,
        door_time: event.door_time,
        event_end: event.event_end,
        cancelled_at: event.cancelled_at,
        fee_in_cents,
        status,
        publish_date: event.publish_date,
        promo_image_url: optimize_cloudinary(&event.promo_image_url),
        original_promo_image_url: event.promo_image_url,
        cover_image_url: event.cover_image_url,
        additional_info: event.additional_info,
        top_line_info: event.top_line_info,
        age_limit: event.age_limit,
        video_url: event.video_url,
        organization: ShortOrganization {
            id: organization.id,
            slug: organization.slug(connection).optional()?.map(|s| s.slug),
            name: organization.name,
        },
        venue: match venue {
            Some(v) => Some(v.for_display(connection)?),
            None => None,
        },
        artists: event_artists,
        ticket_types: display_ticket_types,
        total_interest,
        user_is_interested: user_interest,
        min_ticket_price,
        max_ticket_price,
        is_external: event.is_external,
        external_url: event.external_url,
        override_status: event.override_status,
        limited_tickets_remaining,
        localized_times,
        tracking_keys,
        event_type: event.event_type,
        sales_start_date,
        url: format!("{}/tickets/{}", &state.config.front_end_url, &slug),
        slug,
        facebook_pixel_key: event.facebook_pixel_key,
        extra_admin_data: event
            .extra_admin_data
            .and_then(|data| if user_has_privileges { Some(data) } else { None }),
        facebook_event_id: event.facebook_event_id,
        updated_at: event.updated_at,
    };

    Ok(HttpResponse::Ok().json(&payload))
}

pub async fn clone(
    (connection, clone_fields, path, user, state): (
        Connection,
        Json<CloneFields>,
        Path<PathParameters>,
        AuthUser,
        Data<AppState>,
    ),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let event = Event::find(path.id, connection)?;
    let organization = event.organization(connection)?;
    user.requires_scope_for_organization_event(Scopes::EventClone, &organization, &event, connection)?;
    let event = event.clone_record(&clone_fields.into_inner(), Some(user.id()), connection)?;

    // Clone tickets on blockchain (TODO: should be moved to background job as part of ticket type create)
    let ticket_types = event.ticket_types(false, None, connection)?;
    ticket_types::create_ticket_type_blockchain_assets(&event, &ticket_types, &state, connection)?;

    // Update genres for associated event given new artists
    let action = DomainAction::create(
        None,
        DomainActionTypes::UpdateGenres,
        None,
        json!(UpdateGenresPayload { user_id: user.id() }),
        Some(Tables::Events),
        Some(event.id),
    );
    action.commit(connection)?;

    Ok(HttpResponse::Created().json(event))
}

pub async fn publish(
    (connection, path, user): (Connection, Path<PathParameters>, AuthUser),
) -> Result<HttpResponse, BigNeonError> {
    let conn = connection.get();
    let event = Event::find(path.id, conn)?;
    user.requires_scope_for_organization_event(Scopes::EventWrite, &event.organization(conn)?, &event, conn)?;
    event.publish(Some(user.id()), conn)?;

    Ok(HttpResponse::Ok().finish())
}

pub async fn unpublish(
    (connection, path, user): (Connection, Path<PathParameters>, AuthUser),
) -> Result<HttpResponse, BigNeonError> {
    let conn = connection.get();
    let event = Event::find(path.id, conn)?;
    user.requires_scope_for_organization_event(Scopes::EventWrite, &event.organization(conn)?, &event, conn)?;
    event.unpublish(Some(user.id()), conn)?;
    Ok(HttpResponse::Ok().finish())
}

pub async fn ticket_holder_count(
    (connection, path, user): (Connection, Path<PathParameters>, AuthUser),
) -> Result<HttpResponse, BigNeonError> {
    let conn = connection.get();
    let event = Event::find(path.id, conn)?;
    user.requires_scope_for_organization_event(Scopes::EventWrite, &event.organization(conn)?, &event, conn)?;
    let ticket_holders = Event::find_all_ticket_holders_count(path.id, conn, TicketHoldersCountType::WithEmailAddress)?;
    Ok(HttpResponse::Ok().json(ticket_holders))
}

#[derive(Deserialize, Serialize, Debug)]
pub struct TicketRedeemRequest {
    pub redeem_key: String,
    pub check_in_source: Option<CheckInSource>,
}

pub async fn redeem_ticket(
    (connection, parameters, redeem_parameters, auth_user, state, cache_database): (
        Connection,
        Path<PathParameters>,
        Json<TicketRedeemRequest>,
        AuthUser,
        Data<AppState>,
        CacheDatabase,
    ),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let db_event = Event::find(parameters.id, connection)?;
    let organization = db_event.organization(connection)?;
    auth_user.requires_scope_for_organization_event(Scopes::RedeemTicket, &organization, &db_event, connection)?;
    let ticket =
        TicketInstance::find_by_event_id_redeem_key(parameters.id, redeem_parameters.redeem_key.clone(), connection)?;
    let redeemable = TicketInstance::show_redeemable_ticket(ticket.id, connection)?;

    let result = TicketInstance::redeem_ticket(
        ticket.id,
        redeem_parameters.redeem_key.clone(),
        auth_user.id(),
        redeem_parameters.check_in_source.unwrap_or(CheckInSource::GuestList),
        connection,
    )?;

    match result {
        RedeemResults::TicketRedeemSuccess => {
            //Redeem ticket on chain
            let asset = Asset::find(ticket.asset_id, connection)?;
            match asset.blockchain_asset_id {
                Some(a) => {
                    let wallet = Wallet::find(ticket.wallet_id, connection)?;
                    state.config.tari_client.modify_asset_redeem_token(&wallet.secret_key, &wallet.public_key,
                                                                       &a,
                                                                       vec![ticket.token_id as u64],
                    )?;

                    //Fetch the redeemable again to include the redeemed_by and redeemed_at fields
                    let redeemable = TicketInstance::show_redeemable_ticket(ticket.id, connection)?;

                    // Publish redeem event for redis pubsub
                    cache_database
                        .inner
                        .clone()
                        .and_then(|conn| caching::publish(conn, RedisPubSubChannel::TicketRedemptions, messages::TicketRedemption {
                            ticket_id: ticket.id,
                            event_id: db_event.id,
                            redeemer_id: auth_user.id()
                        }).ok());

                    Ok(HttpResponse::Ok().json(redeemable))
                }
                None => Ok(HttpResponse::BadRequest().json(json!({ "error": "Could not redeem because the asset has not been assigned on the blockchain.".to_string()}))),
            }
        }
        RedeemResults::TicketTransferInProcess => {
            Ok(HttpResponse::BadRequest()
                .json(json!({"error": "Ticket has pending transfer in progress.".to_string()})))
        }
        RedeemResults::TicketAlreadyRedeemed => Ok(HttpResponse::Conflict().json(json!({
        "error": "Ticket has already been redeemed.".to_string(),
        "redeemed_by": redeemable.redeemed_by,
        "redeemed_at": redeemable.redeemed_at
        }))),
        RedeemResults::TicketInvalid => {
            Ok(HttpResponse::BadRequest().json(json!({"error": "Ticket is invalid.".to_string()})))
        }
    }
}

pub async fn show_from_organizations(
    (connection, path, paging, user): (Connection, Path<PathParameters>, Query<PagingParameters>, AuthUser),
) -> Result<WebPayload<EventSummaryResult>, BigNeonError> {
    let conn = connection.get();
    let org = Organization::find(path.id, conn)?;
    user.requires_scope_for_organization(Scopes::OrgReadEvents, &org, conn)?;

    let user_roles = org.get_roles_for_user(&user.user, conn)?;
    let mut events = Event::find_all_events_for_organization(
        path.id,
        Some(
            paging
                .get_tag("past_or_upcoming")
                .unwrap_or_else(|| "Upcoming".to_string())
                .parse()?,
        ),
        if Roles::get_event_limited_roles()
            .iter()
            .find(|r| user_roles.contains(&r))
            .is_some()
        {
            Some(user.user.get_event_ids_for_organization(org.id, conn)?)
        } else {
            None
        },
        false,
        paging.page(),
        paging.limit(),
        conn,
    )?;
    for event in events.data.iter_mut() {
        if !user.has_scope_for_organization_event(Scopes::EventDelete, &org, event.id, conn)? {
            event.eligible_for_deletion = None;
        }

        if !user.has_scope_for_organization_event(Scopes::DashboardRead, &org, event.id, conn)? {
            event.sales_total_in_cents = None;
            event.sold_held = None;
            event.sold_unreserved = None;
            for tt in event.ticket_types.iter_mut() {
                tt.sold_unreserved = None;
                tt.sold_held = None;
                tt.sales_total_in_cents = None;
            }
        }
    }
    Ok(WebPayload::new(StatusCode::OK, events))
}

#[derive(Deserialize)]
pub struct DashboardParameters {
    start_utc: Option<NaiveDate>,
    // Defaults to 29 days ago if not provided
    end_utc: Option<NaiveDate>, // Defaults to today if not provided
}

#[derive(Deserialize, Serialize)]
pub struct DashboardResult {
    pub event: EventSummaryResult,
    pub day_stats: Vec<DayStats>,
    pub cube_js_token: String,
}

pub async fn dashboard(
    (state, connection, path, query, user): (
        Data<AppState>,
        Connection,
        Path<PathParameters>,
        Query<DashboardParameters>,
        AuthUser,
    ),
) -> Result<HttpResponse, BigNeonError> {
    let conn = connection.get();
    let event = Event::find(path.id, conn)?;
    user.requires_scope_for_organization_event(Scopes::DashboardRead, &event.organization(conn)?, &event, conn)?;
    let summary = event.summary(conn)?;
    let end_utc = match query.end_utc {
        Some(end_utc) => end_utc,
        None => std::cmp::min(
            Utc::now().naive_utc().date(),
            event.event_end.unwrap_or(Utc::now().naive_utc()).date(),
        ),
    };

    let start_utc = query.start_utc.unwrap_or(end_utc - Duration::days(29));

    let day_stats = event.get_sales_by_date_range(start_utc, end_utc, conn)?;

    let cube_js_token = create_cube_js_token(event.id, &state.config.cube_js.secret)?;
    Ok(HttpResponse::Ok().json(DashboardResult {
        event: summary,
        day_stats,
        cube_js_token,
    }))
}

fn create_cube_js_token(event_id: Uuid, cube_js_secret: &str) -> Result<String, BigNeonError> {
    #[derive(Debug, Serialize, Deserialize)]
    struct Claims {
        u: UserData,
        iat: i64,
        exp: i64,
    };

    #[derive(Debug, Serialize, Deserialize)]
    struct UserData {
        event_id: Uuid,
    }
    let claims = Claims {
        u: UserData { event_id },
        iat: Utc::now().timestamp(),
        exp: dates::now().add_days(30).finish().timestamp(),
    };

    println!("Secret {}", cube_js_secret);
    let token = encode(&Header::default(), &claims, cube_js_secret.as_ref())?;
    Ok(token)
}

#[derive(Deserialize, Debug)]
pub struct AddArtistRequest {
    pub artist_id: Uuid,
    pub rank: i32,
    pub set_time: Option<NaiveDateTime>,
    pub importance: i32,
    pub stage_id: Option<Uuid>,
}

pub async fn create(
    (connection, new_event, user): (Connection, Json<NewEvent>, AuthUser),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let organization = Organization::find(new_event.organization_id, connection)?;
    user.requires_scope_for_organization(Scopes::EventWrite, &organization, connection)?;

    let new_event = new_event.into_inner();

    let event = new_event.commit(Some(user.id()), connection)?;

    create_domain_action_event(event.id, connection);
    Ok(HttpResponse::Created().json(event))
}

pub async fn update(
    (connection, parameters, event_parameters, user): (
        Connection,
        Path<PathParameters>,
        Json<EventEditableAttributes>,
        AuthUser,
    ),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let event = Event::find(parameters.id, connection)?;
    let organization = event.organization(connection)?;
    user.requires_scope_for_organization_event(Scopes::EventWrite, &organization, &event, connection)?;

    let event_parameters = event_parameters.into_inner();
    // Not sure about this at this time, we don't want to have 404's on the old URL
    //       if let Some(ref name) = event_parameters.name {
    //        event_parameters.slug = Some(event_parameters.slug.unwrap_or(create_slug(name)));
    //    }

    let updated_event = event.update(Some(user.id()), event_parameters, connection)?;

    create_domain_action_event(updated_event.id, connection);

    Ok(HttpResponse::Ok().json(&updated_event))
}

fn create_domain_action_event(event_id: Uuid, conn: &PgConnection) {
    let domain_action = DomainAction::create(
        None,
        DomainActionTypes::SubmitSitemapToSearchEngines,
        None,
        json!({}),
        Some(Tables::Events),
        Some(event_id),
    );
    domain_action.commit(conn).unwrap();
}

pub async fn delete(
    (connection, parameters, user): (Connection, Path<PathParameters>, AuthUser),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let event = Event::find(parameters.id, connection)?;
    let organization = event.organization(connection)?;
    user.requires_scope_for_organization_event(Scopes::EventDelete, &organization, &event, connection)?;

    event.delete(user.id(), connection)?;
    Ok(HttpResponse::Ok().json({}))
}

pub async fn cancel(
    (connection, parameters, user): (Connection, Path<PathParameters>, AuthUser),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let event = Event::find(parameters.id, connection)?;
    let organization = event.organization(connection)?;
    user.requires_scope_for_organization_event(Scopes::EventCancel, &organization, &event, connection)?;

    //Doing this in the DB layer so it can use the DB time as now.
    let updated_event = event.cancel(Some(user.id()), connection)?;

    Ok(HttpResponse::Ok().json(&updated_event))
}

pub async fn list_interested_users(
    (connection, path_parameters, query, user): (Connection, Path<PathParameters>, Query<PagingParameters>, AuthUser),
) -> Result<HttpResponse, BigNeonError> {
    user.requires_scope(Scopes::EventInterest)?;

    let connection = connection.get();
    let paging: Paging = query.clone().into();

    let payload = EventInterest::list_interested_users(
        path_parameters.id,
        user.id(),
        paging.page * paging.limit,
        (paging.page * paging.limit) + paging.limit,
        connection,
    )?;
    Ok(HttpResponse::Ok().json(&payload))
}

pub async fn add_interest(
    (connection, parameters, user): (Connection, Path<PathParameters>, AuthUser),
) -> Result<HttpResponse, BigNeonError> {
    user.requires_scope(Scopes::EventInterest)?;

    let connection = connection.get();
    let event_interest = EventInterest::create(parameters.id, user.id()).commit(connection)?;
    Ok(HttpResponse::Created().json(&event_interest))
}

pub async fn remove_interest(
    (connection, parameters, user): (Connection, Path<PathParameters>, AuthUser),
) -> Result<HttpResponse, BigNeonError> {
    user.requires_scope(Scopes::EventInterest)?;

    let connection = connection.get();
    let event_interest = EventInterest::remove(parameters.id, user.id(), connection)?;
    Ok(HttpResponse::Ok().json(&event_interest))
}

pub async fn add_artist(
    (connection, parameters, event_artist, user): (Connection, Path<PathParameters>, Json<AddArtistRequest>, AuthUser),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let event = Event::find(parameters.id, connection)?;
    let organization = event.organization(connection)?;
    user.requires_scope_for_organization_event(Scopes::EventWrite, &organization, &event, connection)?;

    let event_artist = EventArtist::create(
        parameters.id,
        event_artist.artist_id,
        event_artist.rank,
        event_artist.set_time,
        event_artist.importance,
        event_artist.stage_id,
    )
    .commit(Some(user.id()), connection)?;

    // Trigger update for event and associated users in background
    let action = DomainAction::create(
        None,
        DomainActionTypes::UpdateGenres,
        None,
        json!(UpdateGenresPayload { user_id: user.id() }),
        Some(Tables::Events),
        Some(event.id),
    );
    action.commit(connection)?;

    Ok(HttpResponse::Created().json(&event_artist))
}

#[derive(Deserialize, Debug, Default)]
pub struct UpdateArtistsRequest {
    pub artist_id: Uuid,
    pub set_time: Option<NaiveDateTime>,
    pub importance: i32,
    pub stage_id: Option<Uuid>,
}

#[derive(Deserialize, Debug, Default)]
pub struct UpdateArtistsRequestList {
    pub artists: Vec<UpdateArtistsRequest>,
}

pub async fn update_artists(
    (connection, parameters, artists, user): (
        Connection,
        Path<PathParameters>,
        Json<UpdateArtistsRequestList>,
        AuthUser,
    ),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let event = Event::find(parameters.id, connection)?;
    let organization = event.organization(connection)?;
    user.requires_scope_for_organization_event(Scopes::EventWrite, &organization, &event, connection)?;

    EventArtist::clear_all_from_event(parameters.id, connection)?;

    let mut rank = 0;
    let mut added_artists: Vec<EventArtist> = Vec::new();

    for a in &artists.into_inner().artists {
        added_artists.push(
            EventArtist::create(parameters.id, a.artist_id, rank, a.set_time, a.importance, a.stage_id)
                .commit(Some(user.id()), connection)?,
        );
        rank += 1;
    }

    let action = DomainAction::create(
        None,
        DomainActionTypes::UpdateGenres,
        None,
        json!(UpdateGenresPayload { user_id: user.id() }),
        Some(Tables::Events),
        Some(event.id),
    );
    action.commit(connection)?;

    Ok(HttpResponse::Ok().json(&added_artists))
}

#[derive(Deserialize, Clone)]
pub struct GuestListQueryParameters {
    pub changes_since: Option<NaiveDateTime>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub query: Option<String>,
    pub page: Option<u32>,
    pub limit: Option<i32>,
}

impl From<GuestListQueryParameters> for Paging {
    fn from(s: GuestListQueryParameters) -> Paging {
        let mut default_tags: HashMap<String, Value> = HashMap::new();
        default_tags.insert("query".to_owned(), json!(s.query.clone()));
        default_tags.insert("changes_since".to_owned(), json!(s.changes_since.clone()));

        //TODO Replace u32::MAX with our default of 100
        let limit: u32 = match s.limit {
            Some(limit) => {
                if limit > 0 {
                    limit as u32
                } else {
                    std::u32::MAX
                }
            }
            None => std::u32::MAX,
        };

        PagingParameters {
            page: s.page,
            limit: Some(limit),
            sort: None,
            dir: None,
            tags: default_tags,
        }
        .into()
    }
}

pub async fn guest_list(
    (connection, query, path, user): (
        Connection,
        Query<GuestListQueryParameters>,
        Path<PathParameters>,
        AuthUser,
    ),
) -> Result<HttpResponse, BigNeonError> {
    //TODO refactor GuestListQueryParameters to PagingParameters
    let conn = connection.get();
    let event = Event::find(path.id, conn)?;
    user.requires_scope_for_organization_event(Scopes::EventViewGuests, &event.organization(conn)?, &event, conn)?;

    let query_string = query.clone().query;
    let changes_since = query.clone().changes_since;
    let paging = query.clone().into();
    let tickets_and_total = event.guest_list(query_string, &changes_since, Some(&paging), conn)?;
    let (tickets, total) = tickets_and_total;

    #[derive(Serialize)]
    struct TicketRefundable {
        #[serde(flatten)]
        ticket: RedeemableTicket,
        #[serde(flatten)]
        pending_transfer: PendingTransfer,
        refund_supported: bool,
    }

    let mut tickets_refund: Vec<TicketRefundable> = Vec::new();

    for t in tickets {
        let mut refundable = t.providers.len() != 0;
        for p in t.providers {
            if !ServiceLocator::is_refund_supported(p) {
                refundable = false;
            }
        }

        tickets_refund.push(TicketRefundable {
            ticket: t.ticket.clone(),
            pending_transfer: t
                .pending_transfer
                .clone()
                .unwrap_or(PendingTransfer { ..Default::default() }),
            refund_supported: refundable,
        });
    }

    let mut payload = Payload::new(tickets_refund, query.into_inner().into());
    payload.paging.total = total as u64;
    payload.paging.limit = paging.limit;
    Ok(HttpResponse::Ok().json(payload))
}

pub async fn codes(
    (conn, query, path, user): (Connection, Query<PagingParameters>, Path<PathParameters>, AuthUser),
) -> Result<HttpResponse, BigNeonError> {
    let conn = conn.get();
    let event = Event::find(path.id, conn)?;
    user.requires_scope_for_organization_event(Scopes::CodeRead, &event.organization(conn)?, &event, conn)?;

    let mut code_type: Option<CodeTypes> = None;
    if let Some(value) = query.tags.get("type") {
        code_type = serde_json::from_value(value.clone())?;
    }

    //TODO: remap query to use paging info
    let codes = Code::find_for_event(path.id, code_type, conn)?;
    let mut payload = Payload::from_data(codes, query.page(), query.limit(), None);
    payload.paging.tags = query.tags.clone();

    Ok(HttpResponse::Ok().json(payload))
}

pub async fn holds(
    (conn, query, path, user): (Connection, Query<PagingParameters>, Path<PathParameters>, AuthUser),
) -> Result<HttpResponse, BigNeonError> {
    let conn = conn.get();
    let event = Event::find(path.id, conn)?;
    let organization = &event.organization(conn)?;
    user.requires_scope_for_organization_event(Scopes::HoldRead, &organization, &event, conn)?;
    let holds = Hold::find_for_event(path.id, false, conn)?;
    let mut ticket_type_ids: Vec<Uuid> = holds.iter().map(|h| h.ticket_type_id).collect();
    ticket_type_ids.sort();
    ticket_type_ids.dedup();
    let ticket_types = TicketType::find_by_ids(&ticket_type_ids, conn)?;
    let mut ticket_types_map = HashMap::new();
    for ticket_type in ticket_types {
        ticket_types_map.insert(
            ticket_type.id,
            (
                ticket_type.clone(),
                ticket_type.current_ticket_pricing(false, conn).optional()?,
            ),
        );
    }

    #[derive(Serialize)]
    struct R {
        pub id: Uuid,
        pub name: String,
        pub event_id: Uuid,
        pub redemption_code: Option<String>,
        pub discount_in_cents: Option<i64>,
        pub end_at: Option<NaiveDateTime>,
        pub max_per_user: Option<i64>,
        pub hold_type: HoldTypes,
        pub ticket_type_id: Uuid,
        pub ticket_type_name: String,
        pub price_in_cents: Option<u32>,
        pub available: u32,
        pub quantity: u32,
        pub children_available: u32,
        pub children_quantity: u32,
        pub parent_hold_id: Option<Uuid>,
    }

    let mut list = Vec::<R>::new();
    for hold in holds {
        let (quantity, available) = hold.quantity(conn)?;
        let (children_quantity, children_available) = hold.children_quantity(conn)?;
        let (ticket_type, current_ticket_pricing) = ticket_types_map
            .get(&hold.ticket_type_id)
            .ok_or_else(|| ApplicationError::new("Failed to load hold ticket type".to_string()))?;
        let r = R {
            id: hold.id,
            name: hold.name,
            event_id: hold.event_id,
            redemption_code: hold.redemption_code,
            discount_in_cents: hold.discount_in_cents,
            end_at: hold.end_at,
            max_per_user: hold.max_per_user,
            hold_type: hold.hold_type,
            ticket_type_id: hold.ticket_type_id,
            ticket_type_name: ticket_type.name.clone(),
            price_in_cents: current_ticket_pricing.clone().map(|tp| tp.price_in_cents as u32),
            available,
            quantity,
            children_available,
            children_quantity,
            parent_hold_id: hold.parent_hold_id,
        };

        list.push(r);
    }

    Ok(HttpResponse::Ok().json(Payload::from_data(list, query.page(), query.limit(), None)))
}

pub async fn users(
    (connection, path_parameters, query_parameters, user): (
        Connection,
        Path<PathParameters>,
        Query<PagingParameters>,
        AuthUser,
    ),
) -> Result<WebPayload<DisplayOrganizationUser>, BigNeonError> {
    let connection = connection.get();
    let event = Event::find(path_parameters.id, connection)?;
    let organization = event.organization(connection)?;
    user.requires_scope_for_organization_event(Scopes::OrgRead, &organization, &event, connection)?;

    let mut members: Vec<DisplayOrganizationUser> = organization
        .users(Some(event.id), connection)?
        .into_iter()
        .map(|u| DisplayOrganizationUser {
            user_id: Some(u.1.id),
            first_name: u.1.first_name,
            last_name: u.1.last_name,
            email: u.1.email,
            roles: u.0.role,
            invite_or_member: "member".to_string(),
            invite_id: None,
        })
        .collect();

    for inv in organization.pending_invites(Some(event.id), connection)? {
        members.push(DisplayOrganizationUser {
            user_id: inv.user_id,
            first_name: None,
            last_name: None,
            email: Some(inv.user_email),
            roles: inv.roles,
            invite_or_member: "invite".to_string(),
            invite_id: Some(inv.id),
        });
    }

    let payload = Payload::from_data(members, query_parameters.page(), query_parameters.limit(), None);
    Ok(WebPayload::new(StatusCode::OK, payload))
}

#[derive(Deserialize)]
pub struct EventUserPathParams {
    id: Uuid,
    user_id: Uuid,
}

pub async fn remove_user(
    (connection, path, user): (Connection, Path<EventUserPathParams>, AuthUser),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let event = Event::find(path.id, connection)?;
    let organization = event.organization(connection)?;
    user.requires_scope_for_organization_event(Scopes::OrgUsers, &organization, &event, connection)?;

    let event_user = EventUser::find_by_event_id_user_id(event.id, path.user_id, connection).optional()?;
    match event_user {
        Some(event_user) => {
            event_user.destroy(connection)?;
            Ok(HttpResponse::Ok().json(&organization))
        }
        None => Ok(HttpResponse::Ok().finish()),
    }
}

#[derive(Deserialize)]
pub struct LinkQueryParameters {
    source: Option<String>,
    medium: Option<String>,
    campaign: Option<String>,
    term: Option<String>,
    content: Option<String>,
}

#[derive(Serialize)]
pub struct LinkResult {
    pub link: String,
    pub long_link: String,
}

pub async fn create_link(
    (path, query, state, user, conn): (
        Path<PathParameters>,
        Json<LinkQueryParameters>,
        Data<AppState>,
        AuthUser,
        Connection,
    ),
) -> Result<HttpResponse, BigNeonError> {
    let conn = conn.get();
    let event = Event::find(path.id, conn)?;

    user.requires_scope_for_organization_event(Scopes::EventWrite, &event.organization(conn)?, &event, conn)?;

    let query = query.into_inner();
    let slug = event.slug(conn).unwrap_or(path.id.to_string());
    let long_link_raw = format!(
        "{}/tickets/{}?utm_source={}&utm_medium={}&utm_campaign={}&utm_term={}&utm_content={}",
        state.config.front_end_url,
        slug,
        query.source.as_ref().unwrap_or(&"".to_string()),
        query.medium.as_ref().unwrap_or(&"".to_string()),
        query.campaign.as_ref().unwrap_or(&"".to_string()),
        query.term.as_ref().unwrap_or(&"".to_string()),
        query.content.as_ref().unwrap_or(&"".to_string())
    );
    let long_link_url = Url::parse(long_link_raw.as_str())?;

    let deep_linker = state.service_locator.create_deep_linker()?;
    let short_link = deep_linker.create_deep_link_with_fallback(&long_link_raw);
    Ok(HttpResponse::Ok().json(LinkResult {
        link: short_link,
        long_link: long_link_url.as_str().to_string(),
    }))
}
