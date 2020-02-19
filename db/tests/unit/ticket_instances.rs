use chrono::prelude::*;
use chrono::Duration;
use chrono::NaiveDateTime;
use diesel;
use diesel::result::Error;
use diesel::sql_types;
use diesel::Connection;
use diesel::RunQueryDsl;
use uuid::Uuid;

use bigneon_db::dev::times;
use bigneon_db::dev::TestProject;
use bigneon_db::prelude::*;
use bigneon_db::utils::errors::ErrorCode::ValidationError;

#[test]
fn associate_redeem_key() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project
        .create_event()
        .with_ticket_pricing()
        .with_ticket_type_count(1)
        .finish();
    let user = project.create_user().finish();
    project
        .create_order()
        .for_event(&event)
        .for_user(&user)
        .quantity(1)
        .is_paid()
        .finish();
    let ticket = TicketInstance::find_for_user(user.id, connection)
        .unwrap()
        .pop()
        .unwrap();
    let redeem_key = ticket.redeem_key.clone();
    ticket.associate_redeem_key(connection).unwrap();
    let ticket = TicketInstance::find(ticket.id, connection).unwrap();
    let new_redeem_key = ticket.redeem_key.clone();
    assert_ne!(redeem_key, new_redeem_key);

    ticket.associate_redeem_key(connection).unwrap();
    let ticket = TicketInstance::find(ticket.id, connection).unwrap();
    assert_ne!(new_redeem_key, ticket.redeem_key);
}

#[test]
fn find_for_user_for_display() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().with_fees().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_event_start(NaiveDate::from_ymd(2016, 7, 8).and_hms(9, 10, 11))
        .with_event_end(NaiveDate::from_ymd(2016, 7, 9).and_hms(9, 10, 11))
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let event2 = project
        .create_event()
        .with_organization(&organization)
        .with_event_start(NaiveDate::from_ymd(2017, 7, 8).and_hms(9, 10, 11))
        .with_event_end(NaiveDate::from_ymd(2017, 7, 9).and_hms(9, 10, 11))
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let user = project.create_user().finish();
    project
        .create_order()
        .for_user(&user)
        .quantity(2)
        .for_event(&event)
        .is_paid()
        .finish();
    let mut cart2 = project
        .create_order()
        .for_user(&user)
        .quantity(2)
        .for_event(&event2)
        .finish();

    // Order is not paid so tickets are not accessible
    assert!(
        TicketInstance::find_for_user_for_display(user.id, Some(event2.id), None, None, connection)
            .unwrap()
            .is_empty()
    );

    let total = cart2.calculate_total(connection).unwrap();
    cart2
        .add_external_payment(
            Some("test".to_string()),
            ExternalPaymentType::CreditCard,
            user.id,
            total,
            connection,
        )
        .unwrap();

    let found_tickets =
        TicketInstance::find_for_user_for_display(user.id, Some(event.id), None, None, connection).unwrap();
    assert_eq!(found_tickets.len(), 1);
    assert_eq!(found_tickets[0].0.id, event.id);
    assert_eq!(found_tickets[0].1.len(), 2);

    // Pending transfer
    assert_eq!(found_tickets[0].1[0].pending_transfer, false);
    assert_eq!(
        TicketInstance::find_for_display(found_tickets[0].1[0].id, connection)
            .unwrap()
            .2
            .pending_transfer,
        false
    );
    let transfer = Transfer::create(user.id, Uuid::new_v4(), None, None, false)
        .commit(connection)
        .unwrap();
    transfer
        .add_transfer_ticket(found_tickets[0].1[0].id, connection)
        .unwrap();

    let found_tickets =
        TicketInstance::find_for_user_for_display(user.id, Some(event.id), None, None, connection).unwrap();
    assert_eq!(found_tickets.len(), 1);
    assert_eq!(found_tickets[0].0.id, event.id);
    assert_eq!(found_tickets[0].1.len(), 2);
    assert_eq!(found_tickets[0].1[0].pending_transfer, true);
    assert_eq!(
        TicketInstance::find_for_display(found_tickets[0].1[0].id, connection)
            .unwrap()
            .2
            .pending_transfer,
        true
    );

    // Transfer is completed
    diesel::sql_query(
        r#"
        UPDATE transfers
        SET status = 'Completed'
        WHERE id = $1;
        "#,
    )
    .bind::<sql_types::Uuid, _>(transfer.id)
    .execute(connection)
    .unwrap();
    let found_tickets =
        TicketInstance::find_for_user_for_display(user.id, Some(event.id), None, None, connection).unwrap();
    assert_eq!(found_tickets.len(), 1);
    assert_eq!(found_tickets[0].0.id, event.id);
    assert_eq!(found_tickets[0].1.len(), 2);
    assert_eq!(found_tickets[0].1[0].pending_transfer, false);
    assert_eq!(
        TicketInstance::find_for_display(found_tickets[0].1[0].id, connection)
            .unwrap()
            .2
            .pending_transfer,
        false
    );

    // Another pending transfer
    let transfer = Transfer::create(user.id, Uuid::new_v4(), None, None, false)
        .commit(connection)
        .unwrap();
    transfer
        .add_transfer_ticket(found_tickets[0].1[0].id, connection)
        .unwrap();

    let found_tickets =
        TicketInstance::find_for_user_for_display(user.id, Some(event.id), None, None, connection).unwrap();
    assert_eq!(found_tickets.len(), 1);
    assert_eq!(found_tickets[0].0.id, event.id);
    assert_eq!(found_tickets[0].1.len(), 2);
    assert_eq!(found_tickets[0].1[0].pending_transfer, true);
    assert_eq!(
        TicketInstance::find_for_display(found_tickets[0].1[0].id, connection)
            .unwrap()
            .2
            .pending_transfer,
        true
    );

    // other event
    let found_tickets =
        TicketInstance::find_for_user_for_display(user.id, Some(event2.id), None, None, connection).unwrap();
    assert_eq!(found_tickets.len(), 1);
    assert_eq!(found_tickets[0].0.id, event2.id);
    assert_eq!(found_tickets[0].1.len(), 2);

    // no event specified
    let found_tickets = TicketInstance::find_for_user_for_display(user.id, None, None, None, connection).unwrap();
    assert_eq!(found_tickets.len(), 2);
    assert_eq!(found_tickets[0].0.id, event.id);
    assert_eq!(found_tickets[0].1.len(), 2);
    assert_eq!(found_tickets[1].0.id, event2.id);
    assert_eq!(found_tickets[1].1.len(), 2);

    // start date prior to both event starts
    let found_tickets = TicketInstance::find_for_user_for_display(
        user.id,
        None,
        Some(NaiveDate::from_ymd(2015, 7, 8).and_hms(9, 0, 11)),
        None,
        connection,
    )
    .unwrap();
    assert_eq!(found_tickets.len(), 2);
    assert_eq!(found_tickets[0].0.id, event.id);
    assert_eq!(found_tickets[0].1.len(), 2);
    assert_eq!(found_tickets[1].0.id, event2.id);
    assert_eq!(found_tickets[1].1.len(), 2);

    // start date past event start time but before event end time returns both
    let found_tickets = TicketInstance::find_for_user_for_display(
        user.id,
        None,
        Some(NaiveDate::from_ymd(2016, 7, 8).and_hms(11, 10, 11)),
        None,
        connection,
    )
    .unwrap();
    assert_eq!(found_tickets.len(), 2);
    assert_eq!(found_tickets[0].0.id, event.id);
    assert_eq!(found_tickets[0].1.len(), 2);
    assert_eq!(found_tickets[1].0.id, event2.id);
    assert_eq!(found_tickets[1].1.len(), 2);

    // start date filters out event
    let found_tickets = TicketInstance::find_for_user_for_display(
        user.id,
        None,
        Some(NaiveDate::from_ymd(2017, 7, 8).and_hms(9, 0, 11)),
        None,
        connection,
    )
    .unwrap();
    assert_eq!(found_tickets.len(), 1);
    assert_eq!(found_tickets[0].0.id, event2.id);
    assert_eq!(found_tickets[0].1.len(), 2);

    // end date filters out event
    let found_tickets = TicketInstance::find_for_user_for_display(
        user.id,
        None,
        None,
        Some(NaiveDate::from_ymd(2017, 7, 8).and_hms(9, 0, 11)),
        connection,
    )
    .unwrap();
    assert_eq!(found_tickets.len(), 1);
    assert_eq!(found_tickets[0].0.id, event.id);
    assert_eq!(found_tickets[0].1.len(), 2);
}

