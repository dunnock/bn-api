pub use self::activities::*;
pub use self::announcement_engagements::*;
pub use self::announcements::*;
pub use self::artists::*;
pub use self::assets::*;
pub use self::auth::*;
pub use self::broadcasts::*;
pub use self::codes::*;
pub use self::communication::*;
pub use self::domain_actions::*;
pub use self::domain_event_publishers::*;
pub use self::domain_events::*;
pub use self::enums::*;
pub use self::event_artists::*;
pub use self::event_interest::*;
pub use self::event_report_subscribers::*;
pub use self::event_users::*;
pub use self::events::*;
pub use self::external_logins::FACEBOOK_SITE;
pub use self::external_logins::*;
pub use self::fans::*;
pub use self::fee_schedule_ranges::*;
pub use self::fee_schedules::*;
pub use self::for_display::*;
pub use self::genres::*;
pub use self::global::*;
pub use self::history_item::*;
pub use self::holds::*;
pub use self::notes::*;
pub use self::order_items::*;
pub use self::orders::*;
pub use self::organization_interactions::*;
pub use self::organization_invites::*;
pub use self::organization_users::*;
pub use self::organization_venues::*;
pub use self::organizations::*;
pub use self::paging::*;
pub use self::payment_methods::*;
pub use self::payments::*;
pub use self::platforms::*;
pub use self::push_notification_tokens::*;
pub use self::redeemable_ticket::*;
pub use self::refund_items::*;
pub use self::refunded_tickets::*;
pub use self::refunds::*;
pub use self::regions::*;
pub use self::reports::*;
pub use self::scopes::*;
pub use self::settlement_adjustments::*;
pub use self::settlement_entries::*;
pub use self::settlements::*;
pub use self::slugs::*;
pub use self::stages::*;
pub use self::temporary_users::*;
pub use self::ticket_instances::RedeemResults;
pub use self::ticket_instances::*;
pub use self::ticket_pricing::*;
pub use self::ticket_type_codes::*;
pub use self::ticket_types::*;
pub use self::transfer_tickets::*;
pub use self::transfers::*;
pub use self::users::*;
pub use self::venues::*;
pub use self::wallets::*;

use serde::{Deserialize, Deserializer};
use serde_json::Value;

pub mod concerns;

mod activities;
pub mod analytics;
mod announcement_engagements;
mod announcements;
mod artists;
mod assets;
mod auth;
mod broadcasts;
mod codes;
mod communication;
mod domain_actions;
mod domain_event_publishers;
mod domain_events;
pub mod enums;
mod event_artists;
mod event_interest;
mod event_report_subscribers;
mod event_users;
mod events;
mod external_logins;
mod fans;
mod fee_schedule_ranges;
mod fee_schedules;
mod for_display;
mod genres;
pub mod global;
mod history_item;
mod holds;
mod notes;
mod order_items;
mod orders;
mod organization_interactions;
mod organization_invites;
mod organization_users;
mod organization_venues;
mod organizations;
mod paging;
mod payment_methods;
mod payments;
mod platforms;
mod push_notification_tokens;
mod redeemable_ticket;
mod refund_items;
mod refunded_tickets;
mod refunds;
mod regions;
mod reports;
pub mod scopes;
mod settlement_adjustments;
mod settlement_entries;
mod settlements;
mod slugs;
mod stages;
mod temporary_users;
mod ticket_instances;
mod ticket_pricing;
mod ticket_type_codes;
mod ticket_types;
mod transfer_tickets;
mod transfers;
mod users;
mod venues;
mod wallets;

pub fn deserialize_unless_blank<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    let value: Value = Deserialize::deserialize(deserializer)?;
    if value.as_str().map_or(false, |v| !v.is_empty()) {
        Ok(T::deserialize(value).ok())
    } else {
        Ok(None)
    }
}

pub fn double_option_deserialize_unless_blank<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    let value: Value = Deserialize::deserialize(deserializer)?;

    if value.is_null() {
        Ok(T::deserialize(Value::Null).ok())
    } else {
        if value.as_str().map_or(false, |v| !v.is_empty()) {
            Ok(T::deserialize(value).ok())
        } else {
            Ok(T::deserialize(Value::Null).ok())
        }
    }
}

pub fn from_str_or_num_to_str<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value: Value = Deserialize::deserialize(deserializer)?;

    if value.is_string() {
        Ok(Some(String::from(value.as_str().unwrap_or(""))))
    } else if value.is_number() {
        Ok(Some(String::from(value.as_f64().unwrap_or(0f64).to_string())))
    } else {
        Ok(None)
    }
}

