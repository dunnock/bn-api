use crate::config::{Config, EmailTemplate};
use crate::errors::*;
use crate::utils::expo;
use crate::utils::sendgrid::mail as sendgrid;
use crate::utils::twilio;
use crate::utils::webhook;
use bigneon_db::models::enums::*;
use bigneon_db::models::*;
use bigneon_db::services::CountryLookup;
use customer_io;
use diesel::PgConnection;
use log::Level::Trace;
use serde_json::Value;
use std::collections::HashMap;

pub async fn send_async(
    domain_action: &DomainAction,
    config: &Config,
    conn: &PgConnection,
) -> Result<(), BigNeonError> {
    let communication: Communication = match serde_json::from_value(domain_action.payload.clone()) {
        Ok(v) => v,
        Err(e) => return Err(e.into()),
    };

    if config.environment == Environment::Test {
        return Ok(());
    }

    if config.block_external_comms {
        jlog!(Trace, "Blocked communication", { "communication": communication });

        return Ok(());
    };

    let destination_addresses = communication.destinations.get();

    let future = match communication.comm_type {
        CommunicationType::EmailTemplate => {
            send_email_template(domain_action, &config, conn, communication, &destination_addresses)
        }
        CommunicationType::Sms => twilio::send_sms_async(
            &config.twilio_account_id,
            &config.twilio_api_key,
            communication.source.as_ref().unwrap().get_first().unwrap(),
            destination_addresses,
            &communication.body.unwrap_or(communication.title),
        ),
        CommunicationType::Push => expo::send_push_notification_async(
            &destination_addresses,
            &communication.body.unwrap_or(communication.title),
            communication.extra_data.map(|ed| json!(ed.clone())),
        ),
        CommunicationType::Webhook => webhook::send_webhook_async(
            &destination_addresses,
            &communication.body.unwrap_or(communication.title),
            domain_action.main_table_id,
            conn,
            &config,
        ),
    };
    future.await
}

async fn send_email_template(
    domain_action: &DomainAction,
    config: &Config,
    conn: &PgConnection,
    communication: Communication,
    destination_addresses: &Vec<String>,
) -> Result<(), BigNeonError> {
    if communication.template_id.is_none() {
        return Err(
            ApplicationError::new("Template ID must be specified when communication type is EmailTemplate".to_string())
                .into(),
        );
    }
    let template_id = communication.template_id.as_ref().unwrap();

    // Short circuit logic if communication template and template is blank
    if template_id == "" {
        jlog!(Trace, "Blocked communication, blank template ID", {
            "communication": communication
        });
        return Ok(());
    }
    let extra_data = communication.extra_data;
    // Check for provider. Sendgrid templates start with "d-".

    let template = if template_id.starts_with("d-") {
        EmailTemplate {
            provider: EmailProvider::Sendgrid,
            template_id: template_id.clone(),
        }
    } else {
        match template_id.parse() {
            Ok(t) => t,
            Err(e) => return Err(BigNeonError::from(e)),
        }
    };

    match template.provider {
        EmailProvider::CustomerIo => {
            // At some point there was some confusion and now we have both `extra_data` and
            // `template_data` which are both the same thing. This is because only emails use
            // `template data`, but other communications use `extra_data`. In future, `template_data`
            // should be dropped and only extra data used.
            let mut extra_data = extra_data.unwrap_or(HashMap::new());
            if let Some(ref td) = communication.template_data {
                for map in td {
                    for (key, value) in map {
                        extra_data.insert(key.clone(), json!(value));
                    }
                }
            }

            match customer_io_send_email(
                config,
                communication.destinations.addresses,
                template.template_id.clone(),
                communication.title,
                communication.body,
                extra_data,
                domain_action,
                conn,
            ) {
                Ok(_t) => Ok(()),
                Err(e) => return Err(e.into()),
            }
        }
        EmailProvider::Sendgrid => {
            let mut sendgrid_extra_data: HashMap<String, String> = HashMap::new();
            if let Some(ref ed) = extra_data {
                for (key, value) in ed {
                    sendgrid_extra_data.insert(key.clone(), value.as_str().unwrap_or("").to_string());
                }
            }

            // sendgrid
            sendgrid::send_email_template_async(
                &config.sendgrid_api_key,
                communication.source.as_ref().unwrap().get_first().unwrap(),
                &destination_addresses,
                template.template_id.clone(),
                communication.template_data.as_ref().unwrap(),
                communication.categories.clone(),
                Some(sendgrid_extra_data),
            ).await
        } // Customer IO
    }
}

