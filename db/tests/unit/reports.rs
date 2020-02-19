use bigneon_db::dev::TestProject;
use bigneon_db::models::*;
use bigneon_db::schema::orders;
use bigneon_db::utils::dates;
use chrono::{Datelike, Duration, NaiveDate, TimeZone, Utc};
use chrono_tz::Tz;
use diesel;
use diesel::prelude::*;

#[test]
fn find_event_reports_for_processing() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project
        .create_event()
        .with_ticket_type()
        .starting(dates::now().add_days(1).finish())
        .ending(dates::now().add_days(2).finish())
        .finish();
    let event2 = project
        .create_event()
        .with_ticket_type()
        .starting(dates::now().add_days(1).finish())
        .ending(dates::now().add_days(2).finish())
        .finish();
    let event3 = project
        .create_event()
        .with_ticket_type()
        .starting(dates::now().add_days(1).finish())
        .ending(dates::now().add_days(2).finish())
        .finish();

    // Only events on sale that have not ended are included
    assert_eq!(
        Report::find_event_reports_for_processing(connection)
            .unwrap()
            .get(&ReportTypes::TicketCounts)
            .unwrap(),
        &vec![]
    );

    // Add ticket pricing for each event
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    ticket_type
        .add_ticket_pricing(
            "Pricing1".into(),
            dates::now().add_days(-1).finish(),
            dates::now().add_days(2).finish(),
            3000,
            false,
            None,
            None,
            connection,
        )
        .unwrap();
    let ticket_type2 = &event2.ticket_types(true, None, connection).unwrap()[0];
    ticket_type2
        .add_ticket_pricing(
            "Pricing1".into(),
            dates::now().add_days(-1).finish(),
            dates::now().add_days(2).finish(),
            3000,
            false,
            None,
            None,
            connection,
        )
        .unwrap();
    let ticket_type3 = &event3.ticket_types(true, None, connection).unwrap()[0];
    ticket_type3
        .add_ticket_pricing(
            "Pricing1".into(),
            dates::now().add_days(-1).finish(),
            dates::now().add_days(2).finish(),
            3000,
            false,
            None,
            None,
            connection,
        )
        .unwrap();

    // Adjust event end to prevent some from being included
    let event = event
        .update(
            None,
            EventEditableAttributes {
                event_start: Some(dates::now().add_days(-1).finish()),
                event_end: Some(dates::now().add_minutes(-10).finish()),
                ..Default::default()
            },
            connection,
        )
        .unwrap();
    event2
        .update(
            None,
            EventEditableAttributes {
                event_start: Some(dates::now().add_days(-3).finish()),
                event_end: Some(dates::now().add_days(-2).finish()),
                ..Default::default()
            },
            connection,
        )
        .unwrap();
    let event3 = event3
        .update(
            None,
            EventEditableAttributes {
                event_start: Some(dates::now().add_days(-3).finish()),
                event_end: Some(dates::now().add_days(5).finish()),
                ..Default::default()
            },
            connection,
        )
        .unwrap();

    // Event2 is not included as it ended 2 days ago
    assert_eq!(
        Report::find_event_reports_for_processing(connection)
            .unwrap()
            .get(&ReportTypes::TicketCounts)
            .unwrap(),
        &vec![event.clone(), event3.clone()]
    );

    // Unpublish first event, should hide event
    event.unpublish(None, connection).unwrap();
    assert_eq!(
        Report::find_event_reports_for_processing(connection)
            .unwrap()
            .get(&ReportTypes::TicketCounts)
            .unwrap(),
        &vec![event3]
    );
}

#[test]
fn next_automatic_report_date() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().finish();
    let pt_timezone: Tz = "America/Los_Angeles".parse().unwrap();
    let now = pt_timezone.from_utc_datetime(&Utc::now().naive_utc());
    let pt_today = pt_timezone.ymd(now.year(), now.month(), now.day()).and_hms(4, 0, 0);
    let expected = pt_today.naive_utc() + Duration::days(1);
    assert_eq!(Report::next_automatic_report_date().unwrap(), expected);

    // Organization timezone has no effect on the date
    organization
        .update(
            OrganizationEditableAttributes {
                timezone: Some("Africa/Johannesburg".to_string()),
                ..Default::default()
            },
            None,
            &"encryption_key".to_string(),
            connection,
        )
        .unwrap();
    assert_eq!(Report::next_automatic_report_date().unwrap(), expected);
}

#[test]
fn create_next_automatic_report_domain_action() {
    let project = TestProject::new();
    let connection = project.get_connection();
    assert!(
        DomainAction::upcoming_domain_action(None, None, DomainActionTypes::SendAutomaticReportEmails, connection)
            .unwrap()
            .is_none()
    );

    Report::create_next_automatic_report_domain_action(connection).unwrap();
    let domain_action =
        DomainAction::upcoming_domain_action(None, None, DomainActionTypes::SendAutomaticReportEmails, connection)
            .unwrap()
            .unwrap();
    assert_eq!(
        domain_action.scheduled_at,
        Report::next_automatic_report_date().unwrap()
    );
    assert_eq!(domain_action.status, DomainActionStatus::Pending);
    assert_eq!(domain_action.main_table, None);
    assert_eq!(domain_action.main_table_id, None);
}

