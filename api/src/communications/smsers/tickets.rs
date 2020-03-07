use bigneon_db::models::*;
use crate::config::Config;
use diesel::pg::PgConnection;
use crate::errors::*;
use crate::utils::deep_linker::DeepLinker;

pub fn transfer_cancelled(
    config: &Config,
    phone: String,
    from_user: &User,
    conn: &PgConnection,
) -> Result<(), BigNeonError> {
    let source = CommAddress::from(config.communication_default_source_phone.clone());
    let destinations = CommAddress::from(phone);
    let body = format!("{} has cancelled their transferred tickets.", from_user.full_name());
    Communication::new(
        CommunicationType::Sms,
        body,
        None,
        Some(source),
        destinations,
        None,
        None,
        Some(vec!["transfers"]),
        None,
    )
    .queue(conn)?;

    Ok(())
}

pub fn transfer_drip_reminder(
    phone: String,
    transfer: &Transfer,
    event: &Event,
    config: &Config,
    conn: &PgConnection,
    deep_linker: &dyn DeepLinker,
) -> Result<(), BigNeonError> {
    let receive_tickets_link = transfer.receive_url(config.front_end_url.clone(), conn)?;
    let link = deep_linker.create_deep_link_with_fallback(&receive_tickets_link);
    let source = CommAddress::from(config.communication_default_source_phone.clone());
    let destinations = CommAddress::from(phone);
    let body = format!(
        "{} Follow this link to receive them: {}",
        transfer.drip_header(event, SourceOrDestination::Destination, false, config.environment, conn)?,
        link
    );
    Communication::new(
        CommunicationType::Sms,
        body,
        None,
        Some(source),
        destinations,
        None,
        None,
        Some(vec!["transfers"]),
        None,
    )
    .queue(conn)?;

    Ok(())
}

pub fn send_tickets(
    config: &Config,
    phone: String,
    transfer: &Transfer,
    from_user: &User,
    conn: &PgConnection,
    deep_linker: &dyn DeepLinker,
) -> Result<(), BigNeonError> {
    let receive_tickets_link = transfer.receive_url(config.front_end_url.clone(), conn)?;
    let link = deep_linker.create_deep_link_with_fallback(&receive_tickets_link);

    let source = CommAddress::from(config.communication_default_source_phone.clone());
    let destinations = CommAddress::from(phone);
    let body = format!(
        "{} has sent you some tickets. Follow this link to receive them: {}",
        from_user.full_name(),
        link
    );
    Communication::new(
        CommunicationType::Sms,
        body,
        None,
        Some(source),
        destinations,
        None,
        None,
        Some(vec!["transfers"]),
        None,
    )
    .queue(conn)?;

    Ok(())
}
