use chrono::prelude::*;
use db::dev::TestProject;
use db::prelude::*;
use diesel;
use diesel::sql_types;
use diesel::PgConnection;
use diesel::RunQueryDsl;
use uuid::Uuid;

#[test]
fn transfer_eligible_for_cancelling() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let mut user = project.create_user().finish();
    user = user.add_role(Roles::Super, connection).unwrap();
    let user2 = project.create_user().finish();
    let user3 = project.create_user().finish();
    let event = project
        .create_event()
        .with_ticket_type_count(1)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    project
        .create_order()
        .for_event(&event)
        .for_user(&user)
        .quantity(3)
        .is_paid()
        .finish();
    let user_tickets = TicketInstance::find_for_user(user.id, connection).unwrap();
    // Transferred
    let ticket = &user_tickets[0];
    // Transferred
    let ticket2 = &user_tickets[1];
    // Transferred and later redeemed
    let ticket3 = &user_tickets[2];

    // Completed transfer
    let transfer = TicketInstance::direct_transfer(
        &user,
        &vec![ticket.id, ticket2.id],
        "nowhere",
        TransferMessageType::Email,
        user2.id,
        connection,
    )
    .unwrap();

    let transfer2 = TicketInstance::direct_transfer(
        &user,
        &vec![ticket3.id],
        "nowhere",
        TransferMessageType::Email,
        user2.id,
        connection,
    )
    .unwrap();
    diesel::sql_query(
        r#"
        UPDATE transfers
        SET created_at = $1
        WHERE id = $2;
        "#,
    )
    .bind::<sql_types::Timestamp, _>(dates::now().add_seconds(10).finish())
    .bind::<sql_types::Uuid, _>(transfer2.id)
    .execute(connection)
    .unwrap();

    // Pending transfer for first ticket
    let transfer3 = TicketInstance::create_transfer(&user2, &[ticket.id], None, None, false, connection).unwrap();
    diesel::sql_query(
        r#"
        UPDATE transfers
        SET created_at = $1
        WHERE id = $2;
        "#,
    )
    .bind::<sql_types::Timestamp, _>(dates::now().add_seconds(20).finish())
    .bind::<sql_types::Uuid, _>(transfer3.id)
    .execute(connection)
    .unwrap();

    // Completed transfer for second ticket
    let transfer4 = TicketInstance::direct_transfer(
        &user2,
        &vec![ticket2.id],
        "nowhere",
        TransferMessageType::Email,
        user3.id,
        connection,
    )
    .unwrap();
    diesel::sql_query(
        r#"
        UPDATE transfers
        SET created_at = $1
        WHERE id = $2;
        "#,
    )
    .bind::<sql_types::Timestamp, _>(dates::now().add_seconds(30).finish())
    .bind::<sql_types::Uuid, _>(transfer4.id)
    .execute(connection)
    .unwrap();

    // Redeem ticket 3
    let ticket3 = TicketInstance::find(ticket3.id, connection).unwrap();
    TicketInstance::redeem_ticket(
        ticket3.id,
        ticket3.redeem_key.clone().unwrap(),
        user2.id,
        CheckInSource::GuestList,
        connection,
    )
    .unwrap();

    let activity_items =
        ActivityItem::load_for_event(event.id, user.id, Some(ActivityType::Transfer), connection).unwrap();
    let activity_items = &activity_items_eligibility(activity_items);
    let activity_item = activity_items.iter().find(|ai| ai.0 == transfer.id).unwrap();
    assert_eq!(activity_item.1, false);
    let activity_item = activity_items.iter().find(|ai| ai.0 == transfer2.id).unwrap();
    assert_eq!(activity_item.1, false);
    let activity_items =
        ActivityItem::load_for_event(event.id, user2.id, Some(ActivityType::Transfer), connection).unwrap();
    let activity_items = &activity_items_eligibility(activity_items);
    let activity_item = activity_items.iter().find(|ai| ai.0 == transfer.id).unwrap();
    assert_eq!(activity_item.1, false);
    let activity_item = activity_items.iter().find(|ai| ai.0 == transfer2.id).unwrap();
    assert_eq!(activity_item.1, false);
    let activity_item = activity_items.iter().find(|ai| ai.0 == transfer3.id).unwrap();
    assert_eq!(activity_item.1, true);
    let activity_item = activity_items.iter().find(|ai| ai.0 == transfer4.id).unwrap();
    assert_eq!(activity_item.1, true);
    let activity_items =
        ActivityItem::load_for_event(event.id, user3.id, Some(ActivityType::Transfer), connection).unwrap();
    let activity_items = &activity_items_eligibility(activity_items);
    let activity_item = activity_items.iter().find(|ai| ai.0 == transfer4.id).unwrap();
    assert_eq!(activity_item.1, true);

    // Cancel transfer 3
    assert!(transfer3.cancel(&user, None, connection).is_ok());
    let activity_items =
        ActivityItem::load_for_event(event.id, user.id, Some(ActivityType::Transfer), connection).unwrap();
    let activity_items = &activity_items_eligibility(activity_items);
    let activity_item = activity_items.iter().find(|ai| ai.0 == transfer.id).unwrap();
    assert_eq!(activity_item.1, false);
    let activity_item = activity_items.iter().find(|ai| ai.0 == transfer2.id).unwrap();
    assert_eq!(activity_item.1, false);
    let activity_items =
        ActivityItem::load_for_event(event.id, user2.id, Some(ActivityType::Transfer), connection).unwrap();
    let activity_items = &activity_items_eligibility(activity_items);
    let activity_item = activity_items.iter().find(|ai| ai.0 == transfer.id).unwrap();
    assert_eq!(activity_item.1, false);
    let activity_item = activity_items.iter().find(|ai| ai.0 == transfer2.id).unwrap();
    assert_eq!(activity_item.1, false);
    let activity_item = activity_items.iter().find(|ai| ai.0 == transfer3.id).unwrap();
    assert_eq!(activity_item.1, false);
    let activity_item = activity_items.iter().find(|ai| ai.0 == transfer4.id).unwrap();
    assert_eq!(activity_item.1, true);
    let activity_items =
        ActivityItem::load_for_event(event.id, user3.id, Some(ActivityType::Transfer), connection).unwrap();
    let activity_items = &activity_items_eligibility(activity_items);
    let activity_item = activity_items.iter().find(|ai| ai.0 == transfer4.id).unwrap();
    assert_eq!(activity_item.1, true);

    // Cancel transfer 4
    assert!(transfer4.cancel(&user, None, connection).is_ok());
    let activity_items =
        ActivityItem::load_for_event(event.id, user.id, Some(ActivityType::Transfer), connection).unwrap();
    let activity_items = &activity_items_eligibility(activity_items);
    let activity_item = activity_items.iter().find(|ai| ai.0 == transfer.id).unwrap();
    assert_eq!(activity_item.1, true);
    let activity_item = activity_items.iter().find(|ai| ai.0 == transfer2.id).unwrap();
    assert_eq!(activity_item.1, false);
    let activity_items =
        ActivityItem::load_for_event(event.id, user2.id, Some(ActivityType::Transfer), connection).unwrap();
    let activity_items = &activity_items_eligibility(activity_items);
    let activity_item = activity_items.iter().find(|ai| ai.0 == transfer.id).unwrap();
    assert_eq!(activity_item.1, true);
    let activity_item = activity_items.iter().find(|ai| ai.0 == transfer2.id).unwrap();
    assert_eq!(activity_item.1, false);
    let activity_item = activity_items.iter().find(|ai| ai.0 == transfer3.id).unwrap();
    assert_eq!(activity_item.1, false);
    let activity_item = activity_items.iter().find(|ai| ai.0 == transfer4.id).unwrap();
    assert_eq!(activity_item.1, false);
    let activity_items =
        ActivityItem::load_for_event(event.id, user3.id, Some(ActivityType::Transfer), connection).unwrap();
    let activity_items = &activity_items_eligibility(activity_items);
    let activity_item = activity_items.iter().find(|ai| ai.0 == transfer4.id).unwrap();
    assert_eq!(activity_item.1, false);
    assert!(transfer.cancel(&user, None, connection).is_ok());
}