#[test]
pub fn update() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project
        .create_event()
        .with_ticket_pricing()
        .with_ticket_type_count(1)
        .finish();
    let user = project.create_user().finish();
    project
        .create_order()
        .for_event(&event)
        .for_user(&user)
        .quantity(1)
        .is_paid()
        .finish();
    let ticket = TicketInstance::find_for_user(user.id, connection)
        .unwrap()
        .pop()
        .unwrap();
    let attrs = UpdateTicketInstanceAttributes {
        first_name_override: Some(Some("First".to_string())),
        last_name_override: Some(Some("Last".to_string())),
    };

    let domain_event_count = DomainEvent::find(
        Tables::TicketInstances,
        Some(ticket.id),
        Some(DomainEventTypes::TicketInstanceUpdated),
        connection,
    )
    .unwrap()
    .len();

    let updated = ticket.update(attrs, user.id, &project.connection).unwrap();
    assert_eq!(updated.first_name_override, Some("First".to_string()));
    assert_eq!(updated.last_name_override, Some("Last".to_string()));
    let new_domain_event_count = DomainEvent::find(
        Tables::TicketInstances,
        Some(updated.id),
        Some(DomainEventTypes::TicketInstanceUpdated),
        connection,
    )
    .unwrap()
    .len();

    // 1 order update event should be recorded from the update call
    assert_eq!(domain_event_count + 1, new_domain_event_count);
}

#[test]
pub fn update_with_validation_errors() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project
        .create_event()
        .with_ticket_pricing()
        .with_ticket_type_count(1)
        .finish();
    let user = project.create_user().finish();
    project
        .create_order()
        .for_event(&event)
        .for_user(&user)
        .quantity(1)
        .is_paid()
        .finish();
    let ticket = TicketInstance::find_for_user(user.id, connection)
        .unwrap()
        .pop()
        .unwrap();
    let attrs = UpdateTicketInstanceAttributes {
        first_name_override: Some(Some("First".to_string())),
        last_name_override: None,
    };
    let result = ticket.clone().update(attrs, user.id, connection);
    match result {
        Ok(_) => {
            panic!("Expected error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("last_name_override"));
                assert_eq!(errors["last_name_override"].len(), 1);
                assert_eq!(errors["last_name_override"][0].code, "required");
                assert_eq!(
                    &errors["last_name_override"][0].message.clone().unwrap().into_owned(),
                    "Ticket last name required if first name provided"
                );
            }
            _ => panic!("Expected validation error"),
        },
    }

    let attrs = UpdateTicketInstanceAttributes {
        first_name_override: None,
        last_name_override: Some(Some("Last".to_string())),
    };
    let result = ticket.clone().update(attrs, user.id, connection);
    match result {
        Ok(_) => {
            panic!("Expected error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("first_name_override"));
                assert_eq!(errors["first_name_override"].len(), 1);
                assert_eq!(errors["first_name_override"][0].code, "required");
                assert_eq!(
                    &errors["first_name_override"][0].message.clone().unwrap().into_owned(),
                    "Ticket first name required if last name provided"
                );
            }
            _ => panic!("Expected validation error"),
        },
    }

    let attrs = UpdateTicketInstanceAttributes {
        first_name_override: Some(Some("First".to_string())),
        last_name_override: Some(Some("Last".to_string())),
    };
    assert!(ticket.clone().update(attrs.clone(), user.id, connection).is_ok());

    // Cannot update redeemed ticket
    TicketInstance::redeem_ticket(
        ticket.id,
        ticket.redeem_key.clone().unwrap(),
        user.id,
        CheckInSource::GuestList,
        connection,
    )
    .unwrap();
    let ticket = TicketInstance::find(ticket.id, connection).unwrap();
    assert_eq!(
        ticket.update(attrs.clone(), user.id, connection),
        DatabaseError::business_process_error("Unable to update ticket as it has already been redeemed.",)
    );

    // Cannot update pending ticket
    let box_office_order = project
        .create_order()
        .for_event(&event)
        .for_user(&user)
        .quantity(1)
        .finish();
    let ticket = box_office_order.tickets(None, connection).unwrap().pop().unwrap();
    assert_eq!(
        ticket.update(attrs, user.id, connection),
        DatabaseError::business_process_error("Unable to update ticket as it is not purchased.",)
    );
}

