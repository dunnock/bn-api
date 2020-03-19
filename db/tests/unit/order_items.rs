use chrono::{Duration, NaiveDateTime, Utc};
use db::dev::TestProject;
use db::models::*;
use db::utils::errors::ErrorCode::ValidationError;

#[test]
fn find_fee_item() {
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
    let ticket = &event.ticket_types(true, None, connection).unwrap()[0];
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket.id,
            quantity: 10,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();

    let items = cart.items(&connection).unwrap();
    let order_item = items.iter().find(|i| i.ticket_type_id == Some(ticket.id)).unwrap();

    let fee_item = order_item.find_fee_item(connection).unwrap().unwrap();

    assert_eq!(fee_item.parent_id, Some(order_item.id));
    assert_eq!(fee_item.item_type, OrderItemTypes::PerUnitFees);
}

#[test]
fn event() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project.create_event().with_tickets().finish();
    let order = project.create_order().for_event(&event).is_paid().finish();
    let items = order.items(&connection).unwrap();
    let order_item = items.iter().find(|i| i.item_type == OrderItemTypes::Tickets).unwrap();
    assert_eq!(order_item.event(connection).unwrap(), event);
}

#[test]
fn order() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    let ticket = &event.ticket_types(true, None, connection).unwrap()[0];
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket.id,
            quantity: 1,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();

    let items = cart.items(&connection).unwrap();
    let order_item = items.iter().find(|i| i.ticket_type_id == Some(ticket.id)).unwrap();
    assert_eq!(order_item.order(connection).unwrap().id, cart.id);
}

#[test]
fn code() {
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
    let code = project
        .create_code()
        .with_event(&event)
        .for_ticket_type(&ticket_type)
        .with_code_type(CodeTypes::Discount)
        .with_max_tickets_per_user(Some(5))
        .with_max_uses(1)
        .finish();

    cart.update_quantities(
        user.id,
        &[
            UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 1,
                redemption_code: Some(code.redemption_code.clone()),
            },
            UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 1,
                redemption_code: None,
            },
        ],
        false,
        false,
        connection,
    )
    .unwrap();
    let items = cart.items(connection).unwrap();

    let order_item = items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket_type.id) && i.code_id == Some(code.id))
        .unwrap();
    let order_item2 = items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket_type.id) && i.code_id.is_none())
        .unwrap();
    assert_eq!(Some(code), order_item.code(connection).unwrap());
    assert_eq!(None, order_item2.code(connection).unwrap());
}

#[test]
fn confirm_code_valid() {
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
    let code = project
        .create_code()
        .with_event(&event)
        .for_ticket_type(&ticket_type)
        .with_code_type(CodeTypes::Discount)
        .with_max_tickets_per_user(Some(5))
        .with_max_uses(1)
        .finish();

    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 1,
            redemption_code: Some(code.redemption_code.clone()),
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    let items = cart.items(connection).unwrap();

    let order_item = items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket_type.id) && i.code_id == Some(code.id))
        .unwrap();
    assert!(order_item.confirm_code_valid(connection).is_ok());

    let start_date = NaiveDateTime::from(Utc::now().naive_utc() - Duration::days(3));
    let end_date = NaiveDateTime::from(Utc::now().naive_utc() - Duration::days(2));
    code.update(
        UpdateCodeAttributes {
            start_date: Some(start_date),
            end_date: Some(end_date),
            ..Default::default()
        },
        None,
        connection,
    )
    .unwrap();
    assert!(order_item.confirm_code_valid(connection).is_err());
}

