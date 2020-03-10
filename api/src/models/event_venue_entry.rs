use crate::server::AppState;
use crate::utils::cloudinary::optimize_cloudinary;
use actix_web::web::Data;
use bigneon_db::prelude::*;
use chrono::prelude::*;
use diesel::PgConnection;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, PartialEq, Serialize)]
pub struct EventVenueEntry {
    pub id: Uuid,
    pub name: String,
    pub organization_id: Uuid,
    pub venue_id: Option<Uuid>,
    pub created_at: NaiveDateTime,
    pub event_start: Option<NaiveDateTime>,
    pub door_time: Option<NaiveDateTime>,
    pub status: EventStatus,
    pub publish_date: Option<NaiveDateTime>,
    pub promo_image_url: Option<String>,
    pub original_promo_image_url: Option<String>,
    pub additional_info: Option<String>,
    pub top_line_info: Option<String>,
    pub age_limit: Option<String>,
    pub cancelled_at: Option<NaiveDateTime>,
    pub venue: Option<Venue>,
    pub artists: Option<Vec<DisplayEventArtist>>,
    pub min_ticket_price: Option<i64>,
    pub max_ticket_price: Option<i64>,
    pub is_external: bool,
    pub external_url: Option<String>,
    pub user_is_interested: bool,
    pub localized_times: EventLocalizedTimeStrings,
    pub tracking_keys: TrackingKeys,
    pub event_type: EventTypes,
    pub updated_at: NaiveDateTime,
    pub slug: String,
    pub url: String,
    pub event_end: Option<NaiveDateTime>,
}

impl EventVenueEntry {
    pub fn event_venues_from_events(
        events: Vec<Event>,
        user: Option<User>,
        state: &Data<AppState>,
        connection: &PgConnection,
    ) -> Result<Vec<EventVenueEntry>, DatabaseError> {
        let event_ids: Vec<Uuid> = events.iter().map(|e| e.id).collect();

        let slugs = Slug::load_primary_slugs(&event_ids, Tables::Events, connection)?;
        let mut slug_map = slugs.into_iter().fold(HashMap::new(), |mut map, s| {
            map.insert(s.main_table_id, s.slug);
            map
        });

        let mut venue_ids: Vec<Uuid> = events
            .iter()
            .filter(|e| e.venue_id.is_some())
            .map(|e| e.venue_id.unwrap())
            .collect();
        venue_ids.sort();
        venue_ids.dedup();

        let event_ticket_range_mapping = Event::ticket_pricing_range_by_events(&event_ids, false, connection)?;

        let venues = Venue::find_by_ids(venue_ids, connection)?;
        let venue_map = venues.into_iter().fold(HashMap::new(), |mut map, v| {
            map.insert(v.id, v.clone());
            map
        });

        let mut artists_map = EventArtist::find_all_from_events(&event_ids, connection)?;

        let mut organization_ids: Vec<Uuid> = events.iter().map(|e| e.organization_id).collect();
        organization_ids.sort();
        organization_ids.dedup();

        let tracking_keys_for_orgs =
            Organization::tracking_keys_for_ids(organization_ids, &state.config.api_keys_encryption_key, connection)?;

        let event_interest = match user {
            Some(ref u) => EventInterest::find_interest_by_event_ids_for_user(&event_ids, u.id, connection)?,
            None => HashMap::new(),
        };

        let mut results: Vec<EventVenueEntry> = Vec::new();

        for event in events.into_iter() {
            let venue = event.venue_id.and_then(|v| Some(venue_map[&v].clone()));
            let artists = artists_map.remove(&event.id).map_or(Vec::new(), |x| x);
            let mut min_ticket_price = None;
            let mut max_ticket_price = None;
            if let Some((min, max)) = event_ticket_range_mapping.get(&event.id) {
                min_ticket_price = Some(*min);
                max_ticket_price = Some(*max);
            }

            let localized_times = event.get_all_localized_time_strings(venue.as_ref());
            let organization_id = event.organization_id;
            let tracking_keys = tracking_keys_for_orgs
                .get(&organization_id)
                .unwrap_or(&TrackingKeys { ..Default::default() })
                .clone();
            let slug = slug_map.remove(&event.id).unwrap_or("".to_string());

            results.push(EventVenueEntry {
                venue,
                artists: Some(artists),
                id: event.id,
                name: event.name,
                organization_id,
                venue_id: event.venue_id,
                created_at: event.created_at,
                updated_at: event.updated_at,
                slug: slug.clone(),
                event_start: event.event_start,
                door_time: event.door_time,
                status: event.status,
                publish_date: event.publish_date,
                promo_image_url: optimize_cloudinary(&event.promo_image_url),
                original_promo_image_url: event.promo_image_url,
                additional_info: event.additional_info,
                top_line_info: event.top_line_info,
                age_limit: event.age_limit,
                cancelled_at: event.cancelled_at,
                min_ticket_price,
                max_ticket_price,
                is_external: event.is_external,
                external_url: event.external_url,
                user_is_interested: event_interest.get(&event.id).map(|i| i.to_owned()).unwrap_or(false),
                localized_times,
                tracking_keys,
                event_type: event.event_type,
                url: format!("{}/tickets/{}", state.config.front_end_url, slug),
                event_end: event.event_end,
            });
        }
        Ok(results)
    }
}