#[test]
fn find_ids_for_order() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project
        .create_event()
        .with_ticket_pricing()
        .with_ticket_type_count(1)
        .finish();
    let user = project.create_user().finish();
    let order = project
        .create_order()
        .for_event(&event)
        .for_user(&user)
        .quantity(1)
        .is_paid()
        .finish();
    let ticket = &TicketInstance::find_for_user(user.id, connection).unwrap()[0];
    // Add additional tickets to user account
    project
        .create_order()
        .for_event(&event)
        .for_user(&user)
        .quantity(1)
        .is_paid()
        .finish();
    assert_eq!(
        TicketInstance::find_ids_for_order(order.id, connection).unwrap(),
        vec![ticket.id]
    );
}

#[test]
fn ticket_type() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project
        .create_event()
        .with_ticket_pricing()
        .with_ticket_type_count(1)
        .finish();
    let ticket_type = event.ticket_types(true, None, connection).unwrap().remove(0);
    let user = project.create_user().finish();
    project
        .create_order()
        .for_event(&event)
        .for_user(&user)
        .quantity(1)
        .is_paid()
        .finish();
    let ticket = TicketInstance::find_for_user(user.id, connection).unwrap().remove(0);
    assert_eq!(ticket_type, ticket.ticket_type(connection).unwrap());
}

#[test]
fn cant_reserve_more_than_tickets_available() {
    let project = TestProject::new();
    let connection = project.get_connection();

    let event = project
        .create_event()
        .with_ticket_pricing()
        .with_a_specific_number_of_tickets(1)
        .finish();

    let user = project.create_user().finish();
    let order = project
        .create_order()
        .for_event(&event)
        .for_user(&user)
        .quantity(1)
        .finish();

    let mut order_item: Vec<OrderItem> = order
        .items(connection)
        .unwrap()
        .into_iter()
        .filter(|oi| oi.item_type == OrderItemTypes::Tickets)
        .collect();
    let order_item = order_item.pop().unwrap();
    let res = TicketInstance::reserve_tickets(
        &order_item,
        Some(times::infinity()),
        order_item.ticket_type_id.unwrap(),
        None,
        1,
        connection,
    );

    assert!(res.is_err());
}

#[test]
fn cant_reserve_nullifed_tickets() {
    let project = TestProject::new();
    let connection = project.get_connection();

    let event = project
        .create_event()
        .with_ticket_pricing()
        .with_a_specific_number_of_tickets(2)
        .finish();

    let user = project.create_user().finish();
    let order = project
        .create_order()
        .for_event(&event)
        .for_user(&user)
        .quantity(1)
        .finish();

    let mut order_item: Vec<OrderItem> = order
        .items(connection)
        .unwrap()
        .into_iter()
        .filter(|oi| oi.item_type == OrderItemTypes::Tickets)
        .collect();
    let order_item = order_item.pop().unwrap();

    let asset = Asset::find_by_ticket_type(order_item.ticket_type_id.unwrap(), connection).unwrap();

    TicketInstance::nullify_tickets(asset.id, 1, user.id, connection).unwrap();
    let res = TicketInstance::reserve_tickets(
        &order_item,
        Some(times::infinity()),
        order_item.ticket_type_id.unwrap(),
        None,
        1,
        connection,
    );

    assert!(res.is_err());
}

#[test]
fn release() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let creator = project.create_user().finish();
    let organization = project.create_organization().with_fees().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_ticket_pricing()
        .with_a_specific_number_of_tickets(1)
        .finish();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    let user = project.create_user().finish();
    project
        .create_order()
        .for_event(&event)
        .for_user(&user)
        .quantity(1)
        .is_paid()
        .finish();
    let ticket = TicketInstance::find_for_user(user.id, connection).unwrap().remove(0);
    assert_eq!(ticket.status, TicketInstanceStatus::Purchased);
    TicketInstance::create_transfer(&user, &[ticket.id], None, None, false, connection).unwrap();
    assert!(ticket
        .release(TicketInstanceStatus::Purchased, creator.id, connection)
        .is_ok());

    // Reload ticket
    let ticket = TicketInstance::find(ticket.id, connection).unwrap();
    assert!(ticket.order_item_id.is_none());
    assert_eq!(ticket.status, TicketInstanceStatus::Available);

    // Cart adds ticket type (which only had 1 ticket) setting this to Reserved
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 1,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    let ticket = TicketInstance::find(ticket.id, connection).unwrap();
    assert!(ticket.order_item_id.is_some());
    assert_eq!(ticket.status, TicketInstanceStatus::Reserved);

    // Ticket is not nullified as ticket type is not cancelled
    let domain_events = DomainEvent::find(
        Tables::TicketInstances,
        Some(ticket.id),
        Some(DomainEventTypes::TicketInstanceNullified),
        connection,
    )
    .unwrap();
    assert_eq!(0, domain_events.len());
}

#[test]
fn release_for_cancelled_ticket_type() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let creator = project.create_user().finish();
    let organization = project.create_organization().with_fees().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_ticket_pricing()
        .finish();
    let user = project.create_user().finish();
    project
        .create_order()
        .for_event(&event)
        .for_user(&user)
        .quantity(1)
        .is_paid()
        .finish();
    let ticket = TicketInstance::find_for_user(user.id, connection).unwrap().remove(0);
    assert_eq!(ticket.status, TicketInstanceStatus::Purchased);
    let ticket_type = event.ticket_types(true, None, connection).unwrap().remove(0);
    ticket_type.cancel(connection).unwrap();

    TicketInstance::create_transfer(&user, &[ticket.id], None, None, false, connection).unwrap();
    assert!(ticket
        .release(TicketInstanceStatus::Purchased, creator.id, connection)
        .is_ok());

    // Reload ticket
    let ticket = TicketInstance::find(ticket.id, connection).unwrap();
    assert!(ticket.order_item_id.is_none());
    assert_eq!(ticket.status, TicketInstanceStatus::Nullified);

    // Ticket was nullified so domain event is created for nullification
    let domain_events = DomainEvent::find(
        Tables::TicketInstances,
        Some(ticket.id),
        Some(DomainEventTypes::TicketInstanceNullified),
        connection,
    )
    .unwrap();
    assert_eq!(1, domain_events.len());
}