fn activity_items_eligibility(activity_items: Vec<ActivityItem>) -> Vec<(Uuid, bool)> {
    activity_items
        .into_iter()
        .map(|ai| {
            if let ActivityItem::Transfer {
                transfer_id,
                eligible_for_cancelling,
                ..
            } = ai
            {
                Some((transfer_id, eligible_for_cancelling))
            } else {
                None
            }
        })
        .map(|ai| ai.unwrap())
        .collect()
}

#[test]
fn load_for_event() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let user2 = project.create_user().finish();
    let user3 = project.create_user().finish();
    let user4 = project.create_user().finish();
    let organization = project.create_organization().with_event_fee().with_fees().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_ticket_type_count(1)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let ticket_types = &event.ticket_types(true, None, connection).unwrap();
    let event2 = project.create_event().with_ticket_pricing().finish();
    let hold = project
        .create_hold()
        .with_hold_type(HoldTypes::Discount)
        .with_quantity(10)
        .with_ticket_type_id(ticket_types[0].id)
        .finish();
    let code = project
        .create_code()
        .with_event(&event)
        .with_code_type(CodeTypes::Discount)
        .for_ticket_type(&ticket_types[0])
        .with_discount_in_cents(Some(10))
        .finish();
    let order = project
        .create_order()
        .for_event(&event)
        .on_behalf_of_user(&user)
        .for_user(&user4)
        .quantity(2)
        .with_redemption_code(hold.redemption_code.clone().unwrap())
        .is_paid()
        .finish();
    let order2 = project
        .create_order()
        .for_event(&event)
        .for_user(&user)
        .quantity(3)
        .with_redemption_code(code.redemption_code.clone())
        .is_paid()
        .finish();
    let order3 = project
        .create_order()
        .for_event(&event)
        .for_user(&user2)
        .quantity(3)
        .is_paid()
        .finish();
    let order4 = project
        .create_order()
        .for_event(&event2)
        .for_user(&user2)
        .quantity(2)
        .is_paid()
        .finish();

    let user_tickets = order.tickets(None, connection).unwrap();
    // Transferred
    let ticket = &user_tickets[0];
    // Redeemed
    let ticket2 = &user_tickets[1];

    let user_tickets = order2.tickets(None, connection).unwrap();
    // Refund
    let ticket3 = &user_tickets[0];
    // Refund
    let ticket4 = &user_tickets[1];

    let user_tickets = order3.tickets(None, connection).unwrap();
    // Transfer
    let ticket5 = &user_tickets[0];
    // Redeemed
    let ticket6 = &user_tickets[1];
    // Refund
    let ticket7 = &user_tickets[2];
    let user_tickets = order4.tickets(None, connection).unwrap();
    // Cancelled transfer
    let ticket8 = &user_tickets[0];

    // Completed transfer
    let transfer = TicketInstance::direct_transfer(
        &user,
        &vec![ticket.id],
        "nowhere",
        TransferMessageType::Email,
        user4.id,
        connection,
    )
    .unwrap();

    // Pending transfer
    let transfer2 = TicketInstance::create_transfer(&user2, &[ticket5.id], None, None, false, connection).unwrap();

    // Cancelled transfer
    let transfer3 = TicketInstance::create_transfer(&user2, &[ticket8.id], None, None, false, connection).unwrap();
    let transfer3 = transfer3.cancel(&user4, None, connection).unwrap();

    TicketInstance::redeem_ticket(
        ticket2.id,
        ticket2.redeem_key.clone().unwrap(),
        user3.id,
        CheckInSource::GuestList,
        connection,
    )
    .unwrap();
    let ticket2 = TicketInstance::find(ticket2.id, connection).unwrap();
    TicketInstance::redeem_ticket(
        ticket6.id,
        ticket6.redeem_key.clone().unwrap(),
        user3.id,
        CheckInSource::GuestList,
        connection,
    )
    .unwrap();
    let ticket6 = TicketInstance::find(ticket6.id, connection).unwrap();

    let mut refunding_order = Order::find(
        OrderItem::find(ticket3.order_item_id.unwrap(), connection)
            .unwrap()
            .order_id,
        connection,
    )
    .unwrap();
    let refund_items = vec![RefundItemRequest {
        order_item_id: ticket3.order_item_id.unwrap(),
        ticket_instance_id: Some(ticket3.id),
    }];

    let refund = refunding_order
        .refund(&refund_items, user.id, None, false, connection)
        .unwrap();

    let mut refunding_order2 = Order::find(
        OrderItem::find(ticket4.order_item_id.unwrap(), connection)
            .unwrap()
            .order_id,
        connection,
    )
    .unwrap();
    let refund_items = vec![RefundItemRequest {
        order_item_id: ticket4.order_item_id.unwrap(),
        ticket_instance_id: Some(ticket4.id),
    }];
    let refund2 = refunding_order2
        .refund(&refund_items, user3.id, None, false, connection)
        .unwrap();

    let mut refunding_order3 = Order::find(
        OrderItem::find(ticket7.order_item_id.unwrap(), connection)
            .unwrap()
            .order_id,
        connection,
    )
    .unwrap();
    let refund_items = vec![RefundItemRequest {
        order_item_id: ticket7.order_item_id.unwrap(),
        ticket_instance_id: Some(ticket7.id),
    }];
    let refund3 = refunding_order3
        .refund(&refund_items, user3.id, None, false, connection)
        .unwrap();

    let note = order
        .create_note("Client will pick up at 18h00".to_string(), user3.id, connection)
        .unwrap();
    let note2 = order
        .create_note("Client will pick up at 16h00".to_string(), user4.id, connection)
        .unwrap();
    let note3 = order2
        .create_note("Client will pick up at 11h00".to_string(), user3.id, connection)
        .unwrap();
    let note4 = order3
        .create_note("Client will pick up at 14h00".to_string(), user4.id, connection)
        .unwrap();

    let activity_items = ActivityItem::load_for_event(event.id, user.id, None, connection).unwrap();
    let activity_items2 = ActivityItem::load_for_event(event.id, user2.id, None, connection).unwrap();
    let activity_items3 = ActivityItem::load_for_event(event.id, user3.id, None, connection).unwrap();
    let activity_items4 = ActivityItem::load_for_event(event.id, user4.id, None, connection).unwrap();
    let activity_items5 = ActivityItem::load_for_event(event2.id, user.id, None, connection).unwrap();
    let activity_items6 = ActivityItem::load_for_event(event2.id, user2.id, None, connection).unwrap();
    let activity_items7 = ActivityItem::load_for_event(event2.id, user3.id, None, connection).unwrap();
    let activity_items8 = ActivityItem::load_for_event(event2.id, user4.id, None, connection).unwrap();
    assert_eq!(
        (
            activity_items.len(),
            activity_items2.len(),
            activity_items3.len(),
            activity_items4.len(),
            activity_items5.len(),
            activity_items6.len(),
            activity_items7.len(),
            activity_items8.len(),
        ),
        (10, 5, 0, 1, 0, 3, 0, 0)
    );

    let mut expected_results: Vec<(String, Uuid, Option<String>)> = Vec::new();
    expected_results.push(("CheckIn".to_string(), ticket2.id, None));
    expected_results.push(("Note".to_string(), note2.id, None));
    expected_results.push(("Note".to_string(), note.id, None));
    expected_results.push(("Note".to_string(), note3.id, None));
    expected_results.push(("Purchase".to_string(), order.id, None));
    expected_results.push(("Purchase".to_string(), order2.id, None));
    expected_results.push(("Refund".to_string(), refund.0.id, None));
    expected_results.push(("Refund".to_string(), refund2.0.id, None));
    expected_results.push(("Transfer".to_string(), transfer.id, Some("Accepted".to_string())));
    expected_results.push(("Transfer".to_string(), transfer.id, Some("Started".to_string())));
    expected_results.sort_by_key(|(table, id, additional_data)| {
        format!("{}_{}_{}", table, id, additional_data.clone().unwrap_or("".to_string()))
    });

    let mut expected_results2: Vec<(String, Uuid, Option<String>)> = Vec::new();
    expected_results2.push(("Purchase".to_string(), order3.id, None));
    expected_results2.push(("CheckIn".to_string(), ticket6.id, None));
    expected_results2.push(("Note".to_string(), note4.id, None));
    expected_results2.push(("Transfer".to_string(), transfer2.id, Some("Started".to_string())));
    expected_results2.push(("Refund".to_string(), refund3.0.id, None));
    expected_results2.sort_by_key(|(table, id, additional_data)| {
        format!("{}_{}_{}", table, id, additional_data.clone().unwrap_or("".to_string()))
    });

    let mut expected_results4: Vec<(String, Uuid, Option<String>)> = Vec::new();
    expected_results4.push(("Transfer".to_string(), transfer.id, Some("Accepted".to_string())));
    expected_results4.sort_by_key(|(table, id, additional_data)| {
        format!("{}_{}_{}", table, id, additional_data.clone().unwrap_or("".to_string()))
    });

    let mut expected_results6: Vec<(String, Uuid, Option<String>)> = Vec::new();
    expected_results6.push(("Purchase".to_string(), order4.id, None));
    expected_results6.push(("Transfer".to_string(), transfer3.id, Some("Started".to_string())));
    expected_results6.push(("Transfer".to_string(), transfer3.id, Some("Cancelled".to_string())));
    expected_results6.sort_by_key(|(table, id, additional_data)| {
        format!("{}_{}_{}", table, id, additional_data.clone().unwrap_or("".to_string()))
    });

    for (activity_items, expected_results) in vec![
        (activity_items, expected_results),
        (activity_items2, expected_results2),
        (activity_items4, expected_results4),
        (activity_items6, expected_results6),
    ] {
        let mut records_found: Vec<(String, Uuid, Option<String>)> = Vec::new();
        for activity_item in activity_items {
            records_found.push(verify_activity_item_data(activity_item, connection));
        }
        records_found.sort_by_key(|(table, id, additional_data)| {
            format!("{}_{}_{}", table, id, additional_data.clone().unwrap_or("".to_string()))
        });
        assert_eq!(records_found, expected_results);
    }
}