#[test]
fn ticket_count_report() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().with_event_fee().with_fees().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_name("Event1".to_string())
        .with_tickets()
        .finish();
    let mut ticket_type = event.ticket_types(true, None, connection).unwrap().remove(0);
    let event2 = project
        .create_event()
        .with_organization(&organization)
        .with_name("Event2".to_string())
        .with_tickets()
        .finish();
    let ticket_type2 = event2.ticket_types(true, None, connection).unwrap().remove(0);
    let event3 = project
        .create_event()
        .with_name("Event3".to_string())
        .with_tickets()
        .finish();

    let ticket_pricing = ticket_type.current_ticket_pricing(false, connection).unwrap();
    let ticket_pricing2 = ticket_type2.current_ticket_pricing(false, connection).unwrap();

    let user = project.create_user().finish();
    let user2 = project.create_user().finish();
    let user3 = project.create_user().finish();

    project
        .create_order()
        .quantity(2)
        .for_event(&event)
        .for_user(&user)
        .is_paid()
        .finish();

    // Partially refunded order
    let mut order = project
        .create_order()
        .quantity(2)
        .for_event(&event)
        .on_behalf_of_user(&user2)
        .for_user(&user)
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
    order.refund(&refund_items, user.id, None, false, connection).unwrap();

    // Redeem ticket
    let ticket2 = &tickets[1];
    TicketInstance::redeem_ticket(
        ticket2.id,
        ticket2.redeem_key.clone().unwrap(),
        user.id,
        CheckInSource::GuestList,
        connection,
    )
    .unwrap();

    project
        .create_order()
        .quantity(2)
        .for_event(&event2)
        .for_user(&user3)
        .is_paid()
        .finish();

    // Hold order
    let hold = project
        .create_hold()
        .with_quantity(10)
        .with_ticket_type_id(ticket_type.id)
        .with_hold_type(HoldTypes::Discount)
        .finish();
    project
        .create_order()
        .quantity(2)
        .for_event(&event)
        .for_user(&user)
        .is_paid()
        .with_redemption_code(hold.redemption_code.clone().unwrap())
        .finish();
    project
        .create_order()
        .quantity(1)
        .for_event(&event)
        .for_user(&user)
        .on_behalf_of_user(&user3)
        .is_paid()
        .with_redemption_code(hold.redemption_code.clone().unwrap())
        .finish();

    // Comp order
    let comp = project
        .create_hold()
        .with_quantity(10)
        .with_ticket_type_id(ticket_type.id)
        .with_hold_type(HoldTypes::Comp)
        .finish();
    project
        .create_order()
        .quantity(2)
        .for_event(&event)
        .for_user(&user)
        .is_paid()
        .with_redemption_code(comp.redemption_code.clone().unwrap())
        .finish();

    // Discount code order
    let code = project
        .create_code()
        .with_event(&event)
        .for_ticket_type(&ticket_type)
        .with_discount_in_cents(Some(50))
        .finish();
    project
        .create_order()
        .quantity(2)
        .for_event(&event)
        .for_user(&user)
        .is_paid()
        .with_redemption_code(code.redemption_code.clone())
        .finish();

    // Orders for other organization
    project
        .create_order()
        .quantity(2)
        .for_event(&event3)
        .for_user(&user)
        .is_paid()
        .finish();
    project
        .create_order()
        .quantity(2)
        .for_event(&event3)
        .for_user(&user3)
        .is_paid()
        .finish();

    // New price point for ticket type but has same price so orders won't create a new row
    let old_pricing = TicketPricing::get_default(ticket_type.id, connection).unwrap();
    ticket_type = ticket_type
        .update(TicketTypeEditableAttributes { ..Default::default() }, None, connection)
        .unwrap();
    let new_pricing = TicketPricing::get_default(ticket_type.id, connection).unwrap();
    assert_ne!(old_pricing.id, new_pricing.id);
    let order_dupe_price = project
        .create_order()
        .quantity(1)
        .for_event(&event)
        .for_user(&user)
        .is_paid()
        .finish();
    assert_eq!(
        order_dupe_price
            .items(connection)
            .unwrap()
            .iter()
            .find(|i| i.ticket_type_id == Some(ticket_type.id))
            .unwrap()
            .ticket_pricing_id,
        Some(new_pricing.id)
    );

    // New price point for ticket type with new price so orders will create a new row
    let old_pricing = TicketPricing::get_default(ticket_type.id, connection).unwrap();
    ticket_type = ticket_type
        .update(
            TicketTypeEditableAttributes {
                price_in_cents: Some(200),
                ..Default::default()
            },
            None,
            connection,
        )
        .unwrap();
    let new_pricing = TicketPricing::get_default(ticket_type.id, connection).unwrap();
    assert_ne!(old_pricing.id, new_pricing.id);
    let order_new_price = project
        .create_order()
        .quantity(1)
        .for_event(&event)
        .for_user(&user)
        .is_paid()
        .finish();
    assert_eq!(
        order_new_price
            .items(connection)
            .unwrap()
            .iter()
            .find(|i| i.ticket_type_id == Some(ticket_type.id))
            .unwrap()
            .ticket_pricing_id,
        Some(new_pricing.id)
    );

    // Specific event
    let result = Report::ticket_count_report(Some(event.id), None, connection).unwrap();

    let expected_counts = vec![TicketCountRow {
        organization_id: Some(event.organization_id),
        event_id: Some(event.id),
        ticket_type_id: Some(ticket_type.id),
        ticket_name: Some(ticket_type.name.clone()),
        ticket_status: Some(ticket_type.status.to_string()),
        event_name: Some(event.name.clone()),
        organization_name: Some(organization.name.clone()),
        allocation_count_including_nullified: 100,
        allocation_count: 100,
        unallocated_count: 88,
        reserved_count: 0,
        redeemed_count: 1,
        purchased_count: 12,
        purchased_yesterday_count: 0,
        comp_purchased_yesterday_count: 0,
        nullified_count: 0,
        available_for_purchase_count: 73,
        total_refunded_count: 1,
        comp_count: 10,
        comp_available_count: 8,
        comp_redeemed_count: 0,
        comp_purchased_count: 2,
        comp_reserved_count: 0,
        comp_nullified_count: 0,
        hold_count: 10,
        hold_available_count: 7,
        hold_redeemed_count: 0,
        hold_purchased_count: 3,
        hold_reserved_count: 0,
        hold_nullified_count: 0,
    }];
    assert_eq!(expected_counts, result.counts);
    assert_eq!(8, result.sales.len());

    // Box office orders have no fees so a record is returned for each -- if they had the same fees they would combine
    assert_eq!(
        result
            .sales
            .iter()
            .find(|s| s.hold_id == Some(hold.id) && s.box_office_sale_count == 0),
        Some(&TicketSalesRow {
            organization_id: Some(event.organization_id),
            event_id: Some(event.id),
            ticket_type_id: Some(ticket_type.id),
            hold_id: Some(hold.id),
            ticket_name: Some(ticket_type.name.clone()),
            ticket_status: Some(ticket_type.status.to_string()),
            event_name: Some(event.name.clone()),
            hold_name: Some(hold.name.clone()),
            ticket_pricing_name: Some(ticket_pricing.name.clone()),
            ticket_pricing_price_in_cents: Some(100),
            promo_code_discounted_ticket_price: Some(-10),
            box_office_order_count: 0,
            online_order_count: 1,
            box_office_sales_in_cents: 0,
            online_sales_in_cents: 180,
            box_office_face_sales_in_cents: 0,
            online_face_sales_in_cents: 200,
            box_office_sale_count: 0,
            online_sale_count: 2,
            online_fee_count: 2,
            total_online_fees_in_cents: 20,
            company_online_fees_in_cents: 8,
            client_online_fees_in_cents: 12,
            user_count: 1,
            ..Default::default()
        })
    );
    assert_eq!(
        result
            .sales
            .iter()
            .find(|s| s.hold_id == Some(hold.id) && s.box_office_sale_count > 0),
        Some(&TicketSalesRow {
            organization_id: Some(event.organization_id),
            event_id: Some(event.id),
            ticket_type_id: Some(ticket_type.id),
            hold_id: Some(hold.id),
            ticket_name: Some(ticket_type.name.clone()),
            ticket_status: Some(ticket_type.status.to_string()),
            event_name: Some(event.name.clone()),
            hold_name: Some(hold.name.clone()),
            ticket_pricing_name: Some(ticket_pricing.name.clone()),
            ticket_pricing_price_in_cents: Some(100),
            promo_code_discounted_ticket_price: Some(-10),
            box_office_order_count: 1,
            online_order_count: 0,
            box_office_sales_in_cents: 90,
            online_sales_in_cents: 0,
            box_office_face_sales_in_cents: 100,
            online_face_sales_in_cents: 0,
            box_office_sale_count: 1,
            online_sale_count: 0,
            total_online_fees_in_cents: 0,
            company_online_fees_in_cents: 0,
            client_online_fees_in_cents: 0,
            user_count: 1,
            ..Default::default()
        })
    );

    assert_eq!(
        result.sales.iter().find(|s| s.hold_id == Some(comp.id)),
        Some(&TicketSalesRow {
            organization_id: Some(event.organization_id),
            event_id: Some(event.id),
            ticket_type_id: Some(ticket_type.id),
            hold_id: Some(comp.id),
            ticket_name: Some(ticket_type.name.clone()),
            ticket_status: Some(ticket_type.status.to_string()),
            event_name: Some(event.name.clone()),
            hold_name: Some(comp.name.clone()),
            ticket_pricing_name: Some(ticket_pricing.name.clone()),
            ticket_pricing_price_in_cents: Some(100),
            promo_code_discounted_ticket_price: Some(-100),
            online_order_count: 1,
            online_face_sales_in_cents: 200,
            comp_sale_count: 2,
            user_count: 1,
            ..Default::default()
        })
    );

    assert_eq!(
        result
            .sales
            .iter()
            .find(|s| s.promo_redemption_code == Some(code.redemption_code.clone())),
        Some(&TicketSalesRow {
            organization_id: Some(event.organization_id),
            event_id: Some(event.id),
            ticket_type_id: Some(ticket_type.id),
            hold_id: Some(code.id),
            ticket_name: Some(ticket_type.name.clone()),
            ticket_status: Some(ticket_type.status.to_string()),
            event_name: Some(event.name.clone()),
            hold_name: Some(code.name.clone()),
            promo_redemption_code: Some(code.redemption_code.clone()),
            ticket_pricing_name: Some(ticket_pricing.name.clone()),
            ticket_pricing_price_in_cents: Some(100),
            promo_code_discounted_ticket_price: Some(-50),
            online_order_count: 1,
            online_sales_in_cents: 100,
            online_face_sales_in_cents: 200,
            online_sale_count: 2,
            online_fee_count: 2,
            total_online_fees_in_cents: 20,
            company_online_fees_in_cents: 8,
            client_online_fees_in_cents: 12,
            user_count: 1,
            ..Default::default()
        })
    );

    assert_eq!(
        result.sales.iter().find(|s| s.promo_redemption_code == None
            && s.hold_id == None
            && s.ticket_type_id == Some(ticket_type.id)
            && s.ticket_pricing_price_in_cents != Some(200)
            && s.box_office_sale_count == 0),
        Some(&TicketSalesRow {
            organization_id: Some(event.organization_id),
            event_id: Some(event.id),
            ticket_type_id: Some(ticket_type.id),
            ticket_name: Some(ticket_type.name.clone()),
            ticket_status: Some(ticket_type.status.to_string()),
            event_name: Some(event.name.clone()),
            ticket_pricing_name: Some(ticket_pricing.name.clone()),
            ticket_pricing_price_in_cents: Some(100),
            box_office_order_count: 0,
            online_order_count: 2,
            box_office_sales_in_cents: 0,
            online_sales_in_cents: 300,
            box_office_face_sales_in_cents: 0,
            online_face_sales_in_cents: 300,
            box_office_sale_count: 0,
            box_office_refunded_count: 0,
            online_sale_count: 3,
            online_fee_count: 3,
            total_online_fees_in_cents: 60,
            company_online_fees_in_cents: 24,
            client_online_fees_in_cents: 36,
            user_count: 1,
            ..Default::default()
        })
    );
    assert_eq!(
        result.sales.iter().find(|s| s.promo_redemption_code == None
            && s.hold_id == None
            && s.ticket_type_id == Some(ticket_type.id)
            && s.ticket_pricing_price_in_cents != Some(200)
            && s.box_office_sale_count > 0),
        Some(&TicketSalesRow {
            organization_id: Some(event.organization_id),
            event_id: Some(event.id),
            ticket_type_id: Some(ticket_type.id),
            ticket_name: Some(ticket_type.name.clone()),
            ticket_status: Some(ticket_type.status.to_string()),
            event_name: Some(event.name.clone()),
            ticket_pricing_name: Some(ticket_pricing.name.clone()),
            ticket_pricing_price_in_cents: Some(100),
            box_office_order_count: 1,
            online_order_count: 0,
            box_office_sales_in_cents: 100,
            online_sales_in_cents: 0,
            box_office_face_sales_in_cents: 100,
            online_face_sales_in_cents: 0,
            box_office_sale_count: 1,
            box_office_refunded_count: 1,
            online_sale_count: 0,
            total_online_fees_in_cents: 0,
            company_online_fees_in_cents: 0,
            client_online_fees_in_cents: 0,
            user_count: 1,
            ..Default::default()
        })
    );

    assert_eq!(
        result.sales.iter().find(|s| s.promo_redemption_code == None
            && s.hold_id == None
            && s.ticket_type_id == Some(ticket_type.id)
            && s.ticket_pricing_price_in_cents == Some(200)),
        Some(&TicketSalesRow {
            organization_id: Some(event.organization_id),
            event_id: Some(event.id),
            ticket_type_id: Some(ticket_type.id),
            ticket_name: Some(ticket_type.name.clone()),
            ticket_status: Some(ticket_type.status.to_string()),
            event_name: Some(event.name.clone()),
            ticket_pricing_name: Some(ticket_pricing.name.clone()),
            ticket_pricing_price_in_cents: Some(200),
            box_office_order_count: 0,
            online_order_count: 1,
            box_office_sales_in_cents: 0,
            online_sales_in_cents: 200,
            box_office_face_sales_in_cents: 0,
            online_face_sales_in_cents: 200,
            box_office_sale_count: 0,
            box_office_refunded_count: 0,
            online_sale_count: 1,
            online_fee_count: 1,
            total_online_fees_in_cents: 20,
            company_online_fees_in_cents: 8,
            client_online_fees_in_cents: 12,
            user_count: 1,
            ..Default::default()
        })
    );

    assert_eq!(
        result
            .sales
            .iter()
            .find(|s| s.promo_redemption_code == None && s.hold_id == None && s.ticket_type_id == None),
        Some(&TicketSalesRow {
            organization_id: Some(event.organization_id),
            event_id: Some(event.id),
            per_order_company_online_fees: 500,
            per_order_client_online_fees: 750,
            per_order_total_fees_in_cents: 1250,
            ..Default::default()
        })
    );

    // Organization check
    let result = Report::ticket_count_report(None, Some(organization.id), connection).unwrap();

    assert_eq!(2, result.counts.len());
    assert_eq!(
        result.counts.iter().find(|c| c.event_id == Some(event.id)),
        Some(&TicketCountRow {
            organization_id: Some(event.organization_id),
            event_id: Some(event.id),
            ticket_type_id: Some(ticket_type.id),
            ticket_name: Some(ticket_type.name.clone()),
            ticket_status: Some(ticket_type.status.to_string()),
            event_name: Some(event.name.clone()),
            organization_name: Some(organization.name.clone()),
            allocation_count_including_nullified: 100,
            allocation_count: 100,
            unallocated_count: 88,
            reserved_count: 0,
            redeemed_count: 1,
            purchased_count: 12,
            purchased_yesterday_count: 0,
            comp_purchased_yesterday_count: 0,
            nullified_count: 0,
            available_for_purchase_count: 73,
            total_refunded_count: 1,
            comp_count: 10,
            comp_available_count: 8,
            comp_redeemed_count: 0,
            comp_purchased_count: 2,
            comp_reserved_count: 0,
            comp_nullified_count: 0,
            hold_count: 10,
            hold_available_count: 7,
            hold_redeemed_count: 0,
            hold_purchased_count: 3,
            hold_reserved_count: 0,
            hold_nullified_count: 0,
        })
    );
    assert_eq!(
        result.counts.iter().find(|c| c.event_id == Some(event2.id)),
        Some(&TicketCountRow {
            organization_id: Some(event2.organization_id),
            event_id: Some(event2.id),
            ticket_type_id: Some(ticket_type2.id),
            ticket_name: Some(ticket_type2.name.clone()),
            ticket_status: Some(ticket_type2.status.to_string()),
            event_name: Some(event2.name.clone()),
            organization_name: Some(organization.name.clone()),
            allocation_count_including_nullified: 100,
            allocation_count: 100,
            unallocated_count: 98,
            reserved_count: 0,
            redeemed_count: 0,
            purchased_count: 2,
            purchased_yesterday_count: 0,
            comp_purchased_yesterday_count: 0,
            nullified_count: 0,
            available_for_purchase_count: 98,
            total_refunded_count: 0,
            comp_count: 0,
            comp_available_count: 0,
            comp_redeemed_count: 0,
            comp_purchased_count: 0,
            comp_reserved_count: 0,
            comp_nullified_count: 0,
            hold_count: 0,
            hold_available_count: 0,
            hold_redeemed_count: 0,
            hold_purchased_count: 0,
            hold_reserved_count: 0,
            hold_nullified_count: 0,
        })
    );

    assert_eq!(10, result.sales.len());
    assert_eq!(
        result
            .sales
            .iter()
            .find(|s| s.hold_id == Some(hold.id) && s.box_office_sale_count == 0),
        Some(&TicketSalesRow {
            organization_id: Some(event.organization_id),
            event_id: Some(event.id),
            ticket_type_id: Some(ticket_type.id),
            hold_id: Some(hold.id),
            ticket_name: Some(ticket_type.name.clone()),
            ticket_status: Some(ticket_type.status.to_string()),
            event_name: Some(event.name.clone()),
            hold_name: Some(hold.name.clone()),
            ticket_pricing_name: Some(ticket_pricing.name.clone()),
            ticket_pricing_price_in_cents: Some(100),
            promo_code_discounted_ticket_price: Some(-10),
            box_office_order_count: 0,
            online_order_count: 1,
            box_office_sales_in_cents: 0,
            online_sales_in_cents: 180,
            box_office_face_sales_in_cents: 0,
            online_face_sales_in_cents: 200,
            box_office_sale_count: 0,
            online_sale_count: 2,
            online_fee_count: 2,
            total_online_fees_in_cents: 20,
            company_online_fees_in_cents: 8,
            client_online_fees_in_cents: 12,
            user_count: 1,
            ..Default::default()
        })
    );
    assert_eq!(
        result
            .sales
            .iter()
            .find(|s| s.hold_id == Some(hold.id) && s.box_office_sale_count > 0),
        Some(&TicketSalesRow {
            organization_id: Some(event.organization_id),
            event_id: Some(event.id),
            ticket_type_id: Some(ticket_type.id),
            hold_id: Some(hold.id),
            ticket_name: Some(ticket_type.name.clone()),
            ticket_status: Some(ticket_type.status.to_string()),
            event_name: Some(event.name.clone()),
            hold_name: Some(hold.name),
            ticket_pricing_name: Some(ticket_pricing.name.clone()),
            ticket_pricing_price_in_cents: Some(100),
            promo_code_discounted_ticket_price: Some(-10),
            box_office_order_count: 1,
            online_order_count: 0,
            box_office_sales_in_cents: 90,
            online_sales_in_cents: 0,
            box_office_face_sales_in_cents: 100,
            online_face_sales_in_cents: 0,
            box_office_sale_count: 1,
            online_sale_count: 0,
            total_online_fees_in_cents: 0,
            company_online_fees_in_cents: 0,
            client_online_fees_in_cents: 0,
            user_count: 1,
            ..Default::default()
        })
    );

    assert_eq!(
        result.sales.iter().find(|s| s.hold_id == Some(comp.id)),
        Some(&TicketSalesRow {
            organization_id: Some(event.organization_id),
            event_id: Some(event.id),
            ticket_type_id: Some(ticket_type.id),
            hold_id: Some(comp.id),
            ticket_name: Some(ticket_type.name.clone()),
            ticket_status: Some(ticket_type.status.to_string()),
            event_name: Some(event.name.clone()),
            hold_name: Some(comp.name),
            ticket_pricing_name: Some(ticket_pricing.name.clone()),
            ticket_pricing_price_in_cents: Some(100),
            promo_code_discounted_ticket_price: Some(-100),
            online_order_count: 1,
            online_face_sales_in_cents: 200,
            comp_sale_count: 2,
            user_count: 1,
            ..Default::default()
        })
    );

    assert_eq!(
        result
            .sales
            .iter()
            .find(|s| s.promo_redemption_code == Some(code.redemption_code.clone())),
        Some(&TicketSalesRow {
            organization_id: Some(event.organization_id),
            event_id: Some(event.id),
            ticket_type_id: Some(ticket_type.id),
            hold_id: Some(code.id),
            ticket_name: Some(ticket_type.name.clone()),
            ticket_status: Some(ticket_type.status.to_string()),
            event_name: Some(event.name.clone()),
            hold_name: Some(code.name),
            promo_redemption_code: Some(code.redemption_code),
            ticket_pricing_name: Some(ticket_pricing.name.clone()),
            ticket_pricing_price_in_cents: Some(100),
            promo_code_discounted_ticket_price: Some(-50),
            online_order_count: 1,
            online_sales_in_cents: 100,
            online_face_sales_in_cents: 200,
            online_sale_count: 2,
            online_fee_count: 2,
            total_online_fees_in_cents: 20,
            company_online_fees_in_cents: 8,
            client_online_fees_in_cents: 12,
            user_count: 1,
            ..Default::default()
        })
    );

    assert_eq!(
        result.sales.iter().find(|s| s.promo_redemption_code == None
            && s.hold_id == None
            && s.ticket_type_id == Some(ticket_type.id)
            && s.ticket_pricing_price_in_cents != Some(200)
            && s.box_office_sale_count == 0),
        Some(&TicketSalesRow {
            organization_id: Some(event.organization_id),
            event_id: Some(event.id),
            ticket_type_id: Some(ticket_type.id),
            ticket_name: Some(ticket_type.name.clone()),
            ticket_status: Some(ticket_type.status.to_string()),
            event_name: Some(event.name.clone()),
            ticket_pricing_name: Some(ticket_pricing.name.clone()),
            ticket_pricing_price_in_cents: Some(100),
            box_office_order_count: 0,
            online_order_count: 2,
            box_office_sales_in_cents: 0,
            online_sales_in_cents: 300,
            box_office_face_sales_in_cents: 0,
            online_face_sales_in_cents: 300,
            box_office_sale_count: 0,
            box_office_refunded_count: 0,
            online_sale_count: 3,
            online_fee_count: 3,
            total_online_fees_in_cents: 60,
            company_online_fees_in_cents: 24,
            client_online_fees_in_cents: 36,
            user_count: 1,
            ..Default::default()
        })
    );
    assert_eq!(
        result.sales.iter().find(|s| s.promo_redemption_code == None
            && s.hold_id == None
            && s.ticket_type_id == Some(ticket_type.id)
            && s.ticket_pricing_price_in_cents != Some(200)
            && s.box_office_sale_count > 0),
        Some(&TicketSalesRow {
            organization_id: Some(event.organization_id),
            event_id: Some(event.id),
            ticket_type_id: Some(ticket_type.id),
            ticket_name: Some(ticket_type.name.clone()),
            ticket_status: Some(ticket_type.status.to_string()),
            event_name: Some(event.name.clone()),
            ticket_pricing_name: Some(ticket_pricing.name.clone()),
            ticket_pricing_price_in_cents: Some(100),
            box_office_order_count: 1,
            online_order_count: 0,
            box_office_sales_in_cents: 100,
            online_sales_in_cents: 0,
            box_office_face_sales_in_cents: 100,
            online_face_sales_in_cents: 0,
            box_office_sale_count: 1,
            box_office_refunded_count: 1,
            online_sale_count: 0,
            total_online_fees_in_cents: 0,
            company_online_fees_in_cents: 0,
            client_online_fees_in_cents: 0,
            user_count: 1,
            ..Default::default()
        })
    );

    assert_eq!(
        result.sales.iter().find(|s| s.promo_redemption_code == None
            && s.hold_id == None
            && s.ticket_type_id == Some(ticket_type.id)
            && s.ticket_pricing_price_in_cents == Some(200)),
        Some(&TicketSalesRow {
            organization_id: Some(event.organization_id),
            event_id: Some(event.id),
            ticket_type_id: Some(ticket_type.id),
            ticket_name: Some(ticket_type.name.clone()),
            ticket_status: Some(ticket_type.status.to_string()),
            event_name: Some(event.name.clone()),
            ticket_pricing_name: Some(ticket_pricing.name.clone()),
            ticket_pricing_price_in_cents: Some(200),
            box_office_order_count: 0,
            online_order_count: 1,
            box_office_sales_in_cents: 0,
            online_sales_in_cents: 200,
            box_office_face_sales_in_cents: 0,
            online_face_sales_in_cents: 200,
            box_office_sale_count: 0,
            box_office_refunded_count: 0,
            online_sale_count: 1,
            online_fee_count: 1,
            total_online_fees_in_cents: 20,
            company_online_fees_in_cents: 8,
            client_online_fees_in_cents: 12,
            user_count: 1,
            ..Default::default()
        })
    );

    assert_eq!(
        result.sales.iter().find(|s| s.ticket_type_id == Some(ticket_type2.id)),
        Some(&TicketSalesRow {
            organization_id: Some(event2.organization_id),
            event_id: Some(event2.id),
            ticket_type_id: Some(ticket_type2.id),
            ticket_name: Some(ticket_type2.name.clone()),
            ticket_status: Some(ticket_type2.status.to_string()),
            event_name: Some(event2.name.clone()),
            ticket_pricing_name: Some(ticket_pricing2.name.clone()),
            ticket_pricing_price_in_cents: Some(100),
            box_office_order_count: 0,
            online_order_count: 1,
            online_sales_in_cents: 200,
            online_face_sales_in_cents: 200,
            online_sale_count: 2,
            online_fee_count: 2,
            total_online_fees_in_cents: 40,
            company_online_fees_in_cents: 16,
            client_online_fees_in_cents: 24,
            user_count: 1,
            ..Default::default()
        })
    );

    assert_eq!(
        result.sales.iter().find(|s| s.promo_redemption_code == None
            && s.hold_id == None
            && s.ticket_type_id == None
            && s.event_id == Some(event.id)),
        Some(&TicketSalesRow {
            organization_id: Some(event.organization_id),
            event_id: Some(event.id),
            per_order_company_online_fees: 500,
            per_order_client_online_fees: 750,
            per_order_total_fees_in_cents: 1250,
            ..Default::default()
        })
    );

    assert_eq!(
        result.sales.iter().find(|s| s.promo_redemption_code == None
            && s.hold_id == None
            && s.ticket_type_id == None
            && s.event_id == Some(event2.id)),
        Some(&TicketSalesRow {
            organization_id: Some(event2.organization_id),
            event_id: Some(event2.id),
            per_order_company_online_fees: 100,
            per_order_client_online_fees: 150,
            per_order_total_fees_in_cents: 250,
            ..Default::default()
        })
    );
}

