use chrono::prelude::*;
use db::dev::{times, TestProject};
use db::prelude::*;
use db::schema::ticket_instances;
use db::utils::errors::ErrorCode::ValidationError;
use diesel::prelude::*;
use itertools::Itertools;
use uuid::Uuid;

#[test]
fn status() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project
        .create_event()
        .with_a_specific_number_of_tickets(10)
        .with_ticket_type_count(1)
        .finish();
    // Ticket type created without pricing leading to no active pricing status
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    ticket_type
        .current_ticket_pricing(false, connection)
        .unwrap()
        .destroy(None, connection)
        .unwrap();
    assert_eq!(
        ticket_type.status(false, connection).unwrap(),
        TicketTypeStatus::NoActivePricing
    );

    // Sales ended as all pricing periods have ended (only this one exists at this time which is in the past)
    let ticket_pricing = ticket_type
        .add_ticket_pricing(
            "Ticket Pricing".to_string(),
            dates::now().add_days(-2).finish(),
            dates::now().add_days(-1).finish(),
            10,
            false,
            None,
            None,
            connection,
        )
        .unwrap();
    assert_eq!(
        ticket_type.status(false, connection).unwrap(),
        TicketTypeStatus::SaleEnded
    );
    ticket_pricing.destroy(None, connection).unwrap();

    // On sale soon since pricing won't begin for a day
    ticket_type
        .add_ticket_pricing(
            "Ticket Pricing".to_string(),
            dates::now().add_days(1).finish(),
            dates::now().add_days(2).finish(),
            10,
            false,
            None,
            None,
            connection,
        )
        .unwrap();
    assert_eq!(
        ticket_type.status(false, connection).unwrap(),
        TicketTypeStatus::OnSaleSoon
    );

    // Current pricing that's active shows published
    ticket_type
        .add_ticket_pricing(
            "Ticket Pricing".to_string(),
            dates::now().add_days(-1).finish(),
            dates::now().add_days(1).finish(),
            10,
            false,
            None,
            None,
            connection,
        )
        .unwrap();
    assert_eq!(
        ticket_type.status(false, connection).unwrap(),
        TicketTypeStatus::Published
    );

    // Use up all but 1 quantity, no effect
    project.create_order().for_event(&event).quantity(9).is_paid().finish();
    assert_eq!(
        ticket_type.status(false, connection).unwrap(),
        TicketTypeStatus::Published
    );

    // Last item taken, sold out
    project.create_order().for_event(&event).quantity(1).is_paid().finish();
    assert_eq!(
        ticket_type.status(false, connection).unwrap(),
        TicketTypeStatus::SoldOut
    );
}

#[test]
fn end_date() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let door_time = NaiveDate::from_ymd(2016, 7, 9).and_hms(5, 10, 11);
    let event_start = NaiveDate::from_ymd(2016, 7, 9).and_hms(6, 10, 11);
    let event_end = NaiveDate::from_ymd(2016, 7, 9).and_hms(9, 10, 11);
    let event = project
        .create_event()
        .with_door_time(door_time)
        .with_event_start(event_start)
        .with_event_end(event_end)
        .with_tickets()
        .finish();
    let ticket_type = event.ticket_types(true, None, connection).unwrap().remove(0);

    // Manual
    let end_date = NaiveDate::from_ymd(2016, 7, 9).and_hms(4, 10, 11);
    let ticket_type = ticket_type
        .update(
            TicketTypeEditableAttributes {
                end_date: Some(Some(end_date)),
                end_date_type: Some(TicketTypeEndDateType::Manual),
                ..Default::default()
            },
            None,
            project.get_connection(),
        )
        .unwrap();
    assert_eq!(ticket_type.end_date(connection), Ok(end_date));
    let ticket_pricing = TicketPricing::get_default(ticket_type.id, connection).unwrap();
    assert_eq!(ticket_pricing.end_date, end_date);

    // Event Start
    let ticket_type = ticket_type
        .update(
            TicketTypeEditableAttributes {
                end_date: Some(None),
                end_date_type: Some(TicketTypeEndDateType::EventStart),
                ..Default::default()
            },
            None,
            project.get_connection(),
        )
        .unwrap();
    assert_eq!(ticket_type.end_date(connection), Ok(event_start));
    let ticket_pricing = TicketPricing::get_default(ticket_type.id, connection).unwrap();
    assert_eq!(ticket_pricing.end_date, event_start);

    // Door Time
    let ticket_type = ticket_type
        .update(
            TicketTypeEditableAttributes {
                end_date: Some(None),
                end_date_type: Some(TicketTypeEndDateType::DoorTime),
                ..Default::default()
            },
            None,
            project.get_connection(),
        )
        .unwrap();
    assert_eq!(ticket_type.end_date(connection), Ok(door_time));
    let ticket_pricing = TicketPricing::get_default(ticket_type.id, connection).unwrap();
    assert_eq!(ticket_pricing.end_date, door_time);

    // Event End
    let ticket_type = ticket_type
        .update(
            TicketTypeEditableAttributes {
                end_date: Some(None),
                end_date_type: Some(TicketTypeEndDateType::EventEnd),
                ..Default::default()
            },
            None,
            project.get_connection(),
        )
        .unwrap();
    assert_eq!(ticket_type.end_date(connection), Ok(event_end));
    let ticket_pricing = TicketPricing::get_default(ticket_type.id, connection).unwrap();
    assert_eq!(ticket_pricing.end_date, event_end);
}

