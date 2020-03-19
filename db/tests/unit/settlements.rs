use chrono::prelude::*;
use chrono::{Duration, Timelike};
use chrono_tz::Tz;
use db::dev::TestProject;
use db::models::*;
use db::schema::{orders, refunds};
use db::utils::dates;
use db::utils::errors::ErrorCode::ValidationError;
use diesel;
use diesel::prelude::*;
use diesel::sql_types;
use diesel::RunQueryDsl;

#[test]
fn finalize_settlements() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let settlement = project.create_settlement().finish();
    let settlement2 = project.create_settlement().finish();
    Settlement::finalize_settlements(connection).unwrap();

    let settlement = Settlement::find(settlement.id, connection).unwrap();
    let settlement2 = Settlement::find(settlement2.id, connection).unwrap();
    assert_eq!(SettlementStatus::FinalizedSettlement, settlement.status);
    assert_eq!(SettlementStatus::FinalizedSettlement, settlement2.status);
}

#[test]
fn create_next_finalize_settlements_domain_action() {
    let project = TestProject::new();
    let connection = project.get_connection();
    assert!(
        DomainAction::upcoming_domain_action(None, None, DomainActionTypes::FinalizeSettlements, connection,)
            .unwrap()
            .is_none()
    );

    Settlement::create_next_finalize_settlements_domain_action(connection).unwrap();
    let domain_action =
        DomainAction::upcoming_domain_action(None, None, DomainActionTypes::FinalizeSettlements, connection)
            .unwrap()
            .unwrap();
    assert_eq!(
        domain_action.scheduled_at,
        Settlement::next_finalization_date().unwrap()
    );
    assert_eq!(domain_action.status, DomainActionStatus::Pending);
}

#[test]
fn next_finalization_date() {
    let pt_timezone: Tz = "America/Los_Angeles".parse().unwrap();
    let now = pt_timezone.from_utc_datetime(&Utc::now().naive_utc());
    let pt_today = pt_timezone.ymd(now.year(), now.month(), now.day()).and_hms(0, 0, 0);
    let days_since_monday = pt_today.naive_local().weekday().num_days_from_monday();

    let this_wednesday = now + Duration::days(2 - pt_today.naive_local().weekday().num_days_from_monday() as i64);
    let next_wednesday = now + Duration::days(7 - pt_today.naive_local().weekday().num_days_from_monday() as i64 + 2);

    let expected_pt = if days_since_monday < 2 || (days_since_monday == 2 && now.naive_local().hour() < 12) {
        pt_timezone
            .ymd(this_wednesday.year(), this_wednesday.month(), this_wednesday.day())
            .and_hms(12, 0, 0)
            .naive_utc()
    } else {
        pt_timezone
            .ymd(next_wednesday.year(), next_wednesday.month(), next_wednesday.day())
            .and_hms(12, 0, 0)
            .naive_utc()
    };

    assert_eq!(Settlement::next_finalization_date().unwrap(), expected_pt);
}

#[test]
fn find_last_settlement_for_organization() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().finish();
    let organization2 = project.create_organization().finish();
    assert!(
        Settlement::find_last_settlement_for_organization(&organization, connection)
            .unwrap()
            .is_none()
    );
    assert!(
        Settlement::find_last_settlement_for_organization(&organization2, connection)
            .unwrap()
            .is_none()
    );

    let settlement = project.create_settlement().with_organization(&organization).finish();
    assert_eq!(
        Settlement::find_last_settlement_for_organization(&organization, connection).unwrap(),
        Some(settlement)
    );
    assert!(
        Settlement::find_last_settlement_for_organization(&organization2, connection)
            .unwrap()
            .is_none()
    );
}

#[test]
fn process_settlement_for_organization() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().finish();

    let event = project
        .create_event()
        .with_ticket_pricing()
        .with_ticket_type_count(1)
        .with_organization(&organization)
        .finish();
    project.create_order().for_event(&event).is_paid().finish();

    let domain_events = DomainEvent::find(
        Tables::Organizations,
        Some(organization.id),
        Some(DomainEventTypes::SettlementReportProcessed),
        connection,
    )
    .unwrap();
    assert_eq!(0, domain_events.len());

    let settlement = Settlement::process_settlement_for_organization(&organization, None, connection).unwrap();
    let domain_events = DomainEvent::find(
        Tables::Organizations,
        Some(organization.id),
        Some(DomainEventTypes::SettlementReportProcessed),
        connection,
    )
    .unwrap();
    assert_eq!(1, domain_events.len());
    assert_eq!(settlement.organization_id, organization.id);

    let end_time = organization.next_settlement_date(None).unwrap() - Duration::days(7) - Duration::seconds(1);
    assert_eq!(
        settlement.start_time,
        end_time - Duration::days(7) + Duration::seconds(1)
    );
    assert_eq!(settlement.end_time, end_time);

    // Set old end time which affects start time of new settlement (to handle timezone updates)
    let old_start_time = dates::now().add_days(-14).add_hours(-3).finish();
    let old_end_time = dates::now().add_days(-7).add_hours(-3).finish();
    diesel::sql_query(
        r#"
        UPDATE settlements
        SET start_time = $1,
        end_time = $2
        WHERE id = $3;
        "#,
    )
    .bind::<sql_types::Timestamp, _>(old_start_time)
    .bind::<sql_types::Timestamp, _>(old_end_time)
    .bind::<sql_types::Uuid, _>(settlement.id)
    .execute(connection)
    .unwrap();

    let settlement = Settlement::process_settlement_for_organization(&organization, None, connection).unwrap();
    assert_eq!(
        settlement.start_time.timestamp(),
        (old_end_time + Duration::seconds(1)).timestamp()
    );
    let end_time = organization.next_settlement_date(None).unwrap() - Duration::days(7) - Duration::seconds(1);
    assert_eq!(settlement.end_time.timestamp(), end_time.timestamp());
}