#[test]
fn load_for_order() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let user2 = project.create_user().finish();
    let user3 = project.create_user().finish();
    let organization = project.create_organization().with_event_fee().with_fees().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_ticket_type_count(1)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let ticket_types = &event.ticket_types(true, None, connection).unwrap();
    let hold = project
        .create_hold()
        .with_hold_type(HoldTypes::Discount)
        .with_quantity(10)
        .with_ticket_type_id(ticket_types[0].id)
        .finish();
    let code = project
        .create_code()
        .with_event(&event)
        .with_code_type(CodeTypes::Discount)
        .for_ticket_type(&ticket_types[0])
        .with_discount_in_cents(Some(10))
        .finish();
    let order = project
        .create_order()
        .for_event(&event)
        .on_behalf_of_user(&user)
        .for_user(&user2)
        .quantity(2)
        .with_redemption_code(hold.redemption_code.clone().unwrap())
        .is_paid()
        .finish();
    let order2 = project
        .create_order()
        .for_event(&event)
        .for_user(&user)
        .quantity(3)
        .with_redemption_code(code.redemption_code.clone())
        .is_paid()
        .finish();

    let user_tickets = order.tickets(None, connection).unwrap();
    // Transferred
    let ticket = &user_tickets[0];
    // Redeemed
    let ticket2 = &user_tickets[1];

    let user_tickets = order2.tickets(None, connection).unwrap();
    // Refund
    let ticket3 = &user_tickets[0];

    // Completed transfer
    let transfer = TicketInstance::direct_transfer(
        &user,
        &vec![ticket.id],
        "nowhere",
        TransferMessageType::Email,
        user2.id,
        connection,
    )
    .unwrap();

    TicketInstance::redeem_ticket(
        ticket2.id,
        ticket2.redeem_key.clone().unwrap(),
        user3.id,
        CheckInSource::GuestList,
        connection,
    )
    .unwrap();
    let ticket2 = TicketInstance::find(ticket2.id, connection).unwrap();

    let mut refunding_order = Order::find(
        OrderItem::find(ticket3.order_item_id.unwrap(), connection)
            .unwrap()
            .order_id,
        connection,
    )
    .unwrap();
    let refund_items = vec![RefundItemRequest {
        order_item_id: ticket3.order_item_id.unwrap(),
        ticket_instance_id: Some(ticket3.id),
    }];

    let refund = refunding_order
        .refund(&refund_items, user.id, None, false, connection)
        .unwrap();

    let note = order
        .create_note("Client will pick up at 18h00".to_string(), user3.id, connection)
        .unwrap();
    let note2 = order
        .create_note("Client will pick up at 16h00".to_string(), user2.id, connection)
        .unwrap();
    let note3 = order2
        .create_note("Client will pick up at 11h00".to_string(), user3.id, connection)
        .unwrap();

    let activity_items = ActivityItem::load_for_order(&order, connection).unwrap();
    let activity_items2 = ActivityItem::load_for_order(&order2, connection).unwrap();
    assert_eq!((activity_items.len(), activity_items2.len(),), (6, 3));

    let mut expected_results: Vec<(String, Uuid, Option<String>)> = Vec::new();
    expected_results.push(("CheckIn".to_string(), ticket2.id, None));
    expected_results.push(("Note".to_string(), note2.id, None));
    expected_results.push(("Note".to_string(), note.id, None));
    expected_results.push(("Purchase".to_string(), order.id, None));
    expected_results.push(("Transfer".to_string(), transfer.id, Some("Started".to_string())));
    expected_results.push(("Transfer".to_string(), transfer.id, Some("Accepted".to_string())));
    expected_results.sort_by_key(|(table, id, additional_data)| {
        format!("{}_{}_{}", table, id, additional_data.clone().unwrap_or("".to_string()))
    });

    let mut expected_results2: Vec<(String, Uuid, Option<String>)> = Vec::new();
    expected_results2.push(("Purchase".to_string(), order2.id, None));
    expected_results2.push(("Refund".to_string(), refund.0.id, None));
    expected_results2.push(("Note".to_string(), note3.id, None));
    expected_results2.sort_by_key(|(table, id, additional_data)| {
        format!("{}_{}_{}", table, id, additional_data.clone().unwrap_or("".to_string()))
    });

    for (activity_items, expected_results) in
        vec![(activity_items, expected_results), (activity_items2, expected_results2)]
    {
        let mut records_found: Vec<(String, Uuid, Option<String>)> = Vec::new();
        for activity_item in activity_items {
            records_found.push(verify_activity_item_data(activity_item, connection));
        }
        records_found.sort_by_key(|(table, id, additional_data)| {
            format!("{}_{}_{}", table, id, additional_data.clone().unwrap_or("".to_string()))
        });
        assert_eq!(records_found, expected_results);
    }
}