#[test]
fn new_record_end_date() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let door_time = NaiveDate::from_ymd(2016, 7, 9).and_hms(5, 10, 11);
    let event_start = NaiveDate::from_ymd(2016, 7, 9).and_hms(6, 10, 11);
    let event_end = NaiveDate::from_ymd(2016, 7, 9).and_hms(9, 10, 11);
    let event = project
        .create_event()
        .with_door_time(door_time)
        .with_event_start(event_start)
        .with_event_end(event_end)
        .finish();

    // Manual
    let end_date = NaiveDate::from_ymd(2016, 7, 9).and_hms(4, 10, 11);
    let mut new_ticket_type = NewTicketType {
        event_id: event.id,
        name: "TicketType Name".to_string(),
        description: None,
        status: TicketTypeStatus::Published,
        start_date: None,
        end_date: Some(end_date),
        increment: None,
        limit_per_person: 0,
        price_in_cents: 0,
        visibility: TicketTypeVisibility::Always,
        parent_id: None,
        additional_fee_in_cents: 0,
        end_date_type: TicketTypeEndDateType::Manual,
        app_sales_enabled: true,
        web_sales_enabled: true,
        box_office_sales_enabled: true,
    };
    assert_eq!(new_ticket_type.end_date(connection), Ok(end_date));

    // Event Start
    new_ticket_type.end_date = None;
    new_ticket_type.end_date_type = TicketTypeEndDateType::EventStart;
    assert_eq!(new_ticket_type.end_date(connection), Ok(event_start));

    // Door Time
    new_ticket_type.end_date = None;
    new_ticket_type.end_date_type = TicketTypeEndDateType::DoorTime;
    assert_eq!(new_ticket_type.end_date(connection), Ok(door_time));

    // Event End
    new_ticket_type.end_date = None;
    new_ticket_type.end_date_type = TicketTypeEndDateType::EventEnd;
    assert_eq!(new_ticket_type.end_date(connection), Ok(event_end));
}

#[test]
fn create() {
    let project = TestProject::new();
    let conn = project.get_connection();
    let event = project.create_event().finish();
    let wallet_id = event.issuer_wallet(conn).unwrap().id;
    let sd = NaiveDate::from_ymd(2016, 7, 8).and_hms(4, 10, 11);
    let ed = NaiveDate::from_ymd(2016, 7, 9).and_hms(4, 10, 11);
    let ticket_type = event
        .add_ticket_type(
            "VIP".to_string(),
            None,
            100,
            Some(sd),
            Some(ed),
            TicketTypeEndDateType::Manual,
            Some(wallet_id),
            None,
            0,
            100,
            TicketTypeVisibility::Always,
            None,
            0,
            true,
            true,
            true,
            None,
            conn,
        )
        .unwrap();

    assert_eq!(ticket_type.event_id, event.id);
    assert_eq!(ticket_type.name, "VIP".to_string())
}