#[test]
fn create_post_event_entries() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_event_fee()
        .with_cc_fee(5f32)
        .with_settlement_type(SettlementTypes::PostEvent)
        .finish();
    let organization2 = project
        .create_organization()
        .with_event_fee()
        .with_settlement_type(SettlementTypes::PostEvent)
        .finish();
    let past_event = project
        .create_event()
        .with_organization(&organization)
        .with_ticket_pricing()
        .with_event_start(dates::now().add_days(-15).finish())
        .with_event_end(dates::now().add_days(-6).finish())
        .finish();
    let ticket_type = &past_event.ticket_types(true, None, connection).unwrap()[0];
    let past_event_2 = project
        .create_event()
        .with_organization(&organization2)
        .with_ticket_pricing()
        .with_event_start(dates::now().add_days(-15).finish())
        .with_event_end(dates::now().add_days(-6).finish())
        .finish();
    let ending_future_event = project
        .create_event()
        .with_organization(&organization)
        .with_ticket_pricing()
        .with_event_start(dates::now().add_days(-15).finish())
        .with_event_end(dates::now().add_days(1).finish())
        .finish();

    let hold = project
        .create_hold()
        .with_hold_type(HoldTypes::Discount)
        .with_quantity(10)
        .with_discount_in_cents(10)
        .with_ticket_type_id(ticket_type.id)
        .finish();
    let hold2 = project
        .create_hold()
        .with_hold_type(HoldTypes::Discount)
        .with_quantity(10)
        .with_discount_in_cents(20)
        .with_ticket_type_id(ticket_type.id)
        .finish();
    let hold3 = project
        .create_hold()
        .with_hold_type(HoldTypes::Comp)
        .with_quantity(10)
        .with_ticket_type_id(ticket_type.id)
        .finish();
    let code = project
        .create_code()
        .with_event(&past_event)
        .with_code_type(CodeTypes::Discount)
        .for_ticket_type(&ticket_type)
        .with_discount_in_cents(Some(10))
        .finish();
    project
        .create_order()
        .for_event(&past_event)
        .quantity(3)
        .is_paid()
        .finish();
    project.create_order().for_event(&past_event_2).is_paid().finish();
    project
        .create_order()
        .for_event(&ending_future_event)
        .is_paid()
        .finish();

    // Hold order, 10 discount
    project
        .create_order()
        .for_event(&past_event)
        .quantity(2)
        .with_redemption_code(hold.redemption_code.clone().unwrap())
        .is_paid()
        .finish();

    // Hold order, 20 discount
    project
        .create_order()
        .for_event(&past_event)
        .quantity(2)
        .with_redemption_code(hold2.redemption_code.clone().unwrap())
        .is_paid()
        .finish();

    // Hold order, comp
    project
        .create_order()
        .for_event(&past_event)
        .quantity(2)
        .with_redemption_code(hold3.redemption_code.clone().unwrap())
        .is_paid()
        .finish();

    // Code order, 10 discount
    project
        .create_order()
        .for_event(&past_event)
        .quantity(3)
        .with_redemption_code(code.redemption_code.clone())
        .is_paid()
        .finish();

    // Transaction from before settlement period
    let mut order = project
        .create_order()
        .for_event(&past_event)
        .quantity(5)
        .is_paid()
        .finish();
    diesel::update(orders::table.filter(orders::id.eq(order.id)))
        .set((orders::paid_at.eq(dates::now().add_days(-8).finish()),))
        .execute(connection)
        .unwrap();

    // Refund one ticket from order
    let items = order.items(&connection).unwrap();
    let order_item = items.iter().find(|i| i.ticket_type_id == Some(ticket_type.id)).unwrap();
    let tickets = TicketInstance::find_for_order_item(order_item.id, connection).unwrap();
    let ticket = &tickets[0];
    let ticket2 = &tickets[1];
    let refund_items = vec![RefundItemRequest {
        order_item_id: order_item.id,
        ticket_instance_id: Some(ticket.id),
    }];
    order.refund(&refund_items, user.id, None, false, connection).unwrap();
    assert!(past_event.settled_at.is_none());
    assert!(past_event_2.settled_at.is_none());

    let settlement = Settlement::create(
        organization.id,
        dates::now().add_days(-14).finish(),
        dates::now().add_days(-7).finish(),
        SettlementStatus::PendingSettlement,
        None,
        true,
    )
    .commit(None, connection)
    .unwrap();
    let past_event = Event::find(past_event.id, connection).unwrap();
    assert!(past_event.settled_at.is_none());
    let past_event_2 = Event::find(past_event_2.id, connection).unwrap();
    assert!(past_event_2.settled_at.is_none());

    let display_settlement = settlement.clone().for_display(connection).unwrap();
    assert!(display_settlement.event_entries.is_empty());

    let settlement = Settlement::create(
        organization.id,
        dates::now().add_days(-7).finish(),
        dates::now().finish(),
        SettlementStatus::PendingSettlement,
        None,
        true,
    )
    .commit(None, connection)
    .unwrap();
    let past_event = Event::find(past_event.id, connection).unwrap();
    assert!(past_event.settled_at.is_some());
    let past_event_2 = Event::find(past_event_2.id, connection).unwrap();
    assert!(past_event_2.settled_at.is_none());

    let display_settlement = settlement.clone().for_display(connection).unwrap();
    assert_eq!(display_settlement.event_entries.len(), 1);
    assert_eq!(
        display_settlement.event_entries[0].event,
        past_event.for_display(connection).unwrap()
    );
    let event_entries = &display_settlement.event_entries[0].entries;
    assert_eq!(event_entries.len(), 4);

    // No code entry
    let ticket_type_entry = event_entries
        .clone()
        .into_iter()
        .find(|e| e.settlement_entry_type == SettlementEntryTypes::TicketType && e.face_value_in_cents == 150)
        .unwrap();
    let ticket_type = &past_event.ticket_types(true, None, connection).unwrap()[0];
    assert_eq!(ticket_type_entry.settlement_id, settlement.id);
    assert_eq!(ticket_type_entry.event_id, past_event.id);
    assert_eq!(ticket_type_entry.ticket_type_id, Some(ticket_type.id));
    assert_eq!(ticket_type_entry.face_value_in_cents, 150);
    assert_eq!(ticket_type_entry.revenue_share_value_in_cents, 30);
    assert_eq!(ticket_type_entry.online_sold_quantity, 7);
    assert_eq!(ticket_type_entry.fee_sold_quantity, 7);
    assert_eq!(
        ticket_type_entry.total_sales_in_cents,
        ticket_type_entry.online_sold_quantity * ticket_type_entry.face_value_in_cents
            + ticket_type_entry.fee_sold_quantity * ticket_type_entry.revenue_share_value_in_cents
    );

    // Code and redemption code orders have same price due to identical discount so they are grouped
    let ticket_type_entry = event_entries
        .clone()
        .into_iter()
        .find(|e| e.settlement_entry_type == SettlementEntryTypes::TicketType && e.face_value_in_cents == 140)
        .unwrap();
    let ticket_type = &past_event.ticket_types(true, None, connection).unwrap()[0];
    assert_eq!(ticket_type_entry.settlement_id, settlement.id);
    assert_eq!(ticket_type_entry.event_id, past_event.id);
    assert_eq!(ticket_type_entry.ticket_type_id, Some(ticket_type.id));
    assert_eq!(ticket_type_entry.face_value_in_cents, 140);
    assert_eq!(ticket_type_entry.revenue_share_value_in_cents, 30);
    assert_eq!(ticket_type_entry.online_sold_quantity, 5);
    assert_eq!(ticket_type_entry.fee_sold_quantity, 5);
    assert_eq!(
        ticket_type_entry.total_sales_in_cents,
        ticket_type_entry.online_sold_quantity * ticket_type_entry.face_value_in_cents
            + ticket_type_entry.fee_sold_quantity * ticket_type_entry.revenue_share_value_in_cents
    );

    // Redemption code with different discount
    let ticket_type_entry = event_entries
        .clone()
        .into_iter()
        .find(|e| e.settlement_entry_type == SettlementEntryTypes::TicketType && e.face_value_in_cents == 130)
        .unwrap();
    let ticket_type = &past_event.ticket_types(true, None, connection).unwrap()[0];
    assert_eq!(ticket_type_entry.settlement_id, settlement.id);
    assert_eq!(ticket_type_entry.event_id, past_event.id);
    assert_eq!(ticket_type_entry.ticket_type_id, Some(ticket_type.id));
    assert_eq!(ticket_type_entry.face_value_in_cents, 130);
    assert_eq!(ticket_type_entry.revenue_share_value_in_cents, 30);
    assert_eq!(ticket_type_entry.online_sold_quantity, 2);
    assert_eq!(ticket_type_entry.fee_sold_quantity, 2);
    assert_eq!(
        ticket_type_entry.total_sales_in_cents,
        ticket_type_entry.online_sold_quantity * ticket_type_entry.face_value_in_cents
            + ticket_type_entry.fee_sold_quantity * ticket_type_entry.revenue_share_value_in_cents
    );

    let event_fee_entry = event_entries
        .into_iter()
        .find(|e| e.settlement_entry_type == SettlementEntryTypes::EventFees)
        .unwrap();
    assert_eq!(event_fee_entry.settlement_id, settlement.id);
    assert_eq!(event_fee_entry.event_id, past_event.id);
    assert_eq!(event_fee_entry.ticket_type_id, None);
    assert_eq!(event_fee_entry.face_value_in_cents, 0);
    assert_eq!(event_fee_entry.revenue_share_value_in_cents, 150);
    assert_eq!(event_fee_entry.online_sold_quantity, 0);
    assert_eq!(event_fee_entry.fee_sold_quantity, 5);
    assert_eq!(event_fee_entry.total_sales_in_cents, 750);

    // Refund comes in after post event has been settled, included in next settlement report
    // even though event has already settled
    let refund_items = vec![RefundItemRequest {
        order_item_id: order_item.id,
        ticket_instance_id: Some(ticket2.id),
    }];
    order.refund(&refund_items, user.id, None, false, connection).unwrap();

    let settlement = Settlement::create(
        organization.id,
        dates::now().add_days(-7).finish(),
        dates::now().add_minutes(1).finish(),
        SettlementStatus::PendingSettlement,
        None,
        true,
    )
    .commit(None, connection)
    .unwrap();
    let past_event = Event::find(past_event.id, connection).unwrap();
    assert!(past_event.settled_at.is_some());
    let past_event_2 = Event::find(past_event_2.id, connection).unwrap();
    assert!(past_event_2.settled_at.is_none());

    let display_settlement = settlement.clone().for_display(connection).unwrap();
    assert_eq!(display_settlement.event_entries.len(), 1);
    assert_eq!(
        display_settlement.event_entries[0].event,
        past_event.for_display(connection).unwrap()
    );
    let event_entries = &display_settlement.event_entries[0].entries;
    assert_eq!(event_entries.len(), 1);

    let ticket_type_entry = event_entries
        .clone()
        .into_iter()
        .find(|e| e.settlement_entry_type == SettlementEntryTypes::TicketType && e.face_value_in_cents == 150)
        .unwrap();
    let ticket_type = &past_event.ticket_types(true, None, connection).unwrap()[0];
    assert_eq!(ticket_type_entry.settlement_id, settlement.id);
    assert_eq!(ticket_type_entry.event_id, past_event.id);
    assert_eq!(ticket_type_entry.ticket_type_id, Some(ticket_type.id));
    assert_eq!(ticket_type_entry.face_value_in_cents, 150);
    assert_eq!(ticket_type_entry.revenue_share_value_in_cents, 30);
    assert_eq!(ticket_type_entry.online_sold_quantity, -1);
    assert_eq!(ticket_type_entry.fee_sold_quantity, -1);
    assert_eq!(
        ticket_type_entry.total_sales_in_cents,
        ticket_type_entry.online_sold_quantity * ticket_type_entry.face_value_in_cents
            + ticket_type_entry.fee_sold_quantity * ticket_type_entry.revenue_share_value_in_cents
    );
}

