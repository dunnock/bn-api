use crate::config::Config;
use crate::errors::*;
use bigneon_db::models::*;
use diesel::pg::PgConnection;
use phonenumber::{Mode, PhoneNumber};
use uuid::Uuid;

pub fn checkin_instructions(
    config: &Config,
    phone: PhoneNumber,
    order_id: Uuid,
    conn: &PgConnection,
) -> Result<(), BigNeonError> {
    let source = CommAddress::from(config.communication_default_source_phone.clone());
    let destinations = CommAddress::from(phone.format().mode(Mode::E164).to_string());
    let body = format!(
        "Thank you for your purchase! Your order number is #{}. Please head to the entry and let the door person know your first and last name so that they can check you in.",
        Order::parse_order_number(order_id)

    );

    Communication::new(
        CommunicationType::Sms,
        body,
        None,
        Some(source),
        destinations,
        None,
        None,
        Some(vec!["box-office"]),
        None,
    )
    .queue(conn)?;

    Ok(())
}
