use bigneon_db::prelude::*;
use chrono::NaiveDateTime;
use crate::models::UserDisplayTicketType;
use serde_json::Value;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub struct EventShowResult {
    pub id: Uuid,
    #[serde(rename = "type")]
    pub response_type: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_access_code: Option<Option<String>>,
    pub organization_id: Uuid,
    pub venue_id: Option<Uuid>,
    pub created_at: NaiveDateTime,
    pub event_start: Option<NaiveDateTime>,
    pub door_time: Option<NaiveDateTime>,
    pub event_end: Option<NaiveDateTime>,
    pub cancelled_at: Option<NaiveDateTime>,
    pub fee_in_cents: i64,
    pub status: EventStatus,
    pub publish_date: Option<NaiveDateTime>,
    pub promo_image_url: Option<String>,
    pub original_promo_image_url: Option<String>,
    pub cover_image_url: Option<String>,
    pub additional_info: Option<String>,
    pub top_line_info: Option<String>,
    pub age_limit: Option<String>,
    pub video_url: Option<String>,
    pub organization: ShortOrganization,
    pub venue: Option<DisplayVenue>,
    pub artists: Vec<DisplayEventArtist>,
    pub ticket_types: Vec<UserDisplayTicketType>,
    pub total_interest: u32,
    pub user_is_interested: bool,
    pub min_ticket_price: Option<i64>,
    pub max_ticket_price: Option<i64>,
    pub is_external: bool,
    pub external_url: Option<String>,
    pub override_status: Option<EventOverrideStatus>,
    pub limited_tickets_remaining: Vec<TicketsRemaining>,
    pub localized_times: EventLocalizedTimeStrings,
    pub tracking_keys: TrackingKeys,
    pub event_type: EventTypes,
    pub sales_start_date: Option<NaiveDateTime>,
    pub url: String,
    pub slug: String,
    pub facebook_pixel_key: Option<String>,
    pub extra_admin_data: Option<Value>,
    pub facebook_event_id: Option<String>,
    pub updated_at: NaiveDateTime,
}

//This struct is used to just contain the id and name of the org
#[derive(Serialize, Deserialize, Debug)]
pub struct ShortOrganization {
    pub id: Uuid,
    pub name: String,
    pub slug: Option<String>,
}