#[test]
fn transaction_detail_report() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().with_event_fee().with_fees().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_name("Event1".to_string())
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let ticket_type = event.ticket_types(true, None, connection).unwrap().remove(0);
    let event2 = project
        .create_event()
        .with_organization(&organization)
        .with_name("Event2".to_string())
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let ticket_type2 = event2.ticket_types(true, None, connection).unwrap().remove(0);
    let organization2 = project.create_organization().with_fees().finish();
    let event3 = project
        .create_event()
        .with_organization(&organization2)
        .with_name("Event3".to_string())
        .with_tickets()
        .with_ticket_pricing()
        .finish();

    let fee_schedule = FeeSchedule::find(organization.fee_schedule_id, connection).unwrap();
    let ticket_pricing = ticket_type.current_ticket_pricing(false, connection).unwrap();
    let fee_schedule_range = fee_schedule
        .get_range(ticket_pricing.price_in_cents, connection)
        .unwrap();
    let ticket_pricing2 = ticket_type2.current_ticket_pricing(false, connection).unwrap();
    let fee_schedule_range2 = fee_schedule
        .get_range(ticket_pricing2.price_in_cents, connection)
        .unwrap();

    let user = project.create_user().with_first_name("Bob".into()).finish();
    let user2 = project.create_user().with_first_name("Bobby".into()).finish();
    let user3 = project
        .create_user()
        .with_first_name("Dan".into())
        .with_last_name("Bob".into())
        .finish();
    let user4 = project
        .create_user()
        .with_first_name("Dan".into())
        .with_last_name("Smith".into())
        .finish();

    let mut order = project
        .create_order()
        .quantity(2)
        .for_event(&event)
        .for_user(&user)
        .is_paid()
        .finish();
    let order_paid_at = Utc::now().naive_utc() - Duration::days(5);
    order = diesel::update(orders::table.filter(orders::id.eq(order.id)))
        .set(orders::paid_at.eq(order_paid_at))
        .get_result::<Order>(connection)
        .unwrap();

    let mut order2 = project
        .create_order()
        .quantity(2)
        .for_event(&event)
        .for_user(&user2)
        .is_paid()
        .finish();
    let order2_paid_at = Utc::now().naive_utc() - Duration::days(4);
    order2 = diesel::update(orders::table.filter(orders::id.eq(order2.id)))
        .set(orders::paid_at.eq(order2_paid_at))
        .get_result::<Order>(connection)
        .unwrap();

    let mut order3 = project
        .create_order()
        .quantity(2)
        .for_event(&event2)
        .for_user(&user3)
        .is_paid()
        .finish();
    let order3_paid_at = Utc::now().naive_utc() - Duration::days(3);
    order3 = diesel::update(orders::table.filter(orders::id.eq(order3.id)))
        .set(orders::paid_at.eq(order3_paid_at))
        .get_result::<Order>(connection)
        .unwrap();

    let mut order4 = project
        .create_order()
        .quantity(2)
        .for_event(&event)
        .for_user(&user4)
        .is_paid()
        .finish();
    let order4_paid_at = Utc::now().naive_utc() - Duration::days(2);
    order4 = diesel::update(orders::table.filter(orders::id.eq(order4.id)))
        .set(orders::paid_at.eq(order4_paid_at))
        .get_result::<Order>(connection)
        .unwrap();

    let _order5 = project
        .create_order()
        .quantity(2)
        .for_event(&event3)
        .for_user(&user)
        .is_paid()
        .finish();
    let _order6 = project
        .create_order()
        .quantity(2)
        .for_event(&event3)
        .for_user(&user3)
        .is_paid()
        .finish();

    // No query, for event
    let result = Report::transaction_detail_report(None, Some(event.id), None, None, None, 0, 100, connection).unwrap();
    let expected_results = vec![
        build_transaction_report_row(
            3,
            &organization,
            &user4,
            &order4,
            &event,
            &ticket_type,
            &fee_schedule_range,
            2,
            ticket_pricing.price_in_cents,
        ),
        build_transaction_report_row(
            3,
            &organization,
            &user2,
            &order2,
            &event,
            &ticket_type,
            &fee_schedule_range,
            2,
            ticket_pricing.price_in_cents,
        ),
        build_transaction_report_row(
            3,
            &organization,
            &user,
            &order,
            &event,
            &ticket_type,
            &fee_schedule_range,
            2,
            ticket_pricing.price_in_cents,
        ),
    ];
    assert_eq!(result.data, expected_results);
    assert_eq!(result.paging.total, 3);

    // No query, for organization
    let result =
        Report::transaction_detail_report(None, None, Some(organization.id), None, None, 0, 100, connection).unwrap();
    let expected_results = vec![
        build_transaction_report_row(
            4,
            &organization,
            &user4,
            &order4,
            &event,
            &ticket_type,
            &fee_schedule_range,
            2,
            ticket_pricing.price_in_cents,
        ),
        build_transaction_report_row(
            4,
            &organization,
            &user3,
            &order3,
            &event2,
            &ticket_type2,
            &fee_schedule_range2,
            2,
            ticket_pricing2.price_in_cents,
        ),
        build_transaction_report_row(
            4,
            &organization,
            &user2,
            &order2,
            &event,
            &ticket_type,
            &fee_schedule_range,
            2,
            ticket_pricing.price_in_cents,
        ),
        build_transaction_report_row(
            4,
            &organization,
            &user,
            &order,
            &event,
            &ticket_type,
            &fee_schedule_range,
            2,
            ticket_pricing.price_in_cents,
        ),
    ];
    assert_eq!(result.data, expected_results);
    assert_eq!(result.paging.total, 4);

    // With query, for organization (query finds user's name)
    let query = "Bob".to_string();
    let result =
        Report::transaction_detail_report(Some(query), None, Some(organization.id), None, None, 0, 100, connection)
            .unwrap();
    let expected_results = vec![
        build_transaction_report_row(
            3,
            &organization,
            &user3,
            &order3,
            &event2,
            &ticket_type2,
            &fee_schedule_range2,
            2,
            ticket_pricing2.price_in_cents,
        ),
        build_transaction_report_row(
            3,
            &organization,
            &user2,
            &order2,
            &event,
            &ticket_type,
            &fee_schedule_range,
            2,
            ticket_pricing.price_in_cents,
        ),
        build_transaction_report_row(
            3,
            &organization,
            &user,
            &order,
            &event,
            &ticket_type,
            &fee_schedule_range,
            2,
            ticket_pricing.price_in_cents,
        ),
    ];
    assert_eq!(result.data, expected_results);
    assert_eq!(result.paging.total, 3);

    // With query, for organization (query finds user's email)
    let query = user.email.clone();
    let result =
        Report::transaction_detail_report(query, None, Some(organization.id), None, None, 0, 100, connection).unwrap();
    let expected_results = vec![build_transaction_report_row(
        1,
        &organization,
        &user,
        &order,
        &event,
        &ticket_type,
        &fee_schedule_range,
        2,
        ticket_pricing.price_in_cents,
    )];
    assert_eq!(result.data, expected_results);
    assert_eq!(result.paging.total, 1);

    // With query, for organization (query finds order number)
    let query = order2.order_number();
    let result =
        Report::transaction_detail_report(Some(query), None, Some(organization.id), None, None, 0, 100, connection)
            .unwrap();
    let expected_results = vec![build_transaction_report_row(
        1,
        &organization,
        &user2,
        &order2,
        &event,
        &ticket_type,
        &fee_schedule_range,
        2,
        ticket_pricing.price_in_cents,
    )];
    assert_eq!(result.data, expected_results);
    assert_eq!(result.paging.total, 1);

    // With query, for organization (query finds event name)
    let query = "Event2".to_string();
    let result =
        Report::transaction_detail_report(Some(query), None, Some(organization.id), None, None, 0, 100, connection)
            .unwrap();
    let expected_results = vec![build_transaction_report_row(
        1,
        &organization,
        &user3,
        &order3,
        &event2,
        &ticket_type2,
        &fee_schedule_range2,
        2,
        ticket_pricing2.price_in_cents,
    )];
    assert_eq!(result.data, expected_results);
    assert_eq!(result.paging.total, 1);

    // With pagination
    let result =
        Report::transaction_detail_report(None, None, Some(organization.id), None, None, 0, 1, connection).unwrap();
    let expected_results = vec![build_transaction_report_row(
        4,
        &organization,
        &user4,
        &order4,
        &event,
        &ticket_type,
        &fee_schedule_range,
        2,
        ticket_pricing.price_in_cents,
    )];
    assert_eq!(result.data, expected_results);
    assert_eq!(result.paging.total, 4);

    // No query, for organization with time range
    let start = Utc::now().naive_utc() - Duration::days(4) - Duration::seconds(20);
    let end = Utc::now().naive_utc() - Duration::days(2) + Duration::seconds(20);
    let result = Report::transaction_detail_report(
        None,
        None,
        Some(organization.id),
        Some(start),
        Some(end),
        0,
        100,
        connection,
    )
    .unwrap();
    let expected_results = vec![
        build_transaction_report_row(
            3,
            &organization,
            &user4,
            &order4,
            &event,
            &ticket_type,
            &fee_schedule_range,
            2,
            ticket_pricing.price_in_cents,
        ),
        build_transaction_report_row(
            3,
            &organization,
            &user3,
            &order3,
            &event2,
            &ticket_type2,
            &fee_schedule_range2,
            2,
            ticket_pricing2.price_in_cents,
        ),
        build_transaction_report_row(
            3,
            &organization,
            &user2,
            &order2,
            &event,
            &ticket_type,
            &fee_schedule_range,
            2,
            ticket_pricing.price_in_cents,
        ),
    ];
    assert_eq!(result.data, expected_results);
    assert_eq!(result.paging.total, 3);
}