#[test]
fn settlement_free_ticket_with_ticket_fee_behavior() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_fees()
        .with_max_additional_fee(1000)
        .finish();
    let ticket_type = project
        .create_event()
        .with_organization(&organization)
        .with_ticket_type()
        .with_price(10)
        .with_additional_fees(1000)
        .finish()
        .ticket_types(false, None, connection)
        .unwrap()
        .remove(0);
    let hold = project
        .create_hold()
        .with_hold_type(HoldTypes::Discount)
        .with_quantity(10)
        .with_discount_in_cents(10)
        .with_ticket_type_id(ticket_type.id)
        .finish();
    let event = Event::find(ticket_type.event_id, connection).unwrap();
    let mut order = project
        .create_order()
        .for_event(&event)
        .with_redemption_code(hold.redemption_code.clone().unwrap())
        .is_paid()
        .finish();
    let items = order.items(&connection).unwrap();
    let order_item = items.iter().find(|i| i.ticket_type_id == Some(ticket_type.id)).unwrap();
    let tickets = TicketInstance::find_for_order_item(order_item.id, connection).unwrap();
    let ticket = &tickets[0];
    let refund_items = vec![RefundItemRequest {
        order_item_id: order_item.id,
        ticket_instance_id: Some(ticket.id),
    }];
    // refund one ticket bringing total of fees down to 9
    order.refund(&refund_items, user.id, None, false, connection).unwrap();

    let settlement = Settlement::create(
        organization.id,
        dates::now().add_days(-5).finish(),
        dates::now().add_days(2).finish(),
        SettlementStatus::PendingSettlement,
        None,
        false,
    )
    .commit(None, connection)
    .unwrap();

    let display_settlement = settlement.clone().for_display(connection).unwrap();
    assert_eq!(display_settlement.event_entries.len(), 1);
    let event_entries_data = &display_settlement.event_entries[0];
    let event_entries = &event_entries_data.entries;
    assert_eq!(event_entries.len(), 1);

    let ticket_type_entry = event_entries
        .clone()
        .into_iter()
        .find(|e| {
            e.settlement_entry_type == SettlementEntryTypes::TicketType && e.ticket_type_id == Some(ticket_type.id)
        })
        .unwrap();
    assert_eq!(ticket_type_entry.settlement_id, settlement.id);
    assert_eq!(ticket_type_entry.event_id, event.id);
    assert_eq!(ticket_type_entry.ticket_type_id, Some(ticket_type.id));
    assert_eq!(ticket_type_entry.face_value_in_cents, 0);
    assert_eq!(ticket_type_entry.revenue_share_value_in_cents, 1000);
    assert_eq!(ticket_type_entry.online_sold_quantity, 9);
    assert_eq!(ticket_type_entry.fee_sold_quantity, 9);
    assert_eq!(
        ticket_type_entry.total_sales_in_cents,
        ticket_type_entry.fee_sold_quantity * ticket_type_entry.revenue_share_value_in_cents
    );
}

