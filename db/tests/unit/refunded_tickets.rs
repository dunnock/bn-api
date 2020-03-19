use db::dev::TestProject;
use db::prelude::*;

#[test]
fn mark_fee_only_refunded() {
    let project = TestProject::new();
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
        .quantity(1)
        .is_paid()
        .finish();
    let tickets = TicketInstance::find_for_user(user.id, connection).unwrap();
    let ticket = &tickets[0];
    let mut refunded_ticket = RefundedTicket::find_or_create_by_ticket_instance(&ticket, connection).unwrap();
    assert!(refunded_ticket.ticket_refunded_at.is_none());
    assert!(refunded_ticket.fee_refunded_at.is_none());
    refunded_ticket.mark_fee_only_refunded(connection).unwrap();

    let refunded_ticket = RefundedTicket::find_or_create_by_ticket_instance(&ticket, connection).unwrap();
    assert!(refunded_ticket.ticket_refunded_at.is_none());
    assert!(refunded_ticket.fee_refunded_at.is_some());
}

#[test]
fn find_and_find_by_ticket_instance_ids() {
    let project = TestProject::new();
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
        .quantity(1)
        .is_paid()
        .finish();
    let tickets = TicketInstance::find_for_user(user.id, connection).unwrap();
    let ticket = &tickets[0];
    let refunded_ticket = RefundedTicket::find_or_create_by_ticket_instance(&ticket, connection).unwrap();
    assert_eq!(
        refunded_ticket,
        RefundedTicket::find(refunded_ticket.id, connection).unwrap()
    );
    let found_tickets = RefundedTicket::find_by_ticket_instance_ids(vec![ticket.id], connection).unwrap();
    assert_eq!(found_tickets, vec![refunded_ticket]);
}

#[test]
fn find_or_create_by_ticket_instance() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().with_fees().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_ticket_pricing()
        .with_a_specific_number_of_tickets(1)
        .finish();
    let user = project.create_user().finish();
    let mut order = project
        .create_order()
        .for_event(&event)
        .for_user(&user)
        .quantity(1)
        .is_paid()
        .finish();
    let tickets = TicketInstance::find_for_user(user.id, connection).unwrap();
    let ticket = &tickets[0];
    let refunded_ticket = RefundedTicket::find_or_create_by_ticket_instance(&ticket, connection).unwrap();
    assert_eq!(ticket.order_item_id, Some(refunded_ticket.order_item_id));
    assert_eq!(ticket.id, refunded_ticket.ticket_instance_id);
    assert!(refunded_ticket.ticket_refunded_at.is_none());
    assert!(refunded_ticket.fee_refunded_at.is_none());

    let refund_items = vec![RefundItemRequest {
        order_item_id: refunded_ticket.order_item_id,
        ticket_instance_id: Some(ticket.id),
    }];
    assert!(order.refund(&refund_items, user.id, None, false, connection).is_ok());

    let refunded_ticket = RefundedTicket::find(refunded_ticket.id, connection).unwrap();
    assert_eq!(ticket.order_item_id, Some(refunded_ticket.order_item_id));
    assert_eq!(ticket.id, refunded_ticket.ticket_instance_id);
    assert!(refunded_ticket.ticket_refunded_at.is_some());
    assert!(refunded_ticket.fee_refunded_at.is_some());

    project
        .create_order()
        .for_event(&event)
        .for_user(&user)
        .quantity(1)
        .is_paid()
        .finish();
    let tickets = TicketInstance::find_for_user(user.id, connection).unwrap();
    let ticket = &tickets[0];

    let refunded_ticket2 = RefundedTicket::find_or_create_by_ticket_instance(&ticket, connection).unwrap();
    assert_ne!(refunded_ticket, refunded_ticket2);
    assert_ne!(refunded_ticket.order_item_id, refunded_ticket2.order_item_id);
    assert_eq!(refunded_ticket.ticket_instance_id, refunded_ticket2.ticket_instance_id);
}

#[test]
fn mark_refunded() {
    let project = TestProject::new();
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
        .quantity(2)
        .is_paid()
        .finish();
    let tickets = TicketInstance::find_for_user(user.id, connection).unwrap();
    let ticket = &tickets[0];
    let mut refunded_ticket = RefundedTicket::create(ticket.order_item_id.unwrap(), ticket.id)
        .commit(connection)
        .unwrap();

    assert_eq!(ticket.order_item_id, Some(refunded_ticket.order_item_id));
    assert_eq!(ticket.id, refunded_ticket.ticket_instance_id);
    assert!(refunded_ticket.ticket_refunded_at.is_none());
    assert!(refunded_ticket.fee_refunded_at.is_none());

    // Refunding fee and subsequently refunding just the ticket fee
    refunded_ticket.mark_refunded(true, connection).unwrap();
    assert!(refunded_ticket.ticket_refunded_at.is_none());
    assert!(refunded_ticket.fee_refunded_at.is_some());

    // Refunding ticket as well
    refunded_ticket.mark_refunded(false, connection).unwrap();
    assert!(refunded_ticket.ticket_refunded_at.is_some());
    assert!(refunded_ticket.fee_refunded_at.is_some());

    // Refunding both ticket and fee at once
    let ticket2 = &tickets[1];
    let mut refunded_ticket = RefundedTicket::create(ticket2.order_item_id.unwrap(), ticket2.id)
        .commit(connection)
        .unwrap();
    assert_eq!(ticket.order_item_id, Some(refunded_ticket.order_item_id));
    assert_eq!(ticket2.id, refunded_ticket.ticket_instance_id);
    assert!(refunded_ticket.ticket_refunded_at.is_none());
    assert!(refunded_ticket.fee_refunded_at.is_none());
    refunded_ticket.mark_refunded(false, connection).unwrap();
    assert!(refunded_ticket.ticket_refunded_at.is_some());
    assert!(refunded_ticket.fee_refunded_at.is_some());
}