#[test]
fn update_with_validation_errors() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().with_fees().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .with_ticket_type_count(2)
        .finish();
    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    let ticket_type = &event.ticket_types(false, None, connection).unwrap()[0];
    let ticket_type2 = &event.ticket_types(false, None, connection).unwrap()[1];

    let code = project
        .create_code()
        .with_event(&event)
        .for_ticket_type(&ticket_type)
        .with_code_type(CodeTypes::Discount)
        .with_max_tickets_per_user(Some(5))
        .with_max_uses(100)
        .finish();

    // max_tickets_per_user_reached 6 with a limit of 5
    let result = cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 6,
            redemption_code: Some(code.redemption_code.clone()),
        }],
        false,
        false,
        connection,
    );

    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("quantity"));
                assert_eq!(errors["quantity"].len(), 1);
                assert_eq!(errors["quantity"][0].code, "limit_per_person_exceeded");
                assert_eq!(
                    &errors["quantity"][0].message.clone().unwrap().into_owned(),
                    &format!("Max of {} uses for code {} exceeded", 5, code.redemption_code)
                );
            }
            _ => panic!("Expected validation error"),
        },
    }

    // Purchasing the limit of 5 succeeds
    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 5,
            redemption_code: Some(code.redemption_code.clone()),
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    let total = cart.calculate_total(connection).unwrap();
    cart.add_external_payment(
        Some("Test".to_string()),
        ExternalPaymentType::CreditCard,
        user.id,
        total,
        connection,
    )
    .unwrap();

    // Ticket type requires access code
    let code = project
        .create_code()
        .with_event(&event)
        .for_ticket_type(&ticket_type2)
        .with_code_type(CodeTypes::Access)
        .finish();
    // Check to confirm having multiple access codes tied to a ticket type will not interfere
    project
        .create_code()
        .with_event(&event)
        .for_ticket_type(&ticket_type2)
        .with_code_type(CodeTypes::Access)
        .finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    let result = cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type2.id,
            quantity: 1,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    );

    match result {
        Ok(_) => {
            panic!("Expected error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("code_id"));
                assert_eq!(errors["code_id"].len(), 1);
                assert_eq!(errors["code_id"][0].code, "ticket_type_requires_access_code");
                assert_eq!(
                    &errors["code_id"][0].message.clone().unwrap().into_owned(),
                    "Ticket type requires access code for purchase"
                );
            }
            _ => panic!("Expected validation error"),
        },
    }

    let result = cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type2.id,
            quantity: 1,
            redemption_code: Some(code.redemption_code),
        }],
        false,
        false,
        connection,
    );
    assert!(result.is_ok());

    // Code not active but is being added to the cart
    let start_date = NaiveDateTime::from(Utc::now().naive_utc() - Duration::days(3));
    let end_date = NaiveDateTime::from(Utc::now().naive_utc() - Duration::days(2));
    let code = project
        .create_code()
        .with_event(&event)
        .for_ticket_type(&ticket_type)
        .with_code_type(CodeTypes::Discount)
        .with_start_date(start_date)
        .with_end_date(end_date)
        .finish();

    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    let result = cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 1,
            redemption_code: Some(code.redemption_code),
        }],
        false,
        false,
        connection,
    );
    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("code_id"));
                assert_eq!(errors["code_id"].len(), 1);
                assert_eq!(&errors["code_id"][0].code, "invalid");
                assert_eq!(
                    &errors["code_id"][0].message.clone().unwrap().into_owned(),
                    "Code not valid for current datetime"
                );
            }
            _ => panic!("Expected validation error"),
        },
    }
}

#[test]
fn calculate_quantity() {
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
    let ticket = &event.ticket_types(true, None, connection).unwrap()[0];
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket.id,
            quantity: 10,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();

    let items = cart.items(&connection).unwrap();
    let order_item = items.iter().find(|i| i.ticket_type_id == Some(ticket.id)).unwrap();

    assert_eq!(order_item.calculate_quantity(connection), Ok(10));

    //let datetime_in_past = NaiveDate::from_ymd(2018, 9, 16).and_hms(12, 12, 12);
    //diesel::update(ticket_instances::table.filter(ticket_instances::id.eq(tickets[0].id)))
    //    .set(ticket_instances::reserved_until.eq(datetime_in_past))
    //    .get_result::<TicketInstance>(connection)
    //    .unwrap();

    //assert_eq!(order_item.calculate_quantity(connection), Ok(9));
}