#[test]
// Note this logic is only needed for a transitional period of time
// Currently in production we're manually peforming settlement reports so order exist that have been
// included in a settlement but not marked as such. The logic acts as though the orders prior to the
// first rolling settlement are ignored as they are assumed to be already settled. If the organization
// switches to post event settlements the logic sees there was a rolling previously and uses that date.
fn rolling_to_post_event_settlement_hack_behavior() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_event_fee()
        .with_cc_fee(5f32)
        .finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_ticket_pricing()
        .with_event_start(dates::now().add_days(-14).finish())
        .with_event_end(dates::now().add_days(-1).finish())
        .finish();
    assert!(event.settled_at.is_none());
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];

    // First order is from before the settlement period
    let ancient_order = project.create_order().for_event(&event).is_paid().finish();
    diesel::update(orders::table.filter(orders::id.eq(ancient_order.id)))
        .set((orders::paid_at.eq(dates::now().add_days(-14).finish()),))
        .execute(connection)
        .unwrap();

    // Second order is included in the first settlement period
    let first_settlement_order = project.create_order().for_event(&event).is_paid().finish();
    diesel::update(orders::table.filter(orders::id.eq(first_settlement_order.id)))
        .set((orders::paid_at.eq(dates::now().add_days(-7).finish()),))
        .execute(connection)
        .unwrap();

    // Third order is after the first settlement period
    let mut second_settlement_order = project.create_order().for_event(&event).is_paid().finish();
    diesel::update(orders::table.filter(orders::id.eq(second_settlement_order.id)))
        .set((orders::paid_at.eq(dates::now().add_days(-2).finish()),))
        .execute(connection)
        .unwrap();

    let items = second_settlement_order.items(&connection).unwrap();
    let order_item = items.iter().find(|i| i.ticket_type_id == Some(ticket_type.id)).unwrap();
    let tickets = TicketInstance::find_for_order_item(order_item.id, connection).unwrap();
    let ticket = &tickets[0];
    let refund_items = vec![RefundItemRequest {
        order_item_id: order_item.id,
        ticket_instance_id: Some(ticket.id),
    }];
    let (refund, _) = second_settlement_order
        .refund(&refund_items, user.id, None, false, connection)
        .unwrap();
    diesel::update(refunds::table.filter(refunds::id.eq(refund.id)))
        .set((refunds::created_at.eq(dates::now().add_days(-2).finish()),))
        .execute(connection)
        .unwrap();

    // First settlement run as rolling
    let settlement = Settlement::create(
        organization.id,
        dates::now().add_days(-10).finish(),
        dates::now().add_days(-3).finish(),
        SettlementStatus::PendingSettlement,
        None,
        false,
    )
    .commit(None, connection)
    .unwrap();
    let event = Event::find(event.id, connection).unwrap();
    assert!(event.settled_at.is_none());
    diesel::sql_query(
        r#"
        UPDATE settlements
        SET created_at = $1
        WHERE id = $2;
        "#,
    )
    .bind::<sql_types::Timestamp, _>(dates::now().add_days(-3).finish())
    .bind::<sql_types::Uuid, _>(settlement.id)
    .execute(connection)
    .unwrap();

    let display_settlement = settlement.clone().for_display(connection).unwrap();
    assert_eq!(display_settlement.event_entries.len(), 1);
    let event_entries_data = &display_settlement.event_entries[0];
    let event_entries = &event_entries_data.entries;
    assert_eq!(event_entries.len(), 2);

    let ticket_type_entry = event_entries
        .clone()
        .into_iter()
        .find(|e| {
            e.settlement_entry_type == SettlementEntryTypes::TicketType && e.ticket_type_id == Some(ticket_type.id)
        })
        .unwrap();
    assert_eq!(ticket_type_entry.settlement_id, settlement.id);
    assert_eq!(ticket_type_entry.event_id, event.id);
    assert_eq!(ticket_type_entry.ticket_type_id, Some(ticket_type.id));
    assert_eq!(ticket_type_entry.face_value_in_cents, 150);
    assert_eq!(ticket_type_entry.revenue_share_value_in_cents, 30);
    assert_eq!(ticket_type_entry.online_sold_quantity, 10);
    assert_eq!(ticket_type_entry.fee_sold_quantity, 10);
    assert_eq!(
        ticket_type_entry.total_sales_in_cents,
        ticket_type_entry.online_sold_quantity * ticket_type_entry.face_value_in_cents
            + ticket_type_entry.fee_sold_quantity * ticket_type_entry.revenue_share_value_in_cents
    );
    let event_fee_entry = event_entries
        .clone()
        .into_iter()
        .find(|e| e.settlement_entry_type == SettlementEntryTypes::EventFees)
        .unwrap();
    assert_eq!(event_fee_entry.settlement_id, settlement.id);
    assert_eq!(event_fee_entry.event_id, event.id);
    assert_eq!(event_fee_entry.ticket_type_id, None);
    assert_eq!(event_fee_entry.face_value_in_cents, 0);
    assert_eq!(event_fee_entry.revenue_share_value_in_cents, 150);
    assert_eq!(event_fee_entry.online_sold_quantity, 0);
    assert_eq!(event_fee_entry.fee_sold_quantity, 1);
    assert_eq!(event_fee_entry.total_sales_in_cents, 150);

    // Then a post event settlement is run and only includes the third order and its refund
    let settlement2 = Settlement::create(
        organization.id,
        dates::now().add_days(-3).finish(),
        dates::now().finish(),
        SettlementStatus::PendingSettlement,
        None,
        true,
    )
    .commit(None, connection)
    .unwrap();
    let event = Event::find(event.id, connection).unwrap();
    assert!(event.settled_at.is_some());

    let display_settlement = settlement2.clone().for_display(connection).unwrap();
    assert_eq!(display_settlement.event_entries.len(), 1);
    let event_entries_data = &display_settlement.event_entries[0];
    let event_entries = &event_entries_data.entries;
    assert_eq!(event_entries.len(), 2);

    let ticket_type_entry = event_entries
        .clone()
        .into_iter()
        .find(|e| {
            e.settlement_entry_type == SettlementEntryTypes::TicketType && e.ticket_type_id == Some(ticket_type.id)
        })
        .unwrap();
    assert_eq!(ticket_type_entry.settlement_id, settlement2.id);
    assert_eq!(ticket_type_entry.event_id, event.id);
    assert_eq!(ticket_type_entry.ticket_type_id, Some(ticket_type.id));
    assert_eq!(ticket_type_entry.face_value_in_cents, 150);
    assert_eq!(ticket_type_entry.revenue_share_value_in_cents, 30);
    assert_eq!(ticket_type_entry.online_sold_quantity, 9);
    assert_eq!(ticket_type_entry.fee_sold_quantity, 9);
    assert_eq!(
        ticket_type_entry.total_sales_in_cents,
        ticket_type_entry.online_sold_quantity * ticket_type_entry.face_value_in_cents
            + ticket_type_entry.fee_sold_quantity * ticket_type_entry.revenue_share_value_in_cents
    );
    let event_fee_entry = event_entries
        .clone()
        .into_iter()
        .find(|e| e.settlement_entry_type == SettlementEntryTypes::EventFees)
        .unwrap();
    assert_eq!(event_fee_entry.settlement_id, settlement2.id);
    assert_eq!(event_fee_entry.event_id, event.id);
    assert_eq!(event_fee_entry.ticket_type_id, None);
    assert_eq!(event_fee_entry.face_value_in_cents, 0);
    assert_eq!(event_fee_entry.revenue_share_value_in_cents, 150);
    assert_eq!(event_fee_entry.online_sold_quantity, 0);
    assert_eq!(event_fee_entry.fee_sold_quantity, 1);
    assert_eq!(event_fee_entry.total_sales_in_cents, 150);

    // Reload orders and refunds to confirm correct settlement ids set
    let ancient_order = Order::find(ancient_order.id, connection).unwrap();
    assert_eq!(ancient_order.settlement_id, None);
    let first_settlement_order = Order::find(first_settlement_order.id, connection).unwrap();
    assert_eq!(first_settlement_order.settlement_id, Some(settlement.id));
    let second_settlement_order = Order::find(second_settlement_order.id, connection).unwrap();
    assert_eq!(second_settlement_order.settlement_id, Some(settlement2.id));
    let refund = Refund::find(refund.id, connection).unwrap();
    assert_eq!(refund.settlement_id, Some(settlement2.id));
}