#[test]
fn set_wallet() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().with_fees().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_ticket_pricing()
        .finish();
    let user = project.create_user().finish();
    let user2 = project.create_user().finish();

    let user_wallet = Wallet::find_default_for_user(user.id, connection).unwrap();
    let user2_wallet = Wallet::find_default_for_user(user2.id, connection).unwrap();
    project
        .create_order()
        .for_event(&event)
        .for_user(&user)
        .quantity(1)
        .is_paid()
        .finish();
    let ticket = TicketInstance::find_for_user(user.id, connection).unwrap().remove(0);
    assert_eq!(ticket.wallet_id, user_wallet.id);
    ticket.set_wallet(&user2_wallet, connection).unwrap();
    let ticket = TicketInstance::find(ticket.id, connection).unwrap();
    assert_eq!(ticket.wallet_id, user2_wallet.id);
}

#[test]
fn was_transferred() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().with_fees().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_ticket_pricing()
        .finish();
    let user = project.create_user().finish();
    let user2 = project.create_user().finish();
    project
        .create_order()
        .for_event(&event)
        .for_user(&user)
        .quantity(1)
        .is_paid()
        .finish();
    let ticket = TicketInstance::find_for_user(user.id, connection).unwrap().remove(0);

    // Not transferred
    assert!(!ticket.was_transferred(connection).unwrap());

    let sender_wallet = Wallet::find_default_for_user(user.id, connection).unwrap();
    let receiver_wallet = Wallet::find_default_for_user(user2.id, connection).unwrap();
    let transfer = TicketInstance::create_transfer(&user, &[ticket.id], None, None, false, connection).unwrap();
    TicketInstance::receive_ticket_transfer(
        transfer.into_authorization(connection).unwrap(),
        &sender_wallet,
        user2.id,
        receiver_wallet.id,
        connection,
    )
    .unwrap();

    // Transferred
    assert!(ticket.was_transferred(connection).unwrap());
}

#[test]
fn find() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().with_fees().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let user = project.create_user().finish();
    //let _d_user: DisplayUser = user.into();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    let ticket_pricing = ticket_type.current_ticket_pricing(false, connection).unwrap();

    let display_event = event.for_display(connection).unwrap();
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 1,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    let items = cart.items(&connection).unwrap();
    let order_item = items.iter().find(|i| i.ticket_type_id == Some(ticket_type.id)).unwrap();
    let fee_schedule_range = ticket_type
        .fee_schedule(connection)
        .unwrap()
        .get_range(ticket_pricing.price_in_cents, connection)
        .unwrap();
    let ticket = TicketInstance::find_for_order_item(order_item.id, connection)
        .unwrap()
        .remove(0);
    let expected_ticket = DisplayTicket {
        id: ticket.id,
        order_id: cart.id,
        price_in_cents: (ticket_pricing.price_in_cents + fee_schedule_range.fee_in_cents) as u32,
        ticket_type_id: ticket_type.id,
        ticket_type_name: ticket_type.name.clone(),
        status: TicketInstanceStatus::Reserved,
        redeem_key: ticket.redeem_key,
        pending_transfer: false,
        first_name_override: None,
        last_name_override: None,
        transfer_id: None,
        transfer_key: None,
        transfer_address: None,
        check_in_source: None,
    };
    assert_eq!(
        (display_event, None, expected_ticket),
        TicketInstance::find_for_display(ticket.id, connection).unwrap()
    );
    assert!(TicketInstance::find(Uuid::new_v4(), connection).is_err());
}

#[test]
fn find_show_no_token() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().with_fees().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .with_event_start(NaiveDate::from_ymd(3000, 7, 8).and_hms(9, 10, 11)) //we dont care about the date, it should only be longer than 24 hours from now
        .finish();
    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    let ticket_pricing = ticket_type.current_ticket_pricing(false, connection).unwrap();

    let display_event = event.clone().for_display(connection).unwrap();
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 1,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    let items = cart.items(&connection).unwrap();
    let order_item = items.iter().find(|i| i.ticket_type_id == Some(ticket_type.id)).unwrap();
    let fee_schedule_range = ticket_type
        .fee_schedule(connection)
        .unwrap()
        .get_range(ticket_pricing.price_in_cents, connection)
        .unwrap();
    project
        .create_order()
        .for_event(&event)
        .for_user(&user)
        .quantity(1)
        .is_paid()
        .finish();
    let ticket = TicketInstance::find_for_order_item(order_item.id, connection)
        .unwrap()
        .remove(0);
    let expected_ticket = DisplayTicket {
        id: ticket.id,
        order_id: cart.id,
        price_in_cents: (ticket_pricing.price_in_cents + fee_schedule_range.fee_in_cents) as u32,
        ticket_type_id: ticket_type.id,
        ticket_type_name: ticket_type.name.clone(),
        status: TicketInstanceStatus::Purchased,
        redeem_key: None,
        pending_transfer: false,
        first_name_override: None,
        last_name_override: None,
        transfer_id: None,
        transfer_key: None,
        transfer_address: None,
        check_in_source: None,
    };
    let (found_event, found_user, found_ticket) = TicketInstance::find_for_display(ticket.id, connection).unwrap();
    assert_eq!(
        (display_event.clone(), Some(user.into()), expected_ticket.clone()),
        (found_event, found_user, found_ticket.clone())
    );
    assert!(found_ticket.redeem_key.is_none(), true);

    //make redeem date in the past for the event
    let new_event_redeem_date = EventEditableAttributes {
        redeem_date: Some(NaiveDateTime::from(Utc::now().naive_utc() - Duration::days(2))),
        ..Default::default()
    };

    let _event = event.update(None, new_event_redeem_date, connection).unwrap();

    let (_found_event, _found_user, found_ticket) = TicketInstance::find_for_display(ticket.id, connection).unwrap();
    assert!(found_ticket.redeem_key.is_some(), true);
}

