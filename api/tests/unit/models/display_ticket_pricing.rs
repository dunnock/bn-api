use crate::support::database::TestDatabase;
use api::models::DisplayTicketPricing;
use chrono::prelude::*;
use chrono::Duration;
use db::prelude::*;

#[test]
fn from_ticket_pricing() {
    let database = TestDatabase::new();
    let organization = database.create_organization().with_fees().finish();
    let event = database
        .create_event()
        .with_organization(&organization)
        .with_ticket_pricing()
        .finish();
    let conn = database.connection.get();

    let ticket_type = event.ticket_types(true, None, conn).unwrap().remove(0);
    let ticket_pricing = ticket_type.current_ticket_pricing(false, conn).unwrap();

    let fee_schedule = FeeSchedule::find(organization.fee_schedule_id, conn).unwrap();
    let fee_in_cents = fee_schedule
        .get_range(ticket_pricing.price_in_cents, conn)
        .unwrap()
        .fee_in_cents;

    // Display ticket pricing
    let display_ticket_pricing =
        DisplayTicketPricing::from_ticket_pricing(&ticket_pricing, &fee_schedule, None, None, false, conn).unwrap();
    assert_eq!(display_ticket_pricing.price_in_cents, ticket_pricing.price_in_cents);
    assert_eq!(display_ticket_pricing.discount_in_cents, 0);
    assert_eq!(display_ticket_pricing.fee_in_cents, fee_in_cents);

    // Box office ticket pricing
    let display_ticket_pricing =
        DisplayTicketPricing::from_ticket_pricing(&ticket_pricing, &fee_schedule, None, None, true, conn).unwrap();
    assert_eq!(display_ticket_pricing.price_in_cents, ticket_pricing.price_in_cents);
    assert_eq!(display_ticket_pricing.discount_in_cents, 0);
    assert_eq!(display_ticket_pricing.fee_in_cents, 0); // No fee for box office tickets

    // Holds code with discount
    let hold = database
        .create_hold()
        .with_hold_type(HoldTypes::Discount)
        .with_quantity(1)
        .with_ticket_type_id(ticket_type.id)
        .finish();
    assert_eq!(Some(10), hold.discount_in_cents);
    let discounted_fee_in_cents = fee_schedule
        .get_range(ticket_pricing.price_in_cents - 10, conn)
        .unwrap()
        .fee_in_cents;
    let display_ticket_pricing = DisplayTicketPricing::from_ticket_pricing(
        &ticket_pricing,
        &fee_schedule,
        hold.redemption_code,
        Some(event.id),
        false,
        conn,
    )
    .unwrap();
    assert_eq!(display_ticket_pricing.price_in_cents, ticket_pricing.price_in_cents);
    assert_eq!(display_ticket_pricing.discount_in_cents, 10);
    assert_eq!(display_ticket_pricing.fee_in_cents, discounted_fee_in_cents);

    // Comps code
    let hold = database
        .create_hold()
        .with_hold_type(HoldTypes::Comp)
        .with_quantity(1)
        .with_ticket_type_id(ticket_type.id)
        .finish();
    let display_ticket_pricing = DisplayTicketPricing::from_ticket_pricing(
        &ticket_pricing,
        &fee_schedule,
        hold.redemption_code,
        Some(event.id),
        false,
        conn,
    )
    .unwrap();
    assert_eq!(display_ticket_pricing.price_in_cents, ticket_pricing.price_in_cents);
    assert_eq!(display_ticket_pricing.discount_in_cents, ticket_pricing.price_in_cents);
    assert_eq!(display_ticket_pricing.fee_in_cents, 0);

    // Discount code
    let code = database
        .create_code()
        .with_event(&event)
        .with_code_type(CodeTypes::Discount)
        .for_ticket_type(&ticket_type)
        .with_discount_in_cents(Some(10))
        .finish();
    assert_eq!(Some(10), code.discount_in_cents);
    let discounted_fee_in_cents = fee_schedule
        .get_range(ticket_pricing.price_in_cents - 10, conn)
        .unwrap()
        .fee_in_cents;
    let display_ticket_pricing = DisplayTicketPricing::from_ticket_pricing(
        &ticket_pricing,
        &fee_schedule,
        Some(code.redemption_code),
        Some(event.id),
        false,
        conn,
    )
    .unwrap();
    assert_eq!(display_ticket_pricing.price_in_cents, ticket_pricing.price_in_cents);
    assert_eq!(display_ticket_pricing.discount_in_cents, 10);
    assert_eq!(display_ticket_pricing.fee_in_cents, discounted_fee_in_cents);

    // Code expired
    let code = database
        .create_code()
        .with_event(&event)
        .with_code_type(CodeTypes::Discount)
        .for_ticket_type(&ticket_type)
        .with_discount_in_cents(Some(10))
        .with_end_date(NaiveDateTime::from(
            Utc::now().naive_utc() - Duration::days(1) + Duration::minutes(2),
        ))
        .finish();
    let display_ticket_pricing = DisplayTicketPricing::from_ticket_pricing(
        &ticket_pricing,
        &fee_schedule,
        Some(code.redemption_code),
        Some(event.id),
        false,
        conn,
    )
    .unwrap();
    // Code is not applied
    assert_eq!(display_ticket_pricing.price_in_cents, ticket_pricing.price_in_cents);
    assert_eq!(display_ticket_pricing.discount_in_cents, 0);
    assert_eq!(display_ticket_pricing.fee_in_cents, fee_in_cents);
}