#[test]
fn occurred_at() {
    let project = TestProject::new();
    let user = project.create_user().finish();
    let now = Utc::now().naive_utc();
    let examples = vec![
        ActivityItem::Purchase {
            order_id: Uuid::new_v4(),
            order_number: "1234".to_string(),
            ticket_quantity: 1,
            events: Vec::new(),
            occurred_at: now.clone(),
            paid_at: Some(now),
            purchased_by: user.clone().into(),
            user: user.clone().into(),
            total_in_cents: 10,
        },
        ActivityItem::Transfer {
            transfer_id: Uuid::new_v4(),
            action: "Completed".to_string(),
            status: TransferStatus::Completed,
            ticket_ids: Vec::new(),
            ticket_numbers: vec![],
            destination_addresses: None,
            transfer_message_type: None,
            initiated_by: user.clone().into(),
            accepted_by: Some(user.clone().into()),
            cancelled_by: Some(user.clone().into()),
            occurred_at: now,
            order_id: None,
            order_number: None,
            transfer_key: Uuid::new_v4(),
            eligible_for_cancelling: true,
        },
        ActivityItem::CheckIn {
            ticket_instance_id: Uuid::new_v4(),
            ticket_number: "".to_string(),
            redeemed_for: user.clone().into(),
            redeemed_by: user.clone().into(),
            occurred_at: now,
            order_id: None,
            order_number: None,
        },
        ActivityItem::Refund {
            refund_id: Uuid::new_v4(),
            order_id: Uuid::new_v4(),
            manual_override: false,
            reason: None,
            refund_items: Vec::new(),
            total_in_cents: 11,
            refunded_by: user.clone().into(),
            occurred_at: now,
            order_number: "".to_string(),
        },
        ActivityItem::Note {
            note_id: Uuid::new_v4(),
            order_id: Uuid::new_v4(),
            order_number: "".to_string(),
            created_by: user.into(),
            note: "note".to_string(),
            occurred_at: now,
        },
    ];
    for example in examples {
        assert_eq!(example.occurred_at().round_subsecs(4), now.round_subsecs(4));
    }
}