#[test]
fn find_for_user() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().with_fees().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();

    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 5,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();

    let total = cart.calculate_total(connection).unwrap();
    cart.add_external_payment(
        Some("test".to_string()),
        ExternalPaymentType::CreditCard,
        user.id,
        total,
        connection,
    )
    .unwrap();

    let tickets = TicketInstance::find_for_user(user.id, connection).unwrap();

    assert_eq!(tickets.len(), 5);
    assert!(TicketInstance::find(Uuid::new_v4(), connection).is_err());
}

#[test]
fn release_tickets() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project.create_event().with_ticket_pricing().finish();
    let user = project.create_user().finish();
    let mut order = Order::find_or_create_cart(&user, connection).unwrap();
    let ticket_type_id = event.ticket_types(true, None, connection).unwrap()[0].id;
    order
        .update_quantities(
            user.id,
            &[UpdateOrderItem {
                ticket_type_id,
                quantity: 10,
                redemption_code: None,
            }],
            false,
            false,
            connection,
        )
        .unwrap();

    let items = order.items(&connection).unwrap();
    let order_item = items.iter().find(|i| i.ticket_type_id == Some(ticket_type_id)).unwrap();

    // Release tickets
    let released_tickets = TicketInstance::release_tickets(&order_item, 4, Some(user.id), connection).unwrap();

    assert_eq!(released_tickets.len(), 4);
    assert!(released_tickets
        .iter()
        .filter(|&ticket| ticket.order_item_id == Some(order_item.id))
        .collect::<Vec<&TicketInstance>>()
        .is_empty());
    assert!(released_tickets
        .iter()
        .filter(|&ticket| ticket.reserved_until.is_some())
        .collect::<Vec<&TicketInstance>>()
        .is_empty());

    project
        .get_connection()
        .transaction::<Vec<TicketInstance>, Error, _>(|| {
            // Release requesting too many tickets
            let released_tickets = TicketInstance::release_tickets(&order_item, 7, Some(user.id), connection);
            assert_eq!(released_tickets.unwrap_err().code, 7200,);

            Err(Error::RollbackTransaction)
        })
        .unwrap_err();
}

#[test]
fn release_tickets_cancelled_ticket_type() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project.create_event().with_ticket_pricing().finish();
    let user = project.create_user().finish();
    let mut order = Order::find_or_create_cart(&user, connection).unwrap();
    let ticket_type = event.ticket_types(true, None, connection).unwrap().remove(0);
    order
        .update_quantities(
            user.id,
            &[UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 10,
                redemption_code: None,
            }],
            false,
            false,
            connection,
        )
        .unwrap();

    let items = order.items(&connection).unwrap();
    let order_item = items.iter().find(|i| i.ticket_type_id == Some(ticket_type.id)).unwrap();

    // Cancel ticket type
    ticket_type.cancel(connection).unwrap();
    let released_tickets = TicketInstance::release_tickets(&order_item, 4, Some(user.id), connection).unwrap();

    assert_eq!(released_tickets.len(), 4);
    assert!(released_tickets
        .iter()
        .filter(|&ticket| ticket.order_item_id == Some(order_item.id))
        .collect::<Vec<&TicketInstance>>()
        .is_empty());
    assert!(released_tickets
        .iter()
        .filter(|&ticket| ticket.reserved_until.is_some())
        .collect::<Vec<&TicketInstance>>()
        .is_empty());
    assert_eq!(
        4,
        released_tickets
            .iter()
            .filter(|&ticket| ticket.status == TicketInstanceStatus::Nullified)
            .collect::<Vec<&TicketInstance>>()
            .len()
    );

    // Nullified domain event should exist for each ticket
    for released_ticket in released_tickets {
        let domain_events = DomainEvent::find(
            Tables::TicketInstances,
            Some(released_ticket.id),
            Some(DomainEventTypes::TicketInstanceNullified),
            connection,
        )
        .unwrap();
        assert_eq!(1, domain_events.len());
    }
}

#[test]
fn mark_as_purchased() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().with_fees().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_ticket_pricing()
        .finish();
    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 1,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    let items = cart.items(&connection).unwrap();
    let order_item = items.iter().find(|i| i.ticket_type_id == Some(ticket_type.id)).unwrap();

    TicketInstance::mark_as_purchased(order_item, user.id, connection).unwrap();
    let ticket = TicketInstance::find_for_user(user.id, connection).unwrap().remove(0);

    let domain_events = DomainEvent::find(
        Tables::TicketInstances,
        Some(ticket.id),
        Some(DomainEventTypes::TicketInstancePurchased),
        connection,
    )
    .unwrap();
    assert_eq!(1, domain_events.len());
}