#[test]
pub fn create_with_validation_errors() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project.create_event().finish();
    let wallet_id = event.issuer_wallet(connection).unwrap().id;
    let start_date = NaiveDate::from_ymd(2016, 7, 8).and_hms(4, 10, 11);
    let end_date = NaiveDate::from_ymd(2016, 6, 9).and_hms(4, 10, 11);
    let result = event.add_ticket_type(
        "VIP".to_string(),
        None,
        100,
        Some(start_date),
        Some(end_date),
        TicketTypeEndDateType::Manual,
        Some(wallet_id),
        None,
        0,
        100,
        TicketTypeVisibility::Always,
        None,
        0,
        true,
        true,
        true,
        None,
        connection,
    );
    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("start_date"));
                assert_eq!(errors["start_date"].len(), 1);
                assert_eq!(errors["start_date"][0].code, "start_date_must_be_before_end_date");
                assert_eq!(
                    &errors["start_date"][0].message.clone().unwrap().into_owned(),
                    "Start date must be before end date"
                );
            }
            _ => panic!("Expected validation error"),
        },
    }

    // No end date provided
    let result = event.add_ticket_type(
        "VIP".to_string(),
        None,
        100,
        Some(start_date),
        None,
        TicketTypeEndDateType::Manual,
        Some(wallet_id),
        None,
        0,
        100,
        TicketTypeVisibility::Always,
        None,
        0,
        true,
        true,
        true,
        None,
        connection,
    );
    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("end_date"));
                assert_eq!(errors["end_date"].len(), 1);
                assert_eq!(errors["end_date"][0].code, "required");
                assert_eq!(
                    &errors["end_date"][0].message.clone().unwrap().into_owned(),
                    "End date required for manual end date type"
                );
            }
            _ => panic!("Expected validation error"),
        },
    }
}

#[test]
fn find_by_ids() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project
        .create_event()
        .with_ticket_pricing()
        .with_ticket_type_count(2)
        .finish();
    let mut ticket_types = event.ticket_types(true, None, connection).unwrap();
    let mut found_ticket_types =
        TicketType::find_by_ids(&ticket_types.iter().map(|tt| tt.id).collect(), connection).unwrap();
    ticket_types.sort_by_key(|tt| tt.id);
    found_ticket_types.sort_by_key(|tt| tt.id);
    assert_eq!(ticket_types, found_ticket_types);
}