#[test]
fn create_rolling_entries() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_event_fee()
        .with_cc_fee(5f32)
        .with_settlement_type(SettlementTypes::Rolling)
        .finish();
    let organization2 = project
        .create_organization()
        .with_event_fee()
        .with_settlement_type(SettlementTypes::Rolling)
        .finish();
    let past_event = project
        .create_event()
        .with_organization(&organization)
        .with_ticket_pricing()
        .with_event_start(dates::now().add_days(-15).finish())
        .with_event_end(dates::now().add_days(-14).finish())
        .finish();
    let ticket_type = &past_event.ticket_types(true, None, connection).unwrap()[0];
    let past_event_2 = project
        .create_event()
        .with_organization(&organization2)
        .with_ticket_pricing()
        .with_event_start(dates::now().add_days(-15).finish())
        .with_event_end(dates::now().add_days(-14).finish())
        .finish();
    let ending_future_event = project
        .create_event()
        .with_organization(&organization)
        .with_ticket_pricing()
        .with_event_start(dates::now().add_days(-15).finish())
        .with_event_end(dates::now().add_days(1).finish())
        .finish();
    let ticket_type2 = &ending_future_event.ticket_types(true, None, connection).unwrap()[0];

    let hold = project
        .create_hold()
        .with_hold_type(HoldTypes::Discount)
        .with_quantity(10)
        .with_discount_in_cents(10)
        .with_ticket_type_id(ticket_type.id)
        .finish();
    let hold2 = project
        .create_hold()
        .with_hold_type(HoldTypes::Discount)
        .with_quantity(10)
        .with_discount_in_cents(20)
        .with_ticket_type_id(ticket_type.id)
        .finish();
    let hold3 = project
        .create_hold()
        .with_hold_type(HoldTypes::Comp)
        .with_quantity(10)
        .with_ticket_type_id(ticket_type.id)
        .finish();
    let code = project
        .create_code()
        .with_event(&past_event)
        .with_code_type(CodeTypes::Discount)
        .for_ticket_type(&ticket_type)
        .with_discount_in_cents(Some(10))
        .finish();

    // Transaction from before settlement period
    let order = project.create_order().for_event(&past_event).is_paid().finish();
    diesel::update(orders::table.filter(orders::id.eq(order.id)))
        .set((orders::paid_at.eq(dates::now().add_days(-8).finish()),))
        .execute(connection)
        .unwrap();

    // Transaction from after settlement period
    let order = project.create_order().for_event(&past_event).is_paid().finish();
    diesel::update(orders::table.filter(orders::id.eq(order.id)))
        .set((orders::paid_at.eq(dates::now().add_minutes(1).finish()),))
        .execute(connection)
        .unwrap();

    let mut order = project
        .create_order()
        .quantity(4)
        .for_event(&past_event)
        .is_paid()
        .finish();
    // Refund one ticket from order
    let items = order.items(&connection).unwrap();
    let order_item = items.iter().find(|i| i.ticket_type_id == Some(ticket_type.id)).unwrap();
    let tickets = TicketInstance::find_for_order_item(order_item.id, connection).unwrap();
    let ticket = &tickets[0];
    let refund_items = vec![RefundItemRequest {
        order_item_id: order_item.id,
        ticket_instance_id: Some(ticket.id),
    }];
    order.refund(&refund_items, user.id, None, false, connection).unwrap();

    project.create_order().for_event(&past_event_2).is_paid().finish();
    project
        .create_order()
        .quantity(5)
        .for_event(&ending_future_event)
        .is_paid()
        .finish();

    // Hold order, 10 discount
    project
        .create_order()
        .for_event(&past_event)
        .quantity(2)
        .with_redemption_code(hold.redemption_code.clone().unwrap())
        .is_paid()
        .finish();

    // Hold order, 20 discount
    project
        .create_order()
        .for_event(&past_event)
        .quantity(2)
        .with_redemption_code(hold2.redemption_code.clone().unwrap())
        .is_paid()
        .finish();

    // Hold order, comp
    project
        .create_order()
        .for_event(&past_event)
        .quantity(2)
        .with_redemption_code(hold3.redemption_code.clone().unwrap())
        .is_paid()
        .finish();

    // Code order, 10 discount
    project
        .create_order()
        .for_event(&past_event)
        .quantity(3)
        .with_redemption_code(code.redemption_code.clone())
        .is_paid()
        .finish();

    let settlement = Settlement::create(
        organization.id,
        dates::now().add_days(-7).finish(),
        dates::now().finish(),
        SettlementStatus::PendingSettlement,
        None,
        false,
    )
    .commit(None, connection)
    .unwrap();
    let ending_future_event = Event::find(ending_future_event.id, connection).unwrap();
    assert!(ending_future_event.settled_at.is_none());

    let display_settlement = settlement.clone().for_display(connection).unwrap();
    assert_eq!(display_settlement.event_entries.len(), 2);
    let event_entries_data = display_settlement
        .event_entries
        .iter()
        .find(|event_entry| event_entry.event == past_event.for_display(connection).unwrap())
        .unwrap();
    let event_entries = &event_entries_data.entries;
    assert_eq!(event_entries.len(), 4);

    let ticket_type_entry = event_entries
        .clone()
        .into_iter()
        .find(|e| {
            e.settlement_entry_type == SettlementEntryTypes::TicketType
                && e.ticket_type_id == Some(ticket_type.id)
                && e.face_value_in_cents == 150
        })
        .unwrap();
    assert_eq!(ticket_type_entry.settlement_id, settlement.id);
    assert_eq!(ticket_type_entry.event_id, past_event.id);
    assert_eq!(ticket_type_entry.ticket_type_id, Some(ticket_type.id));
    assert_eq!(ticket_type_entry.face_value_in_cents, 150);
    assert_eq!(ticket_type_entry.revenue_share_value_in_cents, 30);
    assert_eq!(ticket_type_entry.online_sold_quantity, 3);
    assert_eq!(ticket_type_entry.fee_sold_quantity, 3);
    assert_eq!(
        ticket_type_entry.total_sales_in_cents,
        ticket_type_entry.online_sold_quantity * ticket_type_entry.face_value_in_cents
            + ticket_type_entry.fee_sold_quantity * ticket_type_entry.revenue_share_value_in_cents
    );

    let ticket_type_entry = event_entries
        .clone()
        .into_iter()
        .find(|e| {
            e.settlement_entry_type == SettlementEntryTypes::TicketType
                && e.ticket_type_id == Some(ticket_type.id)
                && e.face_value_in_cents == 140
        })
        .unwrap();
    assert_eq!(ticket_type_entry.settlement_id, settlement.id);
    assert_eq!(ticket_type_entry.event_id, past_event.id);
    assert_eq!(ticket_type_entry.ticket_type_id, Some(ticket_type.id));
    assert_eq!(ticket_type_entry.face_value_in_cents, 140);
    assert_eq!(ticket_type_entry.revenue_share_value_in_cents, 30);
    assert_eq!(ticket_type_entry.online_sold_quantity, 5);
    assert_eq!(ticket_type_entry.fee_sold_quantity, 5);
    assert_eq!(
        ticket_type_entry.total_sales_in_cents,
        ticket_type_entry.online_sold_quantity * ticket_type_entry.face_value_in_cents
            + ticket_type_entry.fee_sold_quantity * ticket_type_entry.revenue_share_value_in_cents
    );

    let ticket_type_entry = event_entries
        .clone()
        .into_iter()
        .find(|e| {
            e.settlement_entry_type == SettlementEntryTypes::TicketType
                && e.ticket_type_id == Some(ticket_type.id)
                && e.face_value_in_cents == 130
        })
        .unwrap();
    assert_eq!(ticket_type_entry.settlement_id, settlement.id);
    assert_eq!(ticket_type_entry.event_id, past_event.id);
    assert_eq!(ticket_type_entry.ticket_type_id, Some(ticket_type.id));
    assert_eq!(ticket_type_entry.face_value_in_cents, 130);
    assert_eq!(ticket_type_entry.revenue_share_value_in_cents, 30);
    assert_eq!(ticket_type_entry.online_sold_quantity, 2);
    assert_eq!(ticket_type_entry.fee_sold_quantity, 2);
    assert_eq!(
        ticket_type_entry.total_sales_in_cents,
        ticket_type_entry.online_sold_quantity * ticket_type_entry.face_value_in_cents
            + ticket_type_entry.fee_sold_quantity * ticket_type_entry.revenue_share_value_in_cents
    );

    let event_fee_entry = event_entries
        .clone()
        .into_iter()
        .find(|e| e.settlement_entry_type == SettlementEntryTypes::EventFees && e.event_id == past_event.id)
        .unwrap();
    assert_eq!(event_fee_entry.settlement_id, settlement.id);
    assert_eq!(event_fee_entry.event_id, past_event.id);
    assert_eq!(event_fee_entry.ticket_type_id, None);
    assert_eq!(event_fee_entry.face_value_in_cents, 0);
    assert_eq!(event_fee_entry.revenue_share_value_in_cents, 150);
    assert_eq!(event_fee_entry.online_sold_quantity, 0);
    assert_eq!(event_fee_entry.fee_sold_quantity, 4);
    assert_eq!(event_fee_entry.total_sales_in_cents, 600);

    let event_entries_data = display_settlement
        .event_entries
        .iter()
        .find(|event_entry| event_entry.event == ending_future_event.for_display(connection).unwrap())
        .unwrap();
    let event_entries = &event_entries_data.entries;
    assert_eq!(event_entries.len(), 2);

    let ticket_type_entry = event_entries
        .clone()
        .into_iter()
        .find(|e| {
            e.settlement_entry_type == SettlementEntryTypes::TicketType && e.ticket_type_id == Some(ticket_type2.id)
        })
        .unwrap();
    assert_eq!(ticket_type_entry.settlement_id, settlement.id);
    assert_eq!(ticket_type_entry.event_id, ending_future_event.id);
    assert_eq!(ticket_type_entry.ticket_type_id, Some(ticket_type2.id));
    assert_eq!(ticket_type_entry.face_value_in_cents, 150);
    assert_eq!(ticket_type_entry.revenue_share_value_in_cents, 30);
    assert_eq!(ticket_type_entry.online_sold_quantity, 5);
    assert_eq!(ticket_type_entry.fee_sold_quantity, 5);
    assert_eq!(
        ticket_type_entry.total_sales_in_cents,
        ticket_type_entry.online_sold_quantity * ticket_type_entry.face_value_in_cents
            + ticket_type_entry.fee_sold_quantity * ticket_type_entry.revenue_share_value_in_cents
    );

    let event_fee_entry = event_entries
        .clone()
        .into_iter()
        .find(|e| e.settlement_entry_type == SettlementEntryTypes::EventFees && e.event_id == ending_future_event.id)
        .unwrap();
    assert_eq!(event_fee_entry.settlement_id, settlement.id);
    assert_eq!(event_fee_entry.event_id, ending_future_event.id);
    assert_eq!(event_fee_entry.ticket_type_id, None);
    assert_eq!(event_fee_entry.face_value_in_cents, 0);
    assert_eq!(event_fee_entry.revenue_share_value_in_cents, 150);
    assert_eq!(event_fee_entry.online_sold_quantity, 0);
    assert_eq!(event_fee_entry.fee_sold_quantity, 1);
    assert_eq!(event_fee_entry.total_sales_in_cents, 150);

    // Event has just an order in the next settlement period
    let settlement2 = Settlement::create(
        organization.id,
        dates::now().finish(),
        dates::now().add_days(7).finish(),
        SettlementStatus::PendingSettlement,
        None,
        false,
    )
    .commit(None, connection)
    .unwrap();
    let ending_future_event = Event::find(ending_future_event.id, connection).unwrap();
    assert!(ending_future_event.settled_at.is_some());

    let display_settlement = settlement2.clone().for_display(connection).unwrap();
    assert_eq!(display_settlement.event_entries.len(), 1);
    let event_entries_data = display_settlement
        .event_entries
        .iter()
        .find(|event_entry| event_entry.event == past_event.for_display(connection).unwrap())
        .unwrap();
    let event_entries = &event_entries_data.entries;
    assert_eq!(event_entries.len(), 2);

    let ticket_type_entry = event_entries
        .clone()
        .into_iter()
        .find(|e| {
            e.settlement_entry_type == SettlementEntryTypes::TicketType
                && e.ticket_type_id == Some(ticket_type.id)
                && e.face_value_in_cents == 150
        })
        .unwrap();
    assert_eq!(ticket_type_entry.settlement_id, settlement2.id);
    assert_eq!(ticket_type_entry.event_id, past_event.id);
    assert_eq!(ticket_type_entry.ticket_type_id, Some(ticket_type.id));
    assert_eq!(ticket_type_entry.face_value_in_cents, 150);
    assert_eq!(ticket_type_entry.revenue_share_value_in_cents, 30);
    assert_eq!(ticket_type_entry.online_sold_quantity, 10);
    assert_eq!(ticket_type_entry.fee_sold_quantity, 10);
    assert_eq!(
        ticket_type_entry.total_sales_in_cents,
        ticket_type_entry.online_sold_quantity * ticket_type_entry.face_value_in_cents
            + ticket_type_entry.fee_sold_quantity * ticket_type_entry.revenue_share_value_in_cents
    );

    let event_fee_entry = event_entries
        .clone()
        .into_iter()
        .find(|e| e.settlement_entry_type == SettlementEntryTypes::EventFees && e.event_id == past_event.id)
        .unwrap();
    assert_eq!(event_fee_entry.settlement_id, settlement2.id);
    assert_eq!(event_fee_entry.event_id, past_event.id);
    assert_eq!(event_fee_entry.ticket_type_id, None);
    assert_eq!(event_fee_entry.face_value_in_cents, 0);
    assert_eq!(event_fee_entry.revenue_share_value_in_cents, 150);
    assert_eq!(event_fee_entry.online_sold_quantity, 0);
    assert_eq!(event_fee_entry.fee_sold_quantity, 1);
    assert_eq!(event_fee_entry.total_sales_in_cents, 150);

    // Refund alone in subsequent settlement
    let items = order.items(&connection).unwrap();
    let order_item = items.iter().find(|i| i.ticket_type_id == Some(ticket_type.id)).unwrap();
    let ticket = &tickets[1];
    let refund_items = vec![RefundItemRequest {
        order_item_id: order_item.id,
        ticket_instance_id: Some(ticket.id),
    }];
    let (refund, _) = order.refund(&refund_items, user.id, None, false, connection).unwrap();
    diesel::update(refunds::table.filter(refunds::id.eq(refund.id)))
        .set(refunds::created_at.eq(Utc::now().naive_utc() + Duration::days(9)))
        .execute(connection)
        .unwrap();
    assert_eq!(refund.settlement_id, None);
    let settlement3 = Settlement::create(
        organization.id,
        dates::now().add_days(7).finish(),
        dates::now().add_days(14).finish(),
        SettlementStatus::PendingSettlement,
        None,
        false,
    )
    .commit(None, connection)
    .unwrap();

    let display_settlement = settlement3.clone().for_display(connection).unwrap();
    assert_eq!(display_settlement.event_entries.len(), 1);
    let event_entries_data = display_settlement
        .event_entries
        .iter()
        .find(|event_entry| event_entry.event == past_event.for_display(connection).unwrap())
        .unwrap();
    let event_entries = &event_entries_data.entries;
    assert_eq!(event_entries.len(), 1);

    let ticket_type_entry = event_entries
        .clone()
        .into_iter()
        .find(|e| {
            e.settlement_entry_type == SettlementEntryTypes::TicketType
                && e.ticket_type_id == Some(ticket_type.id)
                && e.face_value_in_cents == 150
        })
        .unwrap();
    assert_eq!(ticket_type_entry.settlement_id, settlement3.id);
    assert_eq!(ticket_type_entry.event_id, past_event.id);
    assert_eq!(ticket_type_entry.ticket_type_id, Some(ticket_type.id));
    assert_eq!(ticket_type_entry.face_value_in_cents, 150);
    assert_eq!(ticket_type_entry.revenue_share_value_in_cents, 30);
    assert_eq!(ticket_type_entry.online_sold_quantity, -1);
    assert_eq!(ticket_type_entry.fee_sold_quantity, -1);
    assert_eq!(
        ticket_type_entry.total_sales_in_cents,
        -ticket_type_entry.face_value_in_cents + -ticket_type_entry.revenue_share_value_in_cents
    );
    // Reloading records shows settlement associated with them marking it as having been processed
    let order = Order::find(order.id, connection).unwrap();
    assert_eq!(order.settlement_id, Some(settlement.id));
    let refund = Refund::find(refund.id, connection).unwrap();
    assert_eq!(refund.settlement_id, Some(settlement3.id));
}