#[test]
fn redeem_ticket() {
    let project = TestProject::new();
    let admin = project.create_user().finish();

    let connection = project.get_connection();

    let organization = project.create_organization().with_fees().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_ticket_pricing()
        .finish();
    let user = project.create_user().finish();
    project
        .create_order()
        .for_event(&event)
        .for_user(&user)
        .quantity(3)
        .is_paid()
        .finish();
    let mut tickets = TicketInstance::find_for_user(user.id, connection).unwrap();
    let ticket = tickets.remove(0);
    let ticket2 = tickets.remove(0);
    let ticket3 = tickets.remove(0);

    // No domain events associated with redeeming for this ticket
    let domain_events = DomainEvent::find(
        Tables::TicketInstances,
        Some(ticket.id),
        Some(DomainEventTypes::TicketInstanceRedeemed),
        connection,
    )
    .unwrap();
    assert_eq!(0, domain_events.len());

    // Invalid key, does not redeem or create redeem event
    let result1 = TicketInstance::redeem_ticket(
        ticket.id,
        "WrongKey".to_string(),
        admin.id,
        CheckInSource::GuestList,
        connection,
    )
    .unwrap();
    assert_eq!(result1, RedeemResults::TicketInvalid);
    let domain_events = DomainEvent::find(
        Tables::TicketInstances,
        Some(ticket.id),
        Some(DomainEventTypes::TicketInstanceRedeemed),
        connection,
    )
    .unwrap();
    assert_eq!(0, domain_events.len());

    // Valid key, redeems and creates redeem event
    let result2 = TicketInstance::redeem_ticket(
        ticket.id,
        ticket.redeem_key.unwrap(),
        admin.id,
        CheckInSource::GuestList,
        connection,
    )
    .unwrap();
    assert_eq!(result2, RedeemResults::TicketRedeemSuccess);
    let domain_events = DomainEvent::find(
        Tables::TicketInstances,
        Some(ticket.id),
        Some(DomainEventTypes::TicketInstanceRedeemed),
        connection,
    )
    .unwrap();
    assert_eq!(1, domain_events.len());

    let ticket = TicketInstance::find(ticket.id, connection).unwrap();
    assert_eq!(ticket.check_in_source, Some(CheckInSource::GuestList));

    let result2 = TicketInstance::redeem_ticket(
        ticket2.id,
        ticket2.redeem_key.unwrap(),
        admin.id,
        CheckInSource::Scanned,
        connection,
    )
    .unwrap();
    assert_eq!(result2, RedeemResults::TicketRedeemSuccess);
    let domain_events = DomainEvent::find(
        Tables::TicketInstances,
        Some(ticket2.id),
        Some(DomainEventTypes::TicketInstanceRedeemed),
        connection,
    )
    .unwrap();
    assert_eq!(1, domain_events.len());
    let ticket2 = TicketInstance::find(ticket2.id, connection).unwrap();
    assert_eq!(ticket2.check_in_source, Some(CheckInSource::Scanned));

    // Cannot redeem a transferred ticket
    let transfer = TicketInstance::create_transfer(&user, &[ticket3.id], None, None, false, connection).unwrap();
    let result = TicketInstance::redeem_ticket(
        ticket3.id,
        ticket3.redeem_key.clone().unwrap(),
        admin.id,
        CheckInSource::GuestList,
        connection,
    )
    .unwrap();
    assert_eq!(result, RedeemResults::TicketTransferInProcess);

    // Cancel transfer, can redeem
    assert!(transfer.cancel(&user, None, connection).is_ok());
    let result = TicketInstance::redeem_ticket(
        ticket3.id,
        ticket3.redeem_key.unwrap(),
        admin.id,
        CheckInSource::GuestList,
        connection,
    )
    .unwrap();
    assert_eq!(result, RedeemResults::TicketRedeemSuccess);
}

#[test]
fn organization() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_ticket_pricing()
        .finish();
    let user = project.create_user().finish();
    project
        .create_order()
        .for_event(&event)
        .for_user(&user)
        .quantity(1)
        .is_paid()
        .finish();
    let ticket = TicketInstance::find_for_user(user.id, connection).unwrap().remove(0);
    assert_eq!(organization, ticket.organization(connection).unwrap());
}

#[test]
fn owner() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_ticket_pricing()
        .finish();
    let user = project.create_user().finish();
    let user2 = project.create_user().finish();
    let user3 = project.create_user().finish();
    project
        .create_order()
        .for_event(&event)
        .for_user(&user)
        .quantity(1)
        .is_paid()
        .finish();
    let ticket = TicketInstance::find_for_user(user.id, connection).unwrap().remove(0);
    assert_eq!(user, ticket.owner(connection).unwrap());

    // Transferred
    TicketInstance::direct_transfer(
        &user,
        &vec![ticket.id],
        "nowhere",
        TransferMessageType::Email,
        user2.id,
        connection,
    )
    .unwrap();
    assert_eq!(user2, ticket.owner(connection).unwrap());

    // Box office purchase
    let box_office_order = project
        .create_order()
        .quantity(1)
        .for_event(&event)
        .for_user(&user)
        .on_behalf_of_user(&user3)
        .is_paid()
        .finish();
    let ticket = box_office_order.tickets(None, connection).unwrap().pop().unwrap();
    assert_eq!(user3, ticket.owner(connection).unwrap());
}

#[test]
fn show_redeemable_ticket() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().with_fees().finish();
    let venue = project.create_venue().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_ticket_pricing()
        .with_venue(&venue)
        .finish();
    let user = project.create_user().finish();
    project
        .create_order()
        .for_event(&event)
        .for_user(&user)
        .quantity(1)
        .is_paid()
        .finish();
    let ticket = TicketInstance::find_for_user(user.id, connection).unwrap().remove(0);

    //make redeem date in the future for an event in 4 days time
    let new_event_redeem_date = EventEditableAttributes {
        redeem_date: Some(NaiveDateTime::from(Utc::now().naive_utc() + Duration::days(2))),
        event_start: Some(NaiveDateTime::from(Utc::now().naive_utc() + Duration::days(4))),
        event_end: Some(NaiveDateTime::from(Utc::now().naive_utc() + Duration::days(5))),
        ..Default::default()
    };

    let event = event.update(None, new_event_redeem_date, connection).unwrap();

    let result = TicketInstance::show_redeemable_ticket(ticket.id, connection).unwrap();
    assert!(result.redeem_key.is_none());

    //make redeem date in the past for an event in 4 days time
    let new_event_redeem_date = EventEditableAttributes {
        redeem_date: Some(NaiveDateTime::from(Utc::now().naive_utc() - Duration::days(2))),
        event_start: Some(NaiveDateTime::from(Utc::now().naive_utc() + Duration::days(4))),
        event_end: Some(NaiveDateTime::from(Utc::now().naive_utc() + Duration::days(5))),
        ..Default::default()
    };

    let event = event.update(None, new_event_redeem_date, connection).unwrap();

    let result = TicketInstance::show_redeemable_ticket(ticket.id, connection).unwrap();
    assert!(result.redeem_key.is_some());

    //make redeem date 12 hours from now, event starts in 24 hours from now
    let event_start = NaiveDateTime::from(Utc::now().naive_utc() + Duration::hours(24));
    let new_event_redeem_date = EventEditableAttributes {
        redeem_date: Some(NaiveDateTime::from(Utc::now().naive_utc() + Duration::hours(12))),
        event_start: Some(event_start.clone()),
        event_end: Some(NaiveDateTime::from(event_start + Duration::hours(24))),
        ..Default::default()
    };

    let event = event.update(None, new_event_redeem_date, connection).unwrap();

    let result = TicketInstance::show_redeemable_ticket(ticket.id, connection).unwrap();
    assert!(result.redeem_key.is_some());

    // no redeem_date set, event starts 24 hours from now
    let event_start = NaiveDateTime::from(Utc::now().naive_utc() + Duration::hours(24));
    let new_event_redeem_date = EventEditableAttributes {
        redeem_date: None,
        event_start: Some(event_start),
        event_end: Some(NaiveDateTime::from(event_start + Duration::hours(24))),
        ..Default::default()
    };

    let event = event.update(None, new_event_redeem_date, connection).unwrap();

    let result = TicketInstance::show_redeemable_ticket(ticket.id, connection).unwrap();
    assert!(result.redeem_key.is_some());

    // Set order on behalf of (should show user information for the on_behalf_of_user user)
    let user2 = project.create_user().finish();
    let order = project
        .create_order()
        .for_event(&event)
        .for_user(&user)
        .on_behalf_of_user(&user2)
        .quantity(1)
        .is_paid()
        .finish();
    let ticket = order.tickets(None, connection).unwrap().remove(0);
    let result = TicketInstance::show_redeemable_ticket(ticket.id, connection).unwrap();
    assert_eq!(result.user_id, Some(user2.id));
}