#[test]
fn valid_unsold_ticket_count() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project.create_event().with_ticket_pricing().finish();
    let ticket_type = event.ticket_types(true, None, connection).unwrap().remove(0);
    assert_eq!(100, ticket_type.valid_unsold_ticket_count(connection).unwrap());

    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    cart.update_quantities(
        user.id,
        &vec![UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 50,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();

    // 50 in cart
    assert_eq!(100, ticket_type.valid_unsold_ticket_count(connection).unwrap());

    let total = cart.calculate_total(connection).unwrap();
    cart.add_external_payment(
        Some("test".to_string()),
        ExternalPaymentType::CreditCard,
        user.id,
        total,
        connection,
    )
    .unwrap();

    // 50 paid
    assert_eq!(50, ticket_type.valid_unsold_ticket_count(connection).unwrap());

    // Add 1 to cart marking it as reserved
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    cart.update_quantities(
        user.id,
        &vec![UpdateOrderItem {
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
    let ticket_instance = ticket_instances::table
        .filter(ticket_instances::order_item_id.eq(order_item.id))
        .first::<TicketInstance>(connection)
        .unwrap();
    assert_eq!(TicketInstanceStatus::Reserved, ticket_instance.status);

    // 0 Nullified events
    let domain_events = DomainEvent::find(
        Tables::TicketInstances,
        None,
        Some(DomainEventTypes::TicketInstanceNullified),
        connection,
    )
    .unwrap();
    assert_eq!(0, domain_events.len());

    // Nullify 49
    let asset = Asset::find_by_ticket_type(ticket_type.id, connection).unwrap();
    TicketInstance::nullify_tickets(asset.id, 49, user.id, connection).unwrap();
    assert_eq!(1, ticket_type.valid_unsold_ticket_count(connection).unwrap());

    // Reload ticket instance, should not nullify
    let ticket_instance = TicketInstance::find(ticket_instance.id, connection).unwrap();
    assert_eq!(TicketInstanceStatus::Reserved, ticket_instance.status);

    // 0 Nullified events for this particular ID
    let domain_events = DomainEvent::find(
        Tables::TicketInstances,
        Some(ticket_instance.id),
        Some(DomainEventTypes::TicketInstanceNullified),
        connection,
    )
    .unwrap();
    assert_eq!(0, domain_events.len());

    // Nullify remaining 1
    TicketInstance::nullify_tickets(asset.id, 1, user.id, connection).unwrap();
    assert_eq!(0, ticket_type.valid_unsold_ticket_count(connection).unwrap());

    // Reload ticket instance, should nullify
    let ticket_instance = TicketInstance::find(ticket_instance.id, connection).unwrap();
    assert_eq!(TicketInstanceStatus::Nullified, ticket_instance.status);

    // 1 Nullified events for this particular ID
    let domain_events = DomainEvent::find(
        Tables::TicketInstances,
        Some(ticket_instance.id),
        Some(DomainEventTypes::TicketInstanceNullified),
        connection,
    )
    .unwrap();
    assert_eq!(1, domain_events.len());
}

#[test]
fn find_for_code() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project
        .create_event()
        .with_ticket_pricing()
        .with_ticket_type_count(2)
        .finish();
    let ticket_types = event.ticket_types(true, None, &connection).unwrap();
    let ticket_type = &ticket_types[0];
    let ticket_type2 = &ticket_types[1];
    let code = project.create_code().with_event(&event).finish();

    TicketTypeCode::create(ticket_type.id, code.id)
        .commit(connection)
        .unwrap();
    TicketTypeCode::create(ticket_type2.id, code.id)
        .commit(connection)
        .unwrap();

    let found_ticket_types = TicketType::find_for_code(code.id, connection).unwrap();
    assert_eq!(
        found_ticket_types
            .into_iter()
            .map(|tt| tt.id)
            .collect::<Vec<Uuid>>()
            .sort(),
        vec![ticket_type.id, ticket_type2.id].sort()
    );
}

#[test]
fn find_by_event_id() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project
        .create_event()
        .with_ticket_pricing()
        .with_ticket_type_count(2)
        .finish();
    let mut ticket_types = event.ticket_types(true, None, &connection).unwrap();
    let ticket_type = ticket_types.remove(0);
    let ticket_type2 = ticket_types.remove(0);

    // No access codes on file checking with access code filtering
    let results = TicketType::find_by_event_id(event.id, true, None, &connection).unwrap();
    assert_eq!(vec![ticket_type.clone(), ticket_type2.clone()], results);

    // No access codes on file checking without access code filtering
    let mut results = TicketType::find_by_event_id(event.id, false, None, &connection).unwrap();
    results.sort_by_key(|r| r.id);
    let mut expected = vec![ticket_type.clone(), ticket_type2.clone()];
    expected.sort_by_key(|r| r.id);
    assert_eq!(expected, results);

    // Add access code
    let code = project
        .create_code()
        .with_event(&event)
        .for_ticket_type(&ticket_type)
        .with_code_type(CodeTypes::Access)
        .finish();

    // One ticket type is filtered while one is not so filtered only shows when code is present
    let results = TicketType::find_by_event_id(event.id, true, None, &connection).unwrap();
    assert_eq!(vec![ticket_type2.clone()], results);
    let results =
        TicketType::find_by_event_id(event.id, true, Some(code.redemption_code.clone()), &connection).unwrap();
    assert_eq!(vec![ticket_type.clone(), ticket_type2.clone()], results);
    let results = TicketType::find_by_event_id(event.id, false, None, &connection).unwrap();
    assert_eq!(vec![ticket_type.clone(), ticket_type2.clone()], results);

    // Add additional code for discount but it does not affect visibility
    let code2 = project
        .create_code()
        .with_event(&event)
        .for_ticket_type(&ticket_type2)
        .with_code_type(CodeTypes::Discount)
        .finish();

    // Behavior mimics previous logic in that no ticket types have been filtered for new code
    let results = TicketType::find_by_event_id(event.id, true, None, &connection).unwrap();
    assert_eq!(vec![ticket_type2.clone()], results);
    let results =
        TicketType::find_by_event_id(event.id, true, Some(code.redemption_code.clone()), &connection).unwrap();
    assert_eq!(vec![ticket_type.clone(), ticket_type2.clone()], results);
    let results =
        TicketType::find_by_event_id(event.id, true, Some(code2.redemption_code.clone()), &connection).unwrap();
    assert_eq!(vec![ticket_type2.clone()], results);
    let results = TicketType::find_by_event_id(event.id, false, None, &connection).unwrap();
    assert_eq!(vec![ticket_type.clone(), ticket_type2.clone()], results);

    // Add additional access code
    let code3 = project
        .create_code()
        .with_event(&event)
        .for_ticket_type(&ticket_type2)
        .with_code_type(CodeTypes::Access)
        .finish();

    // No ticket types present when filtering but they can show up with each redemption code or without filtering
    let results = TicketType::find_by_event_id(event.id, true, None, &connection).unwrap();
    assert!(results.is_empty());
    let results =
        TicketType::find_by_event_id(event.id, true, Some(code.redemption_code.clone()), &connection).unwrap();
    assert_eq!(vec![ticket_type.clone()], results);
    let results =
        TicketType::find_by_event_id(event.id, true, Some(code2.redemption_code.clone()), &connection).unwrap();
    assert!(results.is_empty());
    let results =
        TicketType::find_by_event_id(event.id, true, Some(code3.redemption_code.clone()), &connection).unwrap();
    assert_eq!(vec![ticket_type2.clone()], results);
    let results = TicketType::find_by_event_id(event.id, false, None, &connection).unwrap();
    assert_eq!(vec![ticket_type.clone(), ticket_type2.clone()], results);
}

#[test]
fn create_large_amount() {
    let project = TestProject::new();
    let conn = project.get_connection();
    let event = project.create_event().finish();
    let wallet_id = event.issuer_wallet(conn).unwrap().id;
    let sd = NaiveDate::from_ymd(2016, 7, 8).and_hms(4, 10, 11);
    let ed = NaiveDate::from_ymd(2016, 7, 9).and_hms(4, 10, 11);
    let ticket_type = event
        .add_ticket_type(
            "VIP".to_string(),
            None,
            100_000,
            Some(sd),
            Some(ed),
            TicketTypeEndDateType::Manual,
            Some(wallet_id),
            None,
            0,
            100,
            TicketTypeVisibility::Always,
            None,
            0,
            true,
            true,
            true,
            None,
            conn,
        )
        .unwrap();

    assert_eq!(ticket_type.event_id, event.id);
    assert_eq!(ticket_type.name, "VIP".to_string())
}

#[test]
fn validate_ticket_pricing() {
    let project = TestProject::new();
    let event = project.create_event().with_tickets().finish();
    let ticket_type = event
        .ticket_types(true, None, project.get_connection())
        .unwrap()
        .remove(0);

    // Set short window for validations to detect dates outside of ticket type window
    let ticket_type = ticket_type
        .update(
            TicketTypeEditableAttributes {
                start_date: Some(Some(NaiveDate::from_ymd(2016, 7, 8).and_hms(4, 10, 11))),
                end_date: Some(Some(NaiveDate::from_ymd(2016, 7, 9).and_hms(4, 10, 11))),
                ..Default::default()
            },
            None,
            project.get_connection(),
        )
        .unwrap();

    let start_date1 = NaiveDate::from_ymd(2016, 7, 6).and_hms(4, 10, 11);
    let end_date1 = NaiveDate::from_ymd(2016, 7, 10).and_hms(4, 10, 11);
    let start_date2 = NaiveDate::from_ymd(2016, 7, 7).and_hms(4, 10, 11);
    let end_date2 = NaiveDate::from_ymd(2016, 7, 11).and_hms(4, 10, 11);
    TicketPricing::create(
        ticket_type.id,
        "Early Bird".to_string(),
        start_date1,
        end_date1,
        100,
        false,
        None,
        None,
    )
    .commit(None, project.get_connection())
    .unwrap();
    let ticket_pricing = TicketPricing::create(
        ticket_type.id,
        "Regular".to_string(),
        start_date2,
        end_date2,
        100,
        false,
        None,
        None,
    )
    .commit(None, project.get_connection())
    .unwrap();
    let mut ticket_pricing_parameters: TicketPricingEditableAttributes = Default::default();

    // Overlapping period and overlapping ticket type window
    let validation_results = ticket_type.validate_ticket_pricing(project.get_connection());
    assert!(validation_results.is_err());
    let error = validation_results.unwrap_err();
    match &error.error_code {
        ErrorCode::ValidationError { errors } => {
            assert!(errors.contains_key("ticket_pricing"));
            assert_eq!(errors["ticket_pricing"].len(), 2);
            assert_eq!(errors["ticket_pricing"][0].code, "ticket_pricing_overlapping_periods");
            assert_eq!(
                &errors["ticket_pricing"][0].message.clone().unwrap().into_owned(),
                "Ticket pricing dates overlap another ticket pricing period"
            );

            assert!(errors.contains_key("ticket_pricing.start_date"));
            assert_eq!(errors["ticket_pricing.start_date"].len(), 2);
            assert_eq!(
                errors["ticket_pricing.start_date"][0].code,
                "ticket_pricing_overlapping_ticket_type_start_date"
            );
            assert_eq!(
                &errors["ticket_pricing.start_date"][0]
                    .message
                    .clone()
                    .unwrap()
                    .into_owned(),
                "Ticket pricing dates overlap ticket type start date"
            );

            assert!(errors.contains_key("ticket_pricing.end_date"));
            assert_eq!(errors["ticket_pricing.end_date"].len(), 2);
            assert_eq!(
                errors["ticket_pricing.end_date"][0].code,
                "ticket_pricing_overlapping_ticket_type_end_date"
            );
            assert_eq!(
                &errors["ticket_pricing.end_date"][0]
                    .message
                    .clone()
                    .unwrap()
                    .into_owned(),
                "Ticket pricing dates overlap ticket type end date"
            );
        }
        _ => panic!("Expected validation error"),
    }

    // Ticket type adjusted so ticket pricing inclusive of its dates
    let ticket_type = ticket_type
        .update(
            TicketTypeEditableAttributes {
                start_date: Some(Some(NaiveDate::from_ymd(2016, 6, 1).and_hms(4, 10, 11))),
                end_date: Some(Some(NaiveDate::from_ymd(2055, 7, 6).and_hms(4, 10, 11))),
                ..Default::default()
            },
            None,
            project.get_connection(),
        )
        .unwrap();

    // Period adjusted to not overlap (after existing record)
    ticket_pricing_parameters.start_date = Some(end_date1);
    ticket_pricing_parameters.end_date = Some(NaiveDate::from_ymd(2016, 7, 15).and_hms(4, 10, 11));
    ticket_pricing
        .update(ticket_pricing_parameters.clone(), None, project.get_connection())
        .unwrap();
    ticket_type.validate_ticket_pricing(project.get_connection()).unwrap();

    // Period adjusted to not overlap (prior to existing record)
    ticket_pricing_parameters.start_date = Some(NaiveDate::from_ymd(2016, 7, 4).and_hms(4, 10, 11));
    ticket_pricing_parameters.end_date = Some(start_date1);
    ticket_pricing
        .update(ticket_pricing_parameters.clone(), None, project.get_connection())
        .unwrap();

    ticket_type.validate_ticket_pricing(project.get_connection()).unwrap();
}

#[test]
pub fn remaining_ticket_count() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project.create_event().with_ticket_pricing().finish();
    let ticket_type = event.ticket_types(true, None, connection).unwrap().remove(0);
    let mut order = project.create_order().for_event(&event).finish();
    let user_id = order.user_id;
    assert_eq!(90, ticket_type.valid_available_ticket_count(connection).unwrap());

    order
        .update_quantities(
            user_id,
            &[UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 20,
                redemption_code: None,
            }],
            false,
            false,
            connection,
        )
        .unwrap();
    assert_eq!(80, ticket_type.valid_available_ticket_count(connection).unwrap());

    order
        .update_quantities(
            user_id,
            &[UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 16,
                redemption_code: None,
            }],
            false,
            false,
            connection,
        )
        .unwrap();
    assert_eq!(84, ticket_type.valid_available_ticket_count(connection).unwrap());
}