#[test]
fn create() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let organization = project.create_organization().finish();
    project
        .create_event()
        .with_name("NewEvent".into())
        .with_organization(&organization)
        .finish();
    let domain_events = DomainEvent::find(
        Tables::Organizations,
        Some(organization.id),
        Some(DomainEventTypes::SettlementReportProcessed),
        connection,
    )
    .unwrap();
    assert_eq!(0, domain_events.len());
    let settlement = Settlement::create(
        organization.id,
        NaiveDate::from_ymd(2016, 7, 8).and_hms(4, 10, 11),
        NaiveDate::from_ymd(2020, 7, 8).and_hms(4, 10, 11),
        SettlementStatus::PendingSettlement,
        Some("test comment".to_string()),
        true,
    )
    .commit(Some(user.clone()), connection)
    .unwrap();

    assert_eq!(settlement.organization_id, organization.id);
    assert_eq!(settlement.comment, Some("test comment".to_string()));

    let domain_events = DomainEvent::find(
        Tables::Organizations,
        Some(organization.id),
        Some(DomainEventTypes::SettlementReportProcessed),
        connection,
    )
    .unwrap();
    assert_eq!(1, domain_events.len());
    assert_eq!(domain_events[0].user_id, Some(user.id));
}

#[test]
fn create_with_validation_errors() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().finish();
    project
        .create_event()
        .with_name("NewEvent".into())
        .with_organization(&organization)
        .finish();
    let result = Settlement::create(
        organization.id,
        NaiveDate::from_ymd(2020, 7, 8).and_hms(4, 10, 11),
        NaiveDate::from_ymd(2016, 7, 8).and_hms(4, 10, 11),
        SettlementStatus::PendingSettlement,
        None,
        true,
    )
    .commit(None, connection);

    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("start_time"));
                assert_eq!(errors["start_time"].len(), 1);
                assert_eq!(errors["start_time"][0].code, "end_time_before_start_time");
                assert_eq!(
                    &errors["start_time"][0].message.clone().unwrap().into_owned(),
                    "End time must be after start time"
                );
            }
            _ => panic!("Expected validation error"),
        },
    }
}