#[test]
fn box_office_sales_summary_report() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let box_office_user = project.create_user().with_first_name("BoxOfficeUser1").finish();
    let box_office_user2 = project.create_user().with_first_name("BoxOfficeUser2").finish();
    let user = project.create_user().finish();
    let user2 = project.create_user().finish();
    let user3 = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&box_office_user, Roles::OrgBoxOffice)
        .with_member(&box_office_user2, Roles::OrgBoxOffice)
        .with_fees()
        .with_event_fee()
        .finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_event_start(NaiveDate::from_ymd(2016, 7, 8).and_hms(9, 10, 11))
        .with_name("Event1".to_string())
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let event2 = project
        .create_event()
        .with_organization(&organization)
        .with_event_start(NaiveDate::from_ymd(2017, 7, 8).and_hms(9, 10, 11))
        .with_name("Event2".to_string())
        .with_tickets()
        .with_ticket_pricing()
        .finish();

    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    let hold = project
        .create_hold()
        .with_hold_type(HoldTypes::Discount)
        .with_quantity(10)
        .with_ticket_type_id(ticket_type.id)
        .finish();
    let mut box_office_user_orders = Vec::new();
    let mut box_office_user2_orders = Vec::new();
    box_office_user_orders.push(
        project
            .create_order()
            .quantity(2)
            .for_event(&event)
            .for_user(&box_office_user)
            .on_behalf_of_user(&user)
            .is_paid()
            .with_external_payment_type(ExternalPaymentType::Voucher)
            .finish(),
    );
    box_office_user_orders.push(
        project
            .create_order()
            .quantity(1)
            .for_event(&event2)
            .for_user(&box_office_user)
            .on_behalf_of_user(&user)
            .is_paid()
            .with_external_payment_type(ExternalPaymentType::CreditCard)
            .finish(),
    );
    box_office_user_orders.push(
        project
            .create_order()
            .quantity(1)
            .for_event(&event2)
            .for_user(&box_office_user)
            .on_behalf_of_user(&user3)
            .is_paid()
            .with_external_payment_type(ExternalPaymentType::Cash)
            .finish(),
    );
    box_office_user2_orders.push(
        project
            .create_order()
            .quantity(2)
            .for_event(&event)
            .for_user(&box_office_user2)
            .on_behalf_of_user(&user2)
            .is_paid()
            .with_external_payment_type(ExternalPaymentType::Cash)
            .finish(),
    );
    box_office_user2_orders.push(
        project
            .create_order()
            .quantity(1)
            .for_event(&event)
            .for_user(&box_office_user2)
            .on_behalf_of_user(&user2)
            .is_paid()
            .with_redemption_code(hold.redemption_code.clone().unwrap())
            .with_external_payment_type(ExternalPaymentType::Cash)
            .finish(),
    );

    let expected_report_data = BoxOfficeSalesSummaryReport {
        operators: vec![
            BoxOfficeSalesSummaryOperatorRow {
                operator_id: box_office_user.id,
                operator_name: box_office_user.full_name(),
                events: vec![
                    BoxOfficeSalesSummaryOperatorEventRow {
                        event_name: Some("Event1".to_string()),
                        event_date: Some(NaiveDate::from_ymd(2016, 7, 8).and_hms(9, 10, 11)),
                        number_of_tickets: 2,
                        face_value_in_cents: 150,
                        revenue_share_value_in_cents: 0,
                        total_sales_in_cents: 300,
                    },
                    BoxOfficeSalesSummaryOperatorEventRow {
                        event_name: Some("Event2".to_string()),
                        event_date: Some(NaiveDate::from_ymd(2017, 7, 8).and_hms(9, 10, 11)),
                        number_of_tickets: 2,
                        face_value_in_cents: 150,
                        revenue_share_value_in_cents: 0,
                        total_sales_in_cents: 300,
                    },
                ],
                payments: vec![
                    BoxOfficeSalesSummaryPaymentRow {
                        payment_type: ExternalPaymentType::Cash,
                        quantity: 1,
                        total_sales_in_cents: 150,
                    },
                    BoxOfficeSalesSummaryPaymentRow {
                        payment_type: ExternalPaymentType::CreditCard,
                        quantity: 1,
                        total_sales_in_cents: 150,
                    },
                    BoxOfficeSalesSummaryPaymentRow {
                        payment_type: ExternalPaymentType::Voucher,
                        quantity: 2,
                        total_sales_in_cents: 300,
                    },
                ],
            },
            BoxOfficeSalesSummaryOperatorRow {
                operator_id: box_office_user2.id,
                operator_name: box_office_user2.full_name(),
                events: vec![
                    BoxOfficeSalesSummaryOperatorEventRow {
                        event_name: Some("Event1".to_string()),
                        event_date: Some(NaiveDate::from_ymd(2016, 7, 8).and_hms(9, 10, 11)),
                        number_of_tickets: 1,
                        face_value_in_cents: 140,
                        revenue_share_value_in_cents: 0,
                        total_sales_in_cents: 140,
                    },
                    BoxOfficeSalesSummaryOperatorEventRow {
                        event_name: Some("Event1".to_string()),
                        event_date: Some(NaiveDate::from_ymd(2016, 7, 8).and_hms(9, 10, 11)),
                        number_of_tickets: 2,
                        face_value_in_cents: 150,
                        revenue_share_value_in_cents: 0,
                        total_sales_in_cents: 300,
                    },
                ],
                payments: vec![BoxOfficeSalesSummaryPaymentRow {
                    payment_type: ExternalPaymentType::Cash,
                    quantity: 3,
                    total_sales_in_cents: 440,
                }],
            },
        ],
        payments: vec![
            BoxOfficeSalesSummaryPaymentRow {
                payment_type: ExternalPaymentType::Cash,
                quantity: 4,
                total_sales_in_cents: 590,
            },
            BoxOfficeSalesSummaryPaymentRow {
                payment_type: ExternalPaymentType::CreditCard,
                quantity: 1,
                total_sales_in_cents: 150,
            },
            BoxOfficeSalesSummaryPaymentRow {
                payment_type: ExternalPaymentType::Voucher,
                quantity: 2,
                total_sales_in_cents: 300,
            },
        ],
    };

    let report_data = Report::box_office_sales_summary_report(organization.id, None, None, connection).unwrap();
    assert_eq!(expected_report_data, report_data);
}