#[test]
fn update() {
    let db = TestProject::new();
    let connection = db.get_connection();
    let event = db.create_event().with_tickets().finish();
    let ticket_type = event.ticket_types(true, None, connection).unwrap().remove(0);
    //Change editable parameter and submit ticket type update request
    let update_name = String::from("updated_event_name");
    let update_start_date = NaiveDate::from_ymd(2018, 4, 23).and_hms(5, 14, 18);
    let update_end_date = NaiveDate::from_ymd(2018, 6, 1).and_hms(8, 5, 34);
    let update_parameters = TicketTypeEditableAttributes {
        name: Some(update_name.clone()),
        start_date: Some(Some(update_start_date)),
        end_date: Some(Some(update_end_date)),
        ..Default::default()
    };
    let id = ticket_type.id;
    let updated_ticket_type = ticket_type.update(update_parameters, None, connection).unwrap();
    assert_eq!(updated_ticket_type.id, id);
    assert_eq!(updated_ticket_type.name, update_name);
    assert_eq!(updated_ticket_type.start_date, Some(update_start_date));
    assert_eq!(updated_ticket_type.end_date, Some(update_end_date));
    assert_eq!(updated_ticket_type.end_date_type, TicketTypeEndDateType::Manual);

    // End date type set, end_date cleared and no validation is raised for missing a value
    let update_parameters = TicketTypeEditableAttributes {
        end_date: Some(Some(update_end_date)),
        end_date_type: Some(TicketTypeEndDateType::EventEnd),
        ..Default::default()
    };
    let updated_ticket_type = updated_ticket_type.update(update_parameters, None, connection).unwrap();
    assert_eq!(updated_ticket_type.end_date, None);
    assert_eq!(updated_ticket_type.end_date_type, TicketTypeEndDateType::EventEnd);

    // End date type set to manual, end_date persisted and no validation is raised
    let update_parameters = TicketTypeEditableAttributes {
        end_date: Some(Some(update_end_date)),
        end_date_type: Some(TicketTypeEndDateType::Manual),
        ..Default::default()
    };
    let updated_ticket_type = updated_ticket_type.update(update_parameters, None, connection).unwrap();
    assert_eq!(updated_ticket_type.end_date, Some(update_end_date));
    assert_eq!(updated_ticket_type.end_date_type, TicketTypeEndDateType::Manual);
}

