use crate::communications::mailers::insert_event_template_data;
use crate::config::Config;
use crate::errors::*;
use chrono::prelude::*;
use db::models::*;
use diesel::pg::PgConnection;
use itertools::Itertools;

pub fn send_tickets(
    config: &Config,
    email: String,
    transfer: &Transfer,
    from_user: &User,
    conn: &PgConnection,
) -> Result<(), ApiError> {
    let receive_tickets_link = transfer.receive_url(&config.front_end_url, conn)?;
    let source = CommAddress::from(config.communication_default_source_email.clone());
    let destinations = CommAddress::from(email);
    let title = "{sender_name} has sent you some tickets".to_string();
    let template_id = config.sendgrid_template_bn_transfer_tickets.clone();
    let mut template_data = TemplateData::new();
    template_data.insert("sender_name".to_string(), from_user.full_name());
    template_data.insert("receive_tickets_link".to_string(), receive_tickets_link);
    let events = transfer.events(conn)?;
    let event_ids = events.iter().map(|e| e.id.to_string()).join(",");
    let days_until_event = events
        .iter()
        .map(|e| {
            match e.event_start {
                Some(s) => (Utc::now().naive_utc() - s).num_days(),
                None => 0,
            }
            .to_string()
        })
        .join(",");
    Communication::new(
        CommunicationType::EmailTemplate,
        title,
        None,
        Some(source),
        destinations,
        Some(template_id),
        Some(vec![template_data]),
        Some(vec!["transfer", "transfer_receiver", "transfer_confirmation"]),
        Some(
            map!("event_id".to_string() => json!(event_ids), "days_until_event".to_string() => json!(days_until_event)),
        ),
    )
    .queue(conn)?;

    Ok(())
}

pub fn transfer_drip_reminder(
    email: String,
    transfer: &Transfer,
    event: &Event,
    source_or_destination: SourceOrDestination,
    config: &Config,
    conn: &PgConnection,
) -> Result<(), ApiError> {
    let receive_tickets_link = transfer.receive_url(&config.front_end_url, conn)?;
    let source = CommAddress::from(config.communication_default_source_email.clone());
    let destinations = CommAddress::from(email.clone());
    let title = "BigNeon: Ticket transfer reminder".to_string();
    let user = User::find(transfer.source_user_id, conn)?;
    let template_id = if source_or_destination == SourceOrDestination::Source {
        config.sendgrid_template_bn_transfer_tickets_drip_source.clone()
    } else {
        config.sendgrid_template_bn_transfer_tickets_drip_destination.clone()
    };
    let transfer_cancel_url = format!("{}/my-events?event_id={}", config.front_end_url.clone(), event.id,);

    let mut template_data = TemplateData::new();
    template_data.insert(
        "header".to_string(),
        transfer.drip_header(event, source_or_destination, true, config.environment, conn)?,
    );
    template_data.insert("sender_name".to_string(), Transfer::sender_name(&user));
    template_data.insert(
        "receiver_address".to_string(),
        transfer.transfer_address.clone().unwrap_or("".to_string()),
    );
    template_data.insert("transfer_accept_url".to_string(), receive_tickets_link);
    template_data.insert("transfer_cancel_url".to_string(), transfer_cancel_url);
    template_data.insert("transfer_id".to_string(), transfer.id.to_string());
    insert_event_template_data(&mut template_data, event, conn)?;

    Communication::new(
        CommunicationType::EmailTemplate,
        title,
        None,
        Some(source),
        destinations,
        Some(template_id),
        Some(vec![template_data]),
        Some(vec!["transfer", "transfer_receiver", "transfer_drip"]),
        None,
    )
    .queue(conn)?;

    Ok(())
}

pub fn transfer_sent_receipt(
    user: &User,
    transfer: &Transfer,
    event: &Event,
    config: &Config,
    conn: &PgConnection,
) -> Result<(), ApiError> {
    let source = CommAddress::from(config.communication_default_source_email.clone());
    if let Some(email) = user.email.clone() {
        let destinations = CommAddress::from(email);
        let title = "BigNeon: Ticket transfer sent".to_string();
        let template_id = config.sendgrid_template_bn_transfer_tickets_receipt.clone();
        let transfer_cancel_url = format!("{}/my-events?event_id={}", config.front_end_url.clone(), event.id,);
        let mut template_data = TemplateData::new();
        template_data.insert("sender_name".to_string(), Transfer::sender_name(&user));
        template_data.insert(
            "receiver_address".to_string(),
            transfer.transfer_address.clone().unwrap_or("".to_string()),
        );
        template_data.insert("transfer_cancel_url".to_string(), transfer_cancel_url);
        template_data.insert("transfer_id".to_string(), transfer.id.to_string());
        insert_event_template_data(&mut template_data, event, conn)?;

        Communication::new(
            CommunicationType::EmailTemplate,
            title,
            None,
            Some(source),
            destinations,
            Some(template_id),
            Some(vec![template_data]),
            Some(vec!["transfer", "transfer_sender", "transfer_confirmation"]),
            None,
        )
        .queue(conn)?;
    }
    Ok(())
}

pub fn transfer_cancelled_receipt(
    config: &Config,
    email: String,
    from_user: &User,
    transfer: &Transfer,
    conn: &PgConnection,
) -> Result<(), ApiError> {
    let source = CommAddress::from(config.communication_default_source_email.clone());
    let destinations = CommAddress::from(email);
    let title = "BigNeon: Cancelled ticket transfer".to_string();
    let template_id = config.sendgrid_template_bn_cancel_transfer_tickets_receipt.clone();
    let mut template_data = TemplateData::new();
    template_data.insert("sender_name".to_string(), Transfer::sender_name(&from_user));
    template_data.insert(
        "receiver_address".to_string(),
        transfer.transfer_address.clone().unwrap_or("".to_string()),
    );
    template_data.insert("transfer_id".to_string(), transfer.id.to_string());
    Communication::new(
        CommunicationType::EmailTemplate,
        title,
        None,
        Some(source),
        destinations,
        Some(template_id),
        Some(vec![template_data]),
        Some(vec!["transfer", "transfer_receiver", "transfer_cancellation"]),
        None,
    )
    .queue(conn)?;

    Ok(())
}

pub fn transfer_cancelled(
    config: &Config,
    email: String,
    from_user: &User,
    transfer: &Transfer,
    conn: &PgConnection,
) -> Result<(), ApiError> {
    let source = CommAddress::from(config.communication_default_source_email.clone());
    let destinations = CommAddress::from(email);
    let title = "{sender_name} has cancelled their transfer of tickets".to_string();
    let template_id = config.sendgrid_template_bn_cancel_transfer_tickets.clone();
    let mut template_data = TemplateData::new();
    template_data.insert("sender_name".to_string(), Transfer::sender_name(&from_user));
    template_data.insert(
        "receiver_address".to_string(),
        transfer.transfer_address.clone().unwrap_or("".to_string()),
    );
    template_data.insert("transfer_id".to_string(), transfer.id.to_string());
    Communication::new(
        CommunicationType::EmailTemplate,
        title,
        None,
        Some(source),
        destinations,
        Some(template_id),
        Some(vec![template_data]),
        Some(vec!["transfer", "transfer_receiver", "transfer_cancellation"]),
        None,
    )
    .queue(conn)?;

    Ok(())
}