fn build_transaction_report_row(
    total: i64,
    organization: &Organization,
    user: &User,
    order: &Order,
    event: &Event,
    ticket_type: &TicketType,
    fee_schedule_range: &FeeScheduleRange,
    quantity: i64,
    price_per_ticket: i64,
) -> TransactionReportRow {
    TransactionReportRow {
        total,
        quantity,
        event_name: event.name.clone(),
        ticket_name: ticket_type.name.clone(),
        actual_quantity: quantity,
        refunded_quantity: 0,
        unit_price_in_cents: price_per_ticket,
        face_price_in_cents: price_per_ticket,
        face_price_in_cents_total: price_per_ticket * quantity,
        gross: price_per_ticket * quantity + fee_schedule_range.client_fee_in_cents * quantity,
        client_fee_in_cents: fee_schedule_range.client_fee_in_cents,
        client_fee_in_cents_total: fee_schedule_range.client_fee_in_cents * quantity,
        event_fee_client_in_cents: organization.client_event_fee_in_cents,
        event_fee_client_in_cents_total: organization.client_event_fee_in_cents,
        fee_range_id: Some(fee_schedule_range.id),
        order_type: OrderTypes::Cart,
        payment_method: Some(PaymentMethods::CreditCard.to_string()),
        payment_provider: Some(PaymentProviders::Stripe.to_string()),
        transaction_date: order.paid_at.clone().unwrap(),
        redemption_code: None,
        order_id: order.id,
        event_id: event.id,
        user_id: user.id,
        first_name: user.first_name.clone().unwrap(),
        last_name: user.last_name.clone().unwrap(),
        email: user.email.clone().unwrap(),
        event_start: event.event_start,
        promo_discount_value_in_cents: 0,
        promo_quantity: 0,
        promo_code_name: None,
        promo_redemption_code: None,
        source: None,
        medium: None,
        campaign: None,
        term: None,
        content: None,
        platform: None,
        check_in_source: None,
    }
}