#[test]
fn update_rank() {
    let db = TestProject::new();
    let conn = db.get_connection();
    let event = db.create_event().finish();
    event
        .add_ticket_type(
            "Tix1".to_string(),
            None,
            10,
            Some(times::now()),
            None,
            TicketTypeEndDateType::EventStart,
            None,
            None,
            10,
            10,
            TicketTypeVisibility::Always,
            None,
            0,
            true,
            true,
            true,
            None,
            conn,
        )
        .unwrap();
    let tix2 = event
        .add_ticket_type(
            "Tix2".to_string(),
            None,
            10,
            Some(times::now()),
            None,
            TicketTypeEndDateType::EventStart,
            None,
            None,
            10,
            10,
            TicketTypeVisibility::Always,
            None,
            0,
            true,
            true,
            true,
            None,
            conn,
        )
        .unwrap();
    let types = event.ticket_types(true, None, conn).unwrap();
    assert_eq!(
        types.iter().map(|tt| (tt.name.as_str(), tt.rank)).collect_vec(),
        vec![("Tix1", 0), ("Tix2", 1)]
    );

    // Reorder
    let attrs = TicketTypeEditableAttributes {
        rank: Some(0),
        ..Default::default()
    };
    let tix2 = tix2.update(attrs, None, conn).unwrap();
    let types = event.ticket_types(true, None, conn).unwrap();
    assert_eq!(
        types.iter().map(|tt| (tt.name.as_str(), tt.rank)).collect_vec(),
        vec![("Tix2", 0), ("Tix1", 1)]
    );

    // Reorder
    let attrs = TicketTypeEditableAttributes {
        rank: Some(1),
        ..Default::default()
    };
    tix2.update(attrs, None, conn).unwrap();
    let types = event.ticket_types(true, None, conn).unwrap();
    assert_eq!(
        types.iter().map(|tt| (tt.name.as_str(), tt.rank)).collect_vec(),
        vec![("Tix1", 0), ("Tix2", 1)]
    );
}