fn verify_activity_item_data(activity_item: ActivityItem, connection: &PgConnection) -> (String, Uuid, Option<String>) {
    match activity_item {
        ActivityItem::Purchase {
            order_id,
            order_number,
            ticket_quantity,
            events,
            purchased_by,
            user,
            total_in_cents,
            ..
        } => {
            let found_order = Order::find(order_id, connection).unwrap();
            let order_items = found_order.items(connection).unwrap();
            let found_code = order_items
                .iter()
                .find(|oi| oi.code_id.is_some())
                .map(|oi| Code::find(oi.code_id.unwrap(), connection).unwrap());
            let found_hold = order_items
                .iter()
                .find(|oi| oi.hold_id.is_some())
                .map(|oi| Hold::find(oi.hold_id.unwrap(), connection).unwrap());
            assert_eq!(order_number, Order::order_number(&found_order));
            let mut calculated_quantity = 0;
            let mut calculated_total = 0;
            let mut calculated_code_quantity = 0;
            let mut calculated_code_total = 0;
            let mut calculated_expected_total = 0;
            for item in order_items {
                calculated_expected_total += item.quantity * item.unit_price_in_cents;
                if item.item_type == OrderItemTypes::Tickets {
                    let mut ticket_total = item.quantity * item.unit_price_in_cents;
                    if let Some(fee_item) = item.find_fee_item(connection).unwrap() {
                        ticket_total += fee_item.quantity * fee_item.unit_price_in_cents;
                    }
                    if let Some(discount_item) = item.find_discount_item(connection).unwrap() {
                        ticket_total += discount_item.quantity * discount_item.unit_price_in_cents;
                    }

                    if item.hold_id.is_some() || item.code_id.is_some() {
                        calculated_code_quantity += item.quantity;
                        calculated_code_total += ticket_total;
                    } else {
                        calculated_quantity += item.quantity;
                        calculated_total += ticket_total;
                    }
                } else if item.item_type == OrderItemTypes::EventFees {
                    calculated_total += item.quantity * item.unit_price_in_cents;
                }
            }
            assert_eq!(ticket_quantity, calculated_quantity + calculated_code_quantity);
            assert_eq!(calculated_expected_total, calculated_total + calculated_code_total);
            assert_eq!(calculated_expected_total, total_in_cents);

            for event in found_order.events(connection).unwrap() {
                let found_code_event = events.iter().find(|e| e.event_id == event.id && e.code.is_some());
                if let Some(found_code_event) = found_code_event {
                    assert_eq!(found_code_event.event_id, event.id);
                    assert_eq!(found_code_event.name, event.name);
                    assert_eq!(found_code_event.quantity, calculated_code_quantity);
                    assert_eq!(found_code_event.total_in_cents, calculated_code_total);

                    let mut redemption_code = None;
                    let mut code_discount_in_cents = None;
                    let mut code_type = None;
                    if let Some(hold) = found_hold.clone() {
                        redemption_code = hold.redemption_code.clone();
                        code_discount_in_cents = hold.discount_in_cents;
                        code_type = Some(hold.hold_type.to_string());
                    } else if let Some(code) = found_code.clone() {
                        redemption_code = Some(code.redemption_code.clone());
                        code_discount_in_cents = code.discount_in_cents;
                        code_type = Some(code.code_type.to_string());
                    }
                    assert_eq!(found_code_event.code, redemption_code);
                    assert_eq!(found_code_event.code_discount_in_cents, code_discount_in_cents);
                    assert_eq!(found_code_event.code_type, code_type);
                }

                let found_not_code_event = events.iter().find(|e| e.event_id == event.id && e.code.is_none());

                if let Some(found_not_code_event) = found_not_code_event {
                    assert_eq!(found_not_code_event.event_id, event.id);
                    assert_eq!(found_not_code_event.name, event.name);
                    assert_eq!(found_not_code_event.quantity, calculated_quantity);
                    assert_eq!(found_not_code_event.total_in_cents, calculated_total);
                    assert!(found_not_code_event.code.is_none());
                }
            }

            let expected_user: UserActivityItem = User::find(
                found_order.on_behalf_of_user_id.unwrap_or(found_order.user_id),
                connection,
            )
            .unwrap()
            .into();
            assert_eq!(expected_user, user);
            let expected_purchased_by: UserActivityItem = User::find(found_order.user_id, connection).unwrap().into();
            assert_eq!(expected_purchased_by, purchased_by);
            ("Purchase".to_string(), order_id, None)
        }
        ActivityItem::Transfer {
            transfer_id,
            action,
            status,
            ticket_ids,
            destination_addresses,
            transfer_message_type,
            initiated_by,
            accepted_by,
            cancelled_by,
            ..
        } => {
            let found_transfer = Transfer::find(transfer_id, connection).unwrap();
            assert_eq!(status, found_transfer.status);
            assert_eq!(destination_addresses, found_transfer.transfer_address);
            assert_eq!(transfer_message_type, found_transfer.transfer_message_type);
            assert_eq!(
                ticket_ids,
                found_transfer
                    .transfer_tickets(connection)
                    .unwrap()
                    .iter()
                    .map(|tt| tt.ticket_instance_id)
                    .collect::<Vec<Uuid>>()
            );
            let expected_initated_by: UserActivityItem =
                User::find(found_transfer.source_user_id, connection).unwrap().into();
            assert_eq!(expected_initated_by, initiated_by);

            let expected_accepted_by: Option<UserActivityItem> = found_transfer
                .destination_user_id
                .map(|destination_user_id| User::find(destination_user_id, connection).unwrap().into());
            assert_eq!(expected_accepted_by, accepted_by);

            let expected_cancelled_by: Option<UserActivityItem> = found_transfer
                .cancelled_by_user_id
                .map(|cancelled_by_user_id| User::find(cancelled_by_user_id, connection).unwrap().into());
            assert_eq!(expected_cancelled_by, cancelled_by);

            ("Transfer".to_string(), transfer_id, Some(action))
        }
        ActivityItem::CheckIn {
            ticket_instance_id,
            redeemed_for,
            redeemed_by,
            ..
        } => {
            let found_ticket = TicketInstance::find(ticket_instance_id, connection).unwrap();
            let expected_redeemed_by: UserActivityItem =
                User::find(found_ticket.redeemed_by_user_id.unwrap(), connection)
                    .unwrap()
                    .into();
            assert_eq!(expected_redeemed_by, redeemed_by);
            let expected_redeemed_for: UserActivityItem = found_ticket.owner(connection).unwrap().into();
            assert_eq!(expected_redeemed_for, redeemed_for);
            ("CheckIn".to_string(), ticket_instance_id, None)
        }
        ActivityItem::Refund {
            refund_id,
            reason,
            total_in_cents,
            refunded_by,
            manual_override,
            ..
        } => {
            let found_refund = Refund::find(refund_id, connection).unwrap();
            let refund_amount: i64 = found_refund.items(connection).unwrap().iter().map(|r| r.amount).sum();
            assert_eq!(reason, found_refund.reason);
            assert_eq!(manual_override, found_refund.manual_override);
            assert_eq!(total_in_cents, refund_amount);

            let expected_refunded_by: UserActivityItem = User::find(found_refund.user_id, connection).unwrap().into();
            assert_eq!(expected_refunded_by, refunded_by);
            ("Refund".to_string(), refund_id, None)
        }
        ActivityItem::Note {
            note_id,
            order_id,
            note,
            created_by,
            ..
        } => {
            let found_note = Note::find(note_id, connection).unwrap();
            assert_eq!(found_note.main_id, order_id);
            assert_eq!(found_note.note, note);
            let expected_created_by: UserActivityItem = User::find(found_note.created_by, connection).unwrap().into();
            assert_eq!(expected_created_by, created_by);
            ("Note".to_string(), note_id, None)
        }
    }
}
