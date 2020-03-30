use crate::support::database::TestDatabase;
use api::models::{AdminDisplayTicketType, DisplayTicketPricing};
use db::prelude::*;

#[test]
fn from_ticket_type() {
    let database = TestDatabase::new();

    let admin = database.create_user().finish();
    let organization = database.create_organization().with_fees().finish();
    let event = database
        .create_event()
        .with_organization(&organization)
        .with_ticket_pricing()
        .finish();
    let conn = database.connection.get();

    let ticket_type = event.ticket_types(true, None, conn).unwrap().remove(0);
    let ticket_type = ticket_type
        .update(
            TicketTypeEditableAttributes {
                limit_per_person: Some(150),
                ..Default::default()
            },
            Some(admin.id),
            conn,
        )
        .unwrap();

    let child_ticket_type = event
        .add_ticket_type(
            "Child ticket type".to_string(),
            None,
            105,
            None,
            Some(dates::now().add_hours(-1).finish()),
            TicketTypeEndDateType::Manual,
            Some(event.issuer_wallet(conn).unwrap().id),
            None,
            0,
            100,
            TicketTypeVisibility::Always,
            Some(ticket_type.id),
            0,
            true,
            true,
            true,
            TicketTypeType::Token,
            vec![],
            None,
            None,
            None,
            None,
            conn,
        )
        .unwrap();

    let ticket_pricing = ticket_type.current_ticket_pricing(false, conn).unwrap();
    let fee_schedule = FeeSchedule::find(organization.fee_schedule_id, conn).unwrap();
    let fee_in_cents = fee_schedule
        .get_range(ticket_pricing.price_in_cents, conn)
        .unwrap()
        .fee_in_cents;

    // New event nothing sold
    let display_ticket_type = AdminDisplayTicketType::from_ticket_type(&ticket_type, &fee_schedule, conn).unwrap();
    assert_eq!(display_ticket_type.available, 100);
    assert_eq!(display_ticket_type.status, TicketTypeStatus::Published);
    assert_eq!(
        display_ticket_type.limit_per_person,
        ticket_type.limit_per_person as u32
    );
    let display_ticket_pricing = display_ticket_type
        .ticket_pricing
        .clone()
        .into_iter()
        .find(|tp| tp.id == ticket_pricing.id)
        .unwrap();
    assert_eq!(fee_in_cents, display_ticket_pricing.fee_in_cents);
    assert_eq!(
        DisplayTicketPricing::from_ticket_pricing(&ticket_pricing, &fee_schedule, None, None, false, conn).unwrap(),
        display_ticket_pricing
    );

    // 10 tickets sold / reserved (via create_order for_event)
    let mut order = database.create_order().for_event(&event).quantity(10).finish();
    let display_ticket_type = AdminDisplayTicketType::from_ticket_type(&ticket_type, &fee_schedule, conn).unwrap();
    assert_eq!(display_ticket_type.available, 90);
    assert_eq!(display_ticket_type.status, TicketTypeStatus::Published);

    // Remaining tickets sold
    order
        .update_quantities(
            admin.id,
            &[UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 100,
                redemption_code: None,
            }],
            false,
            false,
            conn,
        )
        .unwrap();
    let display_ticket_type = AdminDisplayTicketType::from_ticket_type(&ticket_type, &fee_schedule, conn).unwrap();
    assert_eq!(display_ticket_type.available, 0);
    assert_eq!(display_ticket_type.status, TicketTypeStatus::SoldOut);

    // Release some tickets
    order
        .update_quantities(
            admin.id,
            &[UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 90,
                redemption_code: None,
            }],
            false,
            false,
            conn,
        )
        .unwrap();
    let display_ticket_type = AdminDisplayTicketType::from_ticket_type(&ticket_type, &fee_schedule, conn).unwrap();
    assert_eq!(display_ticket_type.available, 10);
    assert_eq!(display_ticket_type.status, TicketTypeStatus::Published);
    assert!(display_ticket_type.parent_id.is_none());
    assert!(display_ticket_type.parent_name.is_none());

    let display_ticket_type =
        AdminDisplayTicketType::from_ticket_type(&child_ticket_type, &fee_schedule, conn).unwrap();
    assert_eq!(display_ticket_type.available, 105);
    assert_eq!(display_ticket_type.status, TicketTypeStatus::SaleEnded);
    assert_eq!(display_ticket_type.parent_id, Some(ticket_type.id));
    assert_eq!(display_ticket_type.parent_name, Some(ticket_type.name));
}