#[test]
fn create_transfer() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().with_fees().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();

    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 5,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    let total = cart.calculate_total(connection).unwrap();

    cart.add_external_payment(
        Some("test".to_string()),
        ExternalPaymentType::CreditCard,
        user.id,
        total,
        connection,
    )
    .unwrap();

    let tickets = TicketInstance::find_for_user(user.id, connection).unwrap();

    assert_eq!(tickets.len(), 5);
    //try with a ticket that does not exist in the list

    let mut tickets = TicketInstance::find_for_user(user.id, connection).unwrap();

    let mut ticket_ids: Vec<Uuid> = tickets.iter().map(|t| t.id).collect();
    ticket_ids.push(Uuid::new_v4());

    let transfer = TicketInstance::create_transfer(&user, &ticket_ids, None, None, false, connection);
    assert!(transfer.is_err());

    //Now try with tickets that the user does own
    let ticket_ids: Vec<Uuid> = tickets.iter().map(|t| t.id).collect();
    let transfer2 = TicketInstance::create_transfer(&user, &ticket_ids, None, None, false, connection).unwrap();
    assert_eq!(transfer2.source_user_id, user.id);
    assert!(!transfer2.direct);

    let domain_events = DomainEvent::find(
        Tables::Transfers,
        Some(transfer2.id),
        Some(DomainEventTypes::TransferTicketStarted),
        connection,
    )
    .unwrap();
    assert_eq!(1, domain_events.len());

    // Can't transfer a redeemed ticket
    transfer2.cancel(&user, None, connection).unwrap();
    let ticket = tickets.remove(0);
    TicketInstance::redeem_ticket(
        ticket.id,
        ticket.redeem_key.clone().unwrap(),
        user.id,
        CheckInSource::GuestList,
        connection,
    )
    .unwrap();
    let result = TicketInstance::create_transfer(&user, &ticket_ids, None, None, false, connection);
    assert_eq!(
        result,
        Err(DatabaseError::new(
            ErrorCode::BusinessProcessError,
            Some("Redeemed tickets cannot be transferred".to_string()),
        ))
    );

    // Event ended cannot create transfer
    let ticket_ids: Vec<Uuid> = tickets.iter().map(|t| t.id).collect();
    diesel::sql_query(
        r#"
        UPDATE events
        SET event_start = $1,
        event_end = $2
        WHERE id = $3;
        "#,
    )
    .bind::<sql_types::Timestamp, _>(dates::now().add_days(-7).finish())
    .bind::<sql_types::Timestamp, _>(dates::now().add_days(-1).finish())
    .bind::<sql_types::Uuid, _>(event.id)
    .execute(connection)
    .unwrap();
    let result = TicketInstance::create_transfer(&user, &ticket_ids, None, None, false, connection);
    assert_eq!(
        result,
        Err(DatabaseError::new(
            ErrorCode::BusinessProcessError,
            Some("Cannot transfer ticket, event has ended.".to_string()),
        ))
    );
}

#[test]
fn has_pending_transfer() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let mut user = project.create_user().finish();
    user = user.add_role(Roles::Super, connection).unwrap();
    let user2 = project.create_user().finish();
    let event = project.create_event().with_tickets().finish();
    project
        .create_order()
        .for_event(&event)
        .for_user(&user)
        .is_paid()
        .finish();
    let user_tickets = TicketInstance::find_for_user(user.id, connection).unwrap();
    let ticket = &user_tickets[0];
    assert!(!ticket.has_pending_transfer(connection).unwrap());

    // With pending transfer
    let transfer = TicketInstance::create_transfer(&user, &[ticket.id], None, None, false, connection).unwrap();
    assert!(ticket.has_pending_transfer(connection).unwrap());

    // With cancelled transfer
    transfer.cancel(&user, None, connection).unwrap();
    assert!(!ticket.has_pending_transfer(connection).unwrap());

    // With completed direct transfer
    TicketInstance::direct_transfer(
        &user,
        &[ticket.id],
        "nowhere",
        TransferMessageType::Email,
        user2.id,
        connection,
    )
    .unwrap();
    assert!(!ticket.has_pending_transfer(connection).unwrap());

    // User 2 retransfers
    TicketInstance::create_transfer(&user2, &[ticket.id], None, None, false, connection).unwrap();
    assert!(ticket.has_pending_transfer(connection).unwrap());
}