#[test]
fn cancel() {
    let db = TestProject::new();
    let connection = db.get_connection();
    let event = db.create_event().with_tickets().finish();
    let ticket_type = event.ticket_types(true, None, connection).unwrap().remove(0);

    let cancelled_ticket_type = ticket_type.cancel(connection).unwrap();

    assert_eq!(cancelled_ticket_type.status, TicketTypeStatus::Cancelled);
}

#[test]
pub fn update_with_validation_errors() {
    let db = TestProject::new();
    let connection = db.get_connection();
    let event = db.create_event().with_tickets().finish();
    let ticket_type = event.ticket_types(true, None, connection).unwrap().remove(0);
    //Change editable parameter and submit ticket type update request
    let update_name = String::from("updated_event_name");
    let update_start_date = NaiveDate::from_ymd(2018, 6, 23).and_hms(5, 14, 18);
    let update_end_date = NaiveDate::from_ymd(2018, 4, 1).and_hms(8, 5, 34);
    let update_parameters = TicketTypeEditableAttributes {
        name: Some(update_name.clone()),
        start_date: Some(Some(update_start_date)),
        end_date: Some(Some(update_end_date)),
        ..Default::default()
    };
    let result = ticket_type.clone().update(update_parameters, None, connection);
    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("start_date"));
                assert_eq!(errors["start_date"].len(), 1);
                assert_eq!(errors["start_date"][0].code, "start_date_must_be_before_end_date");
                assert_eq!(
                    &errors["start_date"][0].message.clone().unwrap().into_owned(),
                    "Start date must be before end date"
                );
            }
            _ => panic!("Expected validation error"),
        },
    }

    let update_parameters = TicketTypeEditableAttributes {
        end_date_type: Some(TicketTypeEndDateType::Manual),
        end_date: Some(None),
        ..Default::default()
    };
    let result = ticket_type.update(update_parameters, None, connection);
    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("end_date"));
                assert_eq!(errors["end_date"].len(), 1);
                assert_eq!(errors["end_date"][0].code, "required");
                assert_eq!(
                    &errors["end_date"][0].message.clone().unwrap().into_owned(),
                    "End date required for manual end date type"
                );
            }
            _ => panic!("Expected validation error"),
        },
    }
}

