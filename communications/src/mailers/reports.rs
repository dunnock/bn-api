use bigneon_config::Config;
use bigneon_errors::*;
use bigneon_db::models::{TicketCountReport, Event, CommAddress, Communication, CommunicationType};
use chrono::prelude::*;
use diesel::PgConnection;
use serde_json::json;
use std::collections::HashMap;

pub fn ticket_counts(
    email: String,
    event: &Event,
    ticket_count_report: &TicketCountReport,
    config: &Config,
    conn: &PgConnection,
) -> Result<(), BigNeonError> {
    let source = CommAddress::from(config.communication_default_source_email.clone());
    let destinations = CommAddress::from(email);
    let title = "BigNeon Ticket Counts".to_string();
    let template_id = config.email_templates.ticket_count_report.to_string();
    let mut extra_data: HashMap<String, serde_json::Value> = HashMap::new();
    Event::event_payload_data(&event, &config.front_end_url, &mut extra_data, conn)?;
    extra_data.insert("report".to_string(), json!(ticket_count_report));
    extra_data.insert("timestamp".to_string(), json!(Utc::now().timestamp()));

    Communication::new(
        CommunicationType::EmailTemplate,
        title,
        None,
        Some(source),
        destinations,
        Some(template_id),
        None,
        Some(vec!["ticket_counts", "reports"]),
        Some(extra_data),
    )
    .queue(conn)?;

    Ok(())
}