#[test]
fn receive_ticket_transfer() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().with_fees().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let artist = project.create_artist().finish();
    project
        .create_event_artist()
        .with_event(&event)
        .with_artist(&artist)
        .finish();
    artist
        .set_genres(&vec!["emo".to_string(), "hard-rock".to_string()], None, connection)
        .unwrap();
    event.update_genres(None, connection).unwrap();

    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 5,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    let total = cart.calculate_total(connection).unwrap();

    cart.add_external_payment(
        Some("test".to_string()),
        ExternalPaymentType::CreditCard,
        user.id,
        total,
        connection,
    )
    .unwrap();
    let updated_ticket = TicketInstance::find_for_user(user.id, connection)
        .unwrap()
        .pop()
        .unwrap()
        .update(
            UpdateTicketInstanceAttributes {
                first_name_override: Some(Some("Janus".to_string())),
                last_name_override: Some(Some("Zeal".to_string())),
            },
            user.id,
            connection,
        )
        .unwrap();
    assert_eq!(updated_ticket.first_name_override, Some("Janus".to_string()));
    assert_eq!(updated_ticket.last_name_override, Some("Zeal".to_string()));

    let tickets = TicketInstance::find_for_user(user.id, connection).unwrap();
    let ticket_ids: Vec<Uuid> = tickets.iter().map(|t| t.id).collect();

    let user2 = project.create_user().finish();
    let sender_wallet = Wallet::find_default_for_user(user.id, connection).unwrap();
    let receiver_wallet = Wallet::find_default_for_user(user2.id, connection).unwrap();

    //try receive the wrong number of tickets (too few)
    let transfer = TicketInstance::create_transfer(&user, &ticket_ids, None, None, false, connection).unwrap();

    let mut wrong_auth: TransferAuthorization = transfer.clone().into_authorization(connection).unwrap();
    wrong_auth.num_tickets = 4;
    let receive_auth =
        TicketInstance::receive_ticket_transfer(wrong_auth, &sender_wallet, user2.id, receiver_wallet.id, connection);
    assert!(receive_auth.is_err());
    let reloaded_ticket = TicketInstance::find(updated_ticket.id, connection).unwrap();
    assert_eq!(reloaded_ticket.first_name_override, Some("Janus".to_string()));
    assert_eq!(reloaded_ticket.last_name_override, Some("Zeal".to_string()));

    // Genres prior to transfer
    assert_eq!(
        user.genres(connection).unwrap(),
        vec!["emo".to_string(), "hard-rock".to_string()]
    );
    assert!(user2.genres(connection).unwrap().is_empty());

    //legit receive tickets
    TicketInstance::receive_ticket_transfer(
        transfer.into_authorization(connection).unwrap(),
        &sender_wallet,
        user2.id,
        receiver_wallet.id,
        connection,
    )
    .unwrap();

    // Genres have moved to user2
    assert!(user.genres(connection).unwrap().is_empty());
    assert_eq!(
        user2.genres(connection).unwrap(),
        vec!["emo".to_string(), "hard-rock".to_string()]
    );

    //Look if one of the tickets does have the new wallet_id
    let receive_wallet = Wallet::find_default_for_user(user2.id, connection).unwrap();
    let reloaded_ticket = TicketInstance::find(updated_ticket.id, connection).unwrap();
    assert_eq!(reloaded_ticket.wallet_id, receive_wallet.id);

    // Transferred tickets have their name overrides cleared
    assert_eq!(reloaded_ticket.first_name_override, None);
    assert_eq!(reloaded_ticket.last_name_override, None);

    // Event has ended, cannot accept ticket transfer
    let sender_wallet = Wallet::find_default_for_user(user2.id, connection).unwrap();
    let receiver_wallet = Wallet::find_default_for_user(user.id, connection).unwrap();
    let transfer =
        TicketInstance::create_transfer(&user2, &[reloaded_ticket.id], None, None, false, connection).unwrap();
    diesel::sql_query(
        r#"
        UPDATE events
        SET event_start = $1,
        event_end = $2
        WHERE id = $3;
        "#,
    )
    .bind::<sql_types::Timestamp, _>(dates::now().add_days(-7).finish())
    .bind::<sql_types::Timestamp, _>(dates::now().add_days(-1).finish())
    .bind::<sql_types::Uuid, _>(event.id)
    .execute(connection)
    .unwrap();
    let result = TicketInstance::receive_ticket_transfer(
        transfer.into_authorization(connection).unwrap(),
        &sender_wallet,
        user.id,
        receiver_wallet.id,
        connection,
    );
    assert_eq!(
        result,
        Err(DatabaseError::new(
            ErrorCode::BusinessProcessError,
            Some("Cannot transfer ticket, event has ended.".to_string()),
        ))
    );
}

#[test]
fn transfer_to_existing_user() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().with_fees().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let artist = project.create_artist().finish();
    project
        .create_event_artist()
        .with_event(&event)
        .with_artist(&artist)
        .finish();
    artist
        .set_genres(&vec!["emo".to_string(), "hard-rock".to_string()], None, connection)
        .unwrap();
    event.update_genres(None, connection).unwrap();

    let original_purchaser = project.create_user().finish();
    let receiver = project.create_user().finish();

    let _order = project
        .create_order()
        .for_event(&event)
        .for_user(&original_purchaser)
        .quantity(5)
        .is_paid()
        .finish();
    let mut ticket_ids: Vec<Uuid> = TicketInstance::find_for_user(original_purchaser.id, connection)
        .unwrap()
        .into_iter()
        .map(|ti| ti.id)
        .collect();
    ticket_ids.sort();

    // Genres prior to transfer
    assert_eq!(
        original_purchaser.genres(connection).unwrap(),
        vec!["emo".to_string(), "hard-rock".to_string()]
    );
    assert!(receiver.genres(connection).unwrap().is_empty());

    let transfer = TicketInstance::direct_transfer(
        &original_purchaser,
        &ticket_ids,
        "nowhere",
        TransferMessageType::Email,
        receiver.id,
        connection,
    )
    .unwrap();
    assert!(transfer.direct);
    let mut transfer_ticket_ticket_ids: Vec<Uuid> = transfer
        .transfer_tickets(connection)
        .unwrap()
        .into_iter()
        .map(|ti| ti.ticket_instance_id)
        .collect();
    transfer_ticket_ticket_ids.sort();
    assert_eq!(transfer.source_user_id, original_purchaser.id);
    assert_eq!(transfer.destination_user_id, Some(receiver.id));
    assert_eq!(transfer_ticket_ticket_ids, ticket_ids);

    // Genres updated with tickets now transferred
    assert!(original_purchaser.genres(connection).unwrap().is_empty());
    assert_eq!(
        receiver.genres(connection).unwrap(),
        vec!["emo".to_string(), "hard-rock".to_string()]
    );
}