#[test]
fn promo_code_report() {
    let project = TestProject::new();
    let connection = project.get_connection();

    let creator = project.create_user().finish();

    let organization = project.create_organization().with_event_fee().with_fees().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_name("Event1".to_string())
        .with_ticket_type()
        .with_name("GA".to_string())
        .with_price(1000)
        .finish();

    let wallet_id = event.issuer_wallet(connection).unwrap().id;

    event
        .add_ticket_type(
            "VIP".to_string(),
            None,
            100,
            Some(dates::now().add_days(-1).finish()),
            None,
            TicketTypeEndDateType::EventEnd,
            Some(wallet_id),
            None,
            0,
            2000,
            TicketTypeVisibility::Always,
            None,
            0,
            true,
            true,
            true,
            None,
            connection,
        )
        .unwrap();

    let ticket_types = event.ticket_types(false, None, connection).unwrap();

    let ticket_type = &ticket_types[0];
    let ticket_type2 = &ticket_types[1];
    let code = project
        .create_code()
        .with_name("Discount 1".into())
        .with_event(&event)
        .with_discount_in_cents(Some(100))
        .for_ticket_type(ticket_type)
        .for_ticket_type(ticket_type2)
        .with_code_type(CodeTypes::Discount)
        .finish();

    let code2 = project
        .create_code()
        .with_name("Discount 2".into())
        .with_event(&event)
        .with_discount_in_cents(Some(300))
        .for_ticket_type(ticket_type)
        .with_code_type(CodeTypes::Discount)
        .finish();

    //Buy some ticket with no codes
    let mut cart = Order::find_or_create_cart(&creator, connection).unwrap();

    cart.update_quantities(
        creator.id,
        &[
            UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 1,
                redemption_code: None,
            },
            UpdateOrderItem {
                ticket_type_id: ticket_type2.id,
                quantity: 1,
                redemption_code: None,
            },
        ],
        true,
        false,
        connection,
    )
    .unwrap();

    let total1 = cart.calculate_total(connection).unwrap();
    cart.add_external_payment(
        Some("Test".to_string()),
        ExternalPaymentType::CreditCard,
        creator.id,
        total1,
        connection,
    )
    .unwrap();

    //Buy some tickets with discounts
    //Buy some ticket with no codes
    let mut cart = Order::find_or_create_cart(&creator, connection).unwrap();

    cart.update_quantities(
        creator.id,
        &[
            UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 1,
                redemption_code: Some(code.redemption_code.clone()),
            },
            UpdateOrderItem {
                ticket_type_id: ticket_type2.id,
                quantity: 1,
                redemption_code: Some(code.redemption_code.clone()),
            },
        ],
        true,
        false,
        connection,
    )
    .unwrap();

    let total2 = cart.calculate_total(connection).unwrap();
    assert_eq!(total2, total1 - 200);
    cart.add_external_payment(
        Some("Test".to_string()),
        ExternalPaymentType::CreditCard,
        creator.id,
        total2,
        connection,
    )
    .unwrap();

    //Buy some more ticket with the second code

    let mut cart = Order::find_or_create_cart(&creator, connection).unwrap();

    cart.update_quantities(
        creator.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 10,
            redemption_code: Some(code2.redemption_code.clone()),
        }],
        true,
        false,
        connection,
    )
    .unwrap();

    let total3 = cart.calculate_total(connection).unwrap();
    cart.add_external_payment(
        Some("Test".to_string()),
        ExternalPaymentType::CreditCard,
        creator.id,
        total3,
        connection,
    )
    .unwrap();

    //Check report
    let report = Report::promo_code_report(Some(event.id), Some(organization.id), connection).unwrap();

    //The order of the rows coming back is not consistent
    let mut test_pass_count = 0;
    for row in report {
        if row.hold_name.is_none() && row.ticket_name == Some("GA".to_string()) {
            test_pass_count += (row.box_office_sales_in_cents == 1000) as i32;
        }

        if row.hold_name.is_none() && row.ticket_name == Some("VIP".to_string()) {
            test_pass_count += (row.box_office_sales_in_cents == 2000) as i32;
        }

        if row.hold_name == Some("Discount 1".to_string()) && row.ticket_name == Some("GA".to_string()) {
            test_pass_count += (row.box_office_sales_in_cents == 900) as i32;
        }

        if row.hold_name == Some("Discount 1".to_string()) && row.ticket_name == Some("VIP".to_string()) {
            test_pass_count += (row.box_office_sales_in_cents == 1900) as i32;
        }

        if row.hold_name == Some("Discount 2".to_string()) && row.ticket_name == Some("GA".to_string()) {
            test_pass_count += (row.box_office_sales_in_cents == 7000) as i32;
        }
    }

    assert_eq!(test_pass_count, 5);
}