#[test]
fn adjustments() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let settlement = project.create_settlement().finish();
    assert!(settlement.adjustments(connection).unwrap().is_empty());

    let settlement_adjustment = project
        .create_settlement_adjustment()
        .with_settlement(&settlement)
        .finish();
    assert_eq!(settlement.adjustments(connection).unwrap(), vec![settlement_adjustment]);
}

#[test]
fn destroy() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let settlement = project.create_settlement().finish();
    settlement.clone().destroy(connection).unwrap();
    assert!(Settlement::find(settlement.id, connection).is_err());
}

#[test]
fn find() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let venue = project.create_venue().finish();
    let user = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .finish();
    let _event = project
        .create_event()
        .with_name("NewEvent".into())
        .with_organization(&organization)
        .with_venue(&venue)
        .finish();
    let settlement = project.create_settlement().with_organization(&organization).finish();
    let read_settlement = Settlement::find(settlement.id, connection).unwrap();
    assert_eq!(settlement.id, read_settlement.id);
    assert_eq!(settlement.comment, read_settlement.comment);
    assert_eq!(settlement.start_time, read_settlement.start_time);
}

#[test]
fn for_display() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let venue = project.create_venue().finish();
    let user = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .finish();
    let event = project
        .create_event()
        .with_name("NewEvent".into())
        .with_organization(&organization)
        .with_venue(&venue)
        .with_ticket_pricing()
        .finish();
    let settlement = project.create_settlement().with_organization(&organization).finish();
    let entry = project
        .create_settlement_entry()
        .with_settlement(&settlement)
        .with_event(&event)
        .finish();
    let adjustment = project
        .create_settlement_adjustment()
        .with_settlement(&settlement)
        .finish();
    let display_settlement = settlement.clone().for_display(connection).unwrap();
    assert_eq!(display_settlement.settlement.id, settlement.id);
    let display_event = event.for_display(connection).unwrap();
    assert_eq!(display_settlement.event_entries.len(), 1);
    assert_eq!(display_settlement.event_entries[0].event, display_event);
    assert_eq!(display_settlement.event_entries[0].entries[0].id, entry.id);
    assert_eq!(display_settlement.adjustments[0], adjustment);
}

#[test]
fn find_for_organization() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let venue = project.create_venue().finish();
    let user = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .finish();
    let _event = project
        .create_event()
        .with_name("NewEvent".into())
        .with_organization(&organization)
        .with_venue(&venue)
        .finish();
    let _event2 = project
        .create_event()
        .with_name("NewEvent2".into())
        .with_organization(&organization)
        .with_venue(&venue)
        .finish();
    let settlement = project.create_settlement().with_organization(&organization).finish();
    let settlements = Settlement::find_for_organization(organization.id, None, None, false, connection)
        .unwrap()
        .data;
    assert_eq!(settlements.len(), 1);
    assert_eq!(settlements[0].id, settlement.id);
}