#[test]
fn from_str_or_num_to_str_properly_deserializes() {
    let event_data = r#"{"age_limit": ""}"#;
    let event: EventEditableAttributes = serde_json::from_str(&event_data).unwrap();
    assert_eq!(event.age_limit, Some("".to_string()));

    let event_data = r#"{}"#;
    let event: EventEditableAttributes = serde_json::from_str(&event_data).unwrap();
    assert_eq!(event.age_limit, None);

    let event_data = r#"{"age_limit": "1"}"#;
    let event: EventEditableAttributes = serde_json::from_str(&event_data).unwrap();
    assert_eq!(event.age_limit, Some("1".to_string()));

    let event_data = r#"{"age_limit": 1}"#;
    let event: EventEditableAttributes = serde_json::from_str(&event_data).unwrap();
    assert_eq!(event.age_limit, Some("1".to_string()));
}

#[test]
fn double_option_deserialize_unless_blank_properly_deserializes() {
    let event_data = r#"{"name": "Event"}"#;
    let event: EventEditableAttributes = serde_json::from_str(&event_data).unwrap();
    assert_eq!(event.name, Some("Event".to_string()));
    assert_eq!(event.top_line_info, None);

    let event_data = r#"{"name": "Event", "top_line_info": null}"#;
    let event: EventEditableAttributes = serde_json::from_str(&event_data).unwrap();
    assert_eq!(event.name, Some("Event".to_string()));
    assert_eq!(event.top_line_info, Some(None));

    let event_data = r#"{"name": "Event", "top_line_info": ""}"#;
    let event: EventEditableAttributes = serde_json::from_str(&event_data).unwrap();
    assert_eq!(event.name, Some("Event".to_string()));
    assert_eq!(event.top_line_info, Some(None));

    let event_data = r#"{"name": "Event", "top_line_info": "Top line info"}"#;
    let event: EventEditableAttributes = serde_json::from_str(&event_data).unwrap();
    assert_eq!(event.name, Some("Event".to_string()));
    assert_eq!(event.top_line_info, Some(Some("Top line info".to_string())));
}

#[test]
fn deserialize_unless_blank_properly_deserializes() {
    let venue_data = r#"{"name": "Venue"}"#;
    let venue: VenueEditableAttributes = serde_json::from_str(&venue_data).unwrap();
    assert_eq!(venue.name, Some("Venue".to_string()));
    assert_eq!(venue.city, None);
    assert_eq!(venue.state, None);
    assert_eq!(venue.address, None);
    assert_eq!(venue.country, None);
    assert_eq!(venue.postal_code, None);

    let venue_data = r#"{
        "name": "Venue",
        "city": null,
        "state": null,
        "address": null,
        "country": null,
        "postal_code": null
    }"#;
    let venue: VenueEditableAttributes = serde_json::from_str(&venue_data).unwrap();
    assert_eq!(venue.name, Some("Venue".to_string()));
    assert_eq!(venue.city, None);
    assert_eq!(venue.state, None);
    assert_eq!(venue.address, None);
    assert_eq!(venue.country, None);
    assert_eq!(venue.postal_code, None);

    let venue_data = r#"{
        "name": "Venue",
        "city": "",
        "state": "",
        "address": "",
        "country": "",
        "postal_code": ""
    }"#;
    let venue: VenueEditableAttributes = serde_json::from_str(&venue_data).unwrap();
    assert_eq!(venue.name, Some("Venue".to_string()));
    assert_eq!(venue.city, None);
    assert_eq!(venue.state, None);
    assert_eq!(venue.address, None);
    assert_eq!(venue.country, None);
    assert_eq!(venue.postal_code, None);

    let venue_data = r#"{
        "name": "Venue",
        "city": "Springfield",
        "state": "MA",
        "address": "111 Main Street",
        "country": "US",
        "postal_code": "01103"
    }"#;
    let venue: VenueEditableAttributes = serde_json::from_str(&venue_data).unwrap();
    assert_eq!(venue.name, Some("Venue".to_string()));
    assert_eq!(venue.city, Some("Springfield".to_string()));
    assert_eq!(venue.state, Some("MA".to_string()));
    assert_eq!(venue.address, Some("111 Main Street".to_string()));
    assert_eq!(venue.country, Some("US".to_string()));
    assert_eq!(venue.postal_code, Some("01103".to_string()));
}