#[test]
fn find() {
    let db = TestProject::new();
    let event = db.create_event().with_tickets().finish();
    let ticket_type = &event.ticket_types(true, None, &db.get_connection()).unwrap()[0];

    let found_ticket_type = TicketType::find(ticket_type.id, &db.get_connection()).unwrap();
    assert_eq!(&found_ticket_type, ticket_type);
}

#[test]
fn tiered_pricing_update() {
    let db = TestProject::new();
    let conn = &db.connection;
    let event = db.create_event().finish();
    let end_date = dates::now().add_hours(2).finish();
    let ticket_type_a = event
        .add_ticket_type(
            "A".to_string(),
            None,
            2,
            Some(dates::now().add_hours(-1).finish()),
            Some(end_date),
            TicketTypeEndDateType::Manual,
            None,
            None,
            10,
            100,
            TicketTypeVisibility::Always,
            None,
            0,
            true,
            true,
            true,
            None,
            conn,
        )
        .unwrap();
    let ticket_type_b = event
        .add_ticket_type(
            "B".to_string(),
            None,
            100,
            None,
            Some(dates::now().add_hours(3).finish()),
            TicketTypeEndDateType::Manual,
            None,
            None,
            10,
            100,
            TicketTypeVisibility::Always,
            Some(ticket_type_a.id),
            0,
            true,
            true,
            true,
            None,
            conn,
        )
        .unwrap();

    let pricing = ticket_type_b.ticket_pricing(true, conn).unwrap();
    // Price should start when first tickets end
    assert_eq!(pricing[0].start_date.round_subsecs(4), end_date.round_subsecs(4));

    // Buy all the tickets
    db.create_order()
        .for_tickets(ticket_type_a.id)
        .quantity(2)
        .is_paid()
        .finish();

    let pricing = ticket_type_b.ticket_pricing(true, conn).unwrap();
    // Price should now have started, because all of the tickets are sold
    assert!(pricing[0].start_date <= dates::now().finish());

    // Updating the ticket type should not change the sales date

    let attrs = TicketTypeEditableAttributes {
        visibility: Some(TicketTypeVisibility::Hidden),
        ..Default::default()
    };

    let ticket_type_b = ticket_type_b.update(attrs, None, conn).unwrap();
    let pricing = ticket_type_b.ticket_pricing(true, conn).unwrap();

    assert!(pricing[0].start_date <= dates::now().finish());

    // Update a ticket type to a specific start date
    let new_start_date = dates::now().add_hours(1).finish();
    let attrs = TicketTypeEditableAttributes {
        start_date: Some(Some(new_start_date)),
        parent_id: Some(None),
        ..Default::default()
    };

    let ticket_type_b = ticket_type_b.update(attrs, None, conn).unwrap();
    let pricing = ticket_type_b.ticket_pricing(true, conn).unwrap();
    // Ticket sales should have paused.
    assert_eq!(pricing[0].start_date.round_subsecs(4), new_start_date.round_subsecs(4));

    // Update a ticket type to use parent again
    let attrs = TicketTypeEditableAttributes {
        start_date: Some(None),
        parent_id: Some(Some(ticket_type_a.id)),
        ..Default::default()
    };

    let ticket_type_b = ticket_type_b.update(attrs, None, conn).unwrap();
    let pricing = ticket_type_b.ticket_pricing(true, conn).unwrap();

    // Price should have started again, because all of the tickets are sold
    assert!(pricing[0].start_date <= dates::now().finish());
}

#[test]
#[should_panic]
pub fn additional_fee_cannot_be_more_than_org_max() {
    let project = TestProject::new();

    let organization = project
        .create_organization()
        .with_event_fee()
        .with_max_additional_fee(1)
        .finish();

    project
        .create_event()
        .with_organization(&organization)
        .with_ticket_type()
        .with_additional_fees(2)
        .finish();
}

#[test]
#[should_panic]
pub fn additional_fee_cannot_be_negative() {
    let project = TestProject::new();

    let organization = project
        .create_organization()
        .with_event_fee()
        .with_max_additional_fee(1)
        .finish();

    project
        .create_event()
        .with_organization(&organization)
        .with_ticket_type()
        .with_additional_fees(-1)
        .finish();
}