pub fn customer_io_send_email(
    config: &Config,
    dest_email_addresses: Vec<String>,
    template_id: String,
    title: String,
    body: Option<String>,
    mut template_data: HashMap<String, Value>,
    domain_action: &DomainAction,
    conn: &PgConnection,
) -> Result<(), BigNeonError> {
    // new() try's to parse base url to URL
    let client = customer_io::CustomerIoClient::new(
        config.customer_io.api_key.clone(),
        config.customer_io.site_id.clone(),
        &config.customer_io.base_url,
    )?;

    if !template_data.contains_key("timestamp") {
        template_data.insert("timestamp".to_string(), json!(domain_action.scheduled_at.timestamp()));
    }

    template_data.insert("subject".to_string(), json!(title));

    if let Some(b) = body {
        template_data.insert("message".to_string(), json!(b));
    }

    if domain_action.main_table == Some(Tables::Events) && domain_action.main_table_id.is_some() {
        let event = Event::find(domain_action.main_table_id.unwrap(), conn)?;
        let venue = event.venue(conn)?;
        let localized_times = event.get_all_localized_times(venue.as_ref());

        template_data.insert("show_name".to_string(), json!(event.name.clone()));

        if let Some(event_start) = localized_times.event_start {
            template_data.insert(
                "show_start_date".to_string(),
                json!(format!(
                    "{} {}",
                    event_start.format("%A,"),
                    event_start.format("%e %B %Y").to_string().trim()
                )
                .to_string()),
            );
            template_data.insert(
                "show_start_time".to_string(),
                json!(event_start.format("%l:%M %p %Z").to_string().trim().to_string()),
            );
        }

        if let Some(venue_id) = event.venue_id {
            let venue = Venue::find(venue_id, conn)?;

            template_data.insert("show_venue_name".to_string(), json!(venue.name.clone()));

            template_data.insert("show_venue_address".to_string(), json!(venue.address));
            template_data.insert("show_venue_city".to_string(), json!(venue.city));

            // need to convert state to 2 letter abbreviation
            let venue_state = parse_state(&venue.state, &venue.country);
            template_data.insert("show_venue_state".to_string(), json!(venue_state));
            template_data.insert("show_venue_postal_code".to_string(), json!(venue.postal_code));
        }
    }
    // loop dest_email_addresses, each email will be sent different email address
    for email_address in dest_email_addresses {
        let event = customer_io::Event {
            name: template_id.clone(),
            data: customer_io::EventData {
                recipient: Some(email_address),
                extra: template_data.clone(),
            },
        };
        client.create_anonymous_event(event)?;
    }
    Ok(())
}

// if Country giving is not US, just return the state, do nothing with it
// else lookup the state if it is not a abbreviation (2 letters) and return the abbreviation
fn parse_state(state_to_parse: &str, country: &str) -> String {
    if country.trim().to_lowercase() == "us" || country.trim().to_lowercase() == "united states" {
        CountryLookup::new()
            .ok()
            .and_then(|c| c.find("US"))
            .and_then(|country_datum| country_datum.convert_state(state_to_parse))
            .unwrap_or(state_to_parse.to_string())
    } else {
        state_to_parse.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::parse_state;
    #[test]
    fn convert_states_test() {
        assert_eq!(parse_state(" utah ", "US"), "UT");
        assert_eq!(parse_state(" ut ", "us"), "UT");
        assert_eq!(parse_state(" West Virginia ", "US"), "WV");
        assert_eq!(parse_state("southdakota", "US"), "southdakota"); // failing misspelled state
        assert_eq!(parse_state("Gauteng", "SA"), "Gauteng"); // if country is not US, just return the giving state
    }
}
