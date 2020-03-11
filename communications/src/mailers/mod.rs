use crate::errors::BigNeonError;
use bigneon_db::models::*;
use diesel::PgConnection;
use url::form_urlencoded::byte_serialize;

pub mod orders;
pub mod organization_invites;
pub mod reports;
pub mod tickets;
pub mod user;

pub fn insert_event_template_data(
    template_data: &mut TemplateData,
    event: &Event,
    conn: &PgConnection,
) -> Result<(), BigNeonError> {
    let organization = event.organization(conn)?;
    let venue = event.venue(conn)?;
    let localized_times = event.get_all_localized_times(venue.as_ref());
    let mut artists: Vec<DisplayEventArtist> = event.artists(conn)?;
    artists.sort_by_key(|a| a.rank);
    template_data.insert("event_id".to_string(), event.id.to_string());
    template_data.insert("event_name".to_string(), event.name.clone());
    template_data.insert(
        "event_promo_url".to_string(),
        event.promo_image_url.clone().unwrap_or("".to_string()),
    );
    template_data.insert(
        "event_age_limit".to_string(),
        event.age_limit.clone().unwrap_or("This event is all ages".to_string()),
    );
    if let Some(event_start) = localized_times.event_start {
        template_data.insert(
            "event_date".to_string(),
            format!(
                "{} {}",
                event_start.format("%A,"),
                event_start.format("%e %B %Y").to_string().trim()
            )
            .to_string(),
        );
        template_data.insert(
            "event_event_start".to_string(),
            event_start.format("%l:%M %p %Z").to_string().trim().to_string(),
        );
    }

    if let Some(door_time) = localized_times.door_time {
        template_data.insert(
            "event_doors_open_time".to_string(),
            door_time.format("%l:%M %p %Z").to_string().trim().to_string(),
        );
    }

    let artist_headliners: Vec<String> = artists
        .iter()
        .filter(|a| a.importance == 0)
        .map(|a| a.artist.name.clone())
        .collect();
    let artist_other: Vec<String> = artists
        .iter()
        .filter(|a| a.importance != 0)
        .map(|a| a.artist.name.clone())
        .collect();
    template_data.insert("artist_headliners".to_string(), artist_headliners.join(", "));
    template_data.insert("artist_other".to_string(), artist_other.join(", "));
    if let Some(venue) = venue {
        let url_encoded_address: String = byte_serialize(
            format!("{},{},{},{}", &venue.address, &venue.city, &venue.state, &venue.country).as_bytes(),
        )
        .collect();
        template_data.insert("venue_name".to_string(), venue.name);
        template_data.insert("venue_address".to_string(), venue.address);
        template_data.insert("venue_city".to_string(), venue.city);
        template_data.insert("venue_state".to_string(), venue.state);
        template_data.insert("venue_country".to_string(), venue.country);
        template_data.insert("venue_postal_code".to_string(), venue.postal_code);
        template_data.insert("venue_phone".to_string(), venue.phone.unwrap_or("".to_string()));
        template_data.insert(
            "venue_promo_image_url".to_string(),
            venue.promo_image_url.unwrap_or("".to_string()),
        );
        let map_link = format!("https://www.google.com/maps/place/{}/", url_encoded_address);
        template_data.insert("venue_map_link".to_string(), map_link);
    }
    template_data.insert("organization_id".to_string(), organization.id.to_string());
    template_data.insert("organization_name".to_string(), organization.name);
    Ok(())
}
