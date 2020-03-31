use chrono::prelude::*;
use chrono::Duration;
use chrono::NaiveDateTime;
use db::dev::TestProject;
use db::models::*;
use db::schema::orders;
use db::utils::errors::ErrorCode::ValidationError;
use diesel;
use diesel::dsl;
use diesel::prelude::*;
use diesel::query_dsl::RunQueryDsl;
use uuid::Uuid;

#[test]
fn purchased_ticket_count() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let user2 = project.create_user().finish();
    let event = project
        .create_event()
        .with_ticket_type_count(1)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let code = project.create_code().with_event(&event).finish();
    let code2 = project.create_code().with_event(&event).finish();
    project
        .create_order()
        .for_event(&event)
        .quantity(10)
        .with_redemption_code(code.redemption_code.clone())
        .for_user(&user)
        .is_paid()
        .finish();
    project
        .create_order()
        .for_event(&event)
        .quantity(5)
        .with_redemption_code(code2.redemption_code.clone())
        .for_user(&user2)
        .is_paid()
        .finish();
    assert_eq!(code.purchased_ticket_count(&user, connection), Ok(10));
    assert_eq!(code.purchased_ticket_count(&user2, connection), Ok(0));
    assert_eq!(code2.purchased_ticket_count(&user2, connection), Ok(5));
    assert_eq!(code2.purchased_ticket_count(&user, connection), Ok(0));

    project
        .create_order()
        .for_event(&event)
        .quantity(5)
        .with_redemption_code(code.redemption_code.clone())
        .for_user(&user2)
        .is_paid()
        .finish();
    assert_eq!(code.purchased_ticket_count(&user, connection), Ok(10));
    assert_eq!(code.purchased_ticket_count(&user2, connection), Ok(5));
    assert_eq!(code2.purchased_ticket_count(&user2, connection), Ok(5));
    assert_eq!(code2.purchased_ticket_count(&user, connection), Ok(0));
}

#[test]
fn available() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project
        .create_event()
        .with_ticket_type_count(1)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let code = project.create_code().with_event(&event).with_max_uses(100).finish();
    assert_eq!(code.available(connection).unwrap(), Some(100));
    project
        .create_order()
        .for_event(&event)
        .quantity(10)
        .with_redemption_code(code.redemption_code.clone())
        .is_paid()
        .finish();
    assert_eq!(code.available(connection).unwrap(), Some(99));

    let code = project.create_code().with_event(&event).with_max_uses(0).finish();
    assert_eq!(code.available(connection).unwrap(), None);
}

#[test]
fn create() {
    let db = TestProject::new();
    let start_date = NaiveDateTime::from(Utc::now().naive_utc() - Duration::days(1));
    let end_date = NaiveDateTime::from(Utc::now().naive_utc() + Duration::days(2));
    let event = db.create_event().with_tickets().finish();
    Code::create(
        "test".into(),
        event.id,
        CodeTypes::Discount,
        "REDEMPTION".into(),
        10,
        Some(100),
        None,
        start_date,
        end_date,
        None,
    )
    .commit(None, db.get_connection())
    .unwrap();
}

#[test]
pub fn confirm_code_valid() {
    let db = TestProject::new();
    let code = db.create_code().finish();
    assert!(code.confirm_code_valid().is_ok());

    let start_date = NaiveDateTime::from(Utc::now().naive_utc() - Duration::days(3));
    let end_date = NaiveDateTime::from(Utc::now().naive_utc() - Duration::days(2));
    let code = db
        .create_code()
        .with_start_date(start_date)
        .with_end_date(end_date)
        .finish();
    let result = code.confirm_code_valid();

    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("code_id"));
                assert_eq!(errors["code_id"].len(), 1);
                assert_eq!(errors["code_id"][0].code, "invalid");
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
pub fn create_with_validation_errors() {
    let db = TestProject::new();
    let event = db.create_event().with_tickets().finish();
    let start_date = NaiveDateTime::from(Utc::now().naive_utc() + Duration::days(1));
    let end_date = NaiveDateTime::from(Utc::now().naive_utc() - Duration::days(2));
    let result = Code::create(
        "test".into(),
        event.id,
        CodeTypes::Discount,
        "A".into(),
        10,
        None,
        None,
        start_date,
        end_date,
        None,
    )
    .commit(None, db.get_connection());
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

                assert!(errors.contains_key("discounts"));
                assert_eq!(errors["discounts"].len(), 1);
                assert_eq!(errors["discounts"][0].code, "required");
                assert_eq!(
                    &errors["discounts"][0].message.clone().unwrap().into_owned(),
                    "Discount required for Discount code type"
                );
            }
            _ => panic!("Expected validation error"),
        },
    }

    // Dupe redemption code
    let code = db.create_code().with_event(&event).finish();
    let start_date = NaiveDateTime::from(Utc::now().naive_utc() - Duration::days(1));
    let end_date = NaiveDateTime::from(Utc::now().naive_utc() + Duration::days(2));
    let result = Code::create(
        "test".into(),
        event.id,
        CodeTypes::Discount,
        code.redemption_code,
        10,
        Some(100),
        None,
        start_date,
        end_date,
        None,
    )
    .commit(None, db.get_connection());
    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("redemption_code"));
                assert_eq!(errors["redemption_code"].len(), 1);
                assert_eq!(errors["redemption_code"][0].code, "uniqueness");
                assert_eq!(
                    &errors["redemption_code"][0].message.clone().unwrap().into_owned(),
                    "Redemption code must be unique"
                );
            }
            _ => panic!("Expected validation error"),
        },
    }

    // Redemption code used by a hold
    let hold = db.create_hold().with_event(&event).finish();
    let start_date = NaiveDateTime::from(Utc::now().naive_utc() - Duration::days(1));
    let end_date = NaiveDateTime::from(Utc::now().naive_utc() + Duration::days(2));
    let result = Code::create(
        "test".into(),
        event.id,
        CodeTypes::Discount,
        hold.redemption_code.unwrap(),
        10,
        Some(100),
        None,
        start_date,
        end_date,
        None,
    )
    .commit(None, db.get_connection());
    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("redemption_code"));
                assert_eq!(errors["redemption_code"].len(), 1);
                assert_eq!(errors["redemption_code"][0].code, "uniqueness");
                assert_eq!(
                    &errors["redemption_code"][0].message.clone().unwrap().into_owned(),
                    "Redemption code must be unique"
                );
            }
            _ => panic!("Expected validation error"),
        },
    }

    // Create with both absolute and percentage discounts
    let start_date = NaiveDateTime::from(Utc::now().naive_utc() - Duration::days(1));
    let end_date = NaiveDateTime::from(Utc::now().naive_utc() + Duration::days(2));
    let result = Code::create(
        "test2".into(),
        event.id,
        CodeTypes::Discount,
        "testing two discounts".into(),
        10,
        Some(100),
        Some(10),
        start_date,
        end_date,
        None,
    )
    .commit(None, db.get_connection());

    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }

        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("discounts"));
                assert_eq!(errors["discounts"].len(), 1);
                assert_eq!(errors["discounts"][0].code, "only_single_discount_type_allowed");
                assert_eq!(
                    &errors["discounts"][0].message.clone().unwrap().into_owned(),
                    "Cannot apply more than one type of discount"
                );
            }
            _ => panic!("Expected validation error"),
        },
    }

    // Access code does not require a discount
    let start_date = NaiveDateTime::from(Utc::now().naive_utc() - Duration::days(1));
    let end_date = NaiveDateTime::from(Utc::now().naive_utc() + Duration::days(2));
    let result = Code::create(
        "test".into(),
        event.id,
        CodeTypes::Access,
        "NEWUNUSEDCODE".into(),
        10,
        None,
        None,
        start_date,
        end_date,
        None,
    )
    .commit(None, db.get_connection());

    assert!(result.is_ok());
}

#[test]
fn update() {
    let db = TestProject::new();
    let code = db.create_code().finish();

    let update_patch = UpdateCodeAttributes {
        name: Some("New name".into()),
        ..Default::default()
    };
    let new_code = code.update(update_patch, None, db.get_connection()).unwrap();
    assert_eq!(new_code.name, "New name".to_string());
}

#[test]
pub fn update_with_validation_errors() {
    let db = TestProject::new();
    let event = db.create_event().with_tickets().finish();
    let code = db.create_code().with_event(&event).finish();
    let start_date = NaiveDateTime::from(Utc::now().naive_utc() + Duration::days(1));
    let end_date = NaiveDateTime::from(Utc::now().naive_utc() - Duration::days(2));

    let update_patch = UpdateCodeAttributes {
        redemption_code: Some("a".into()),
        start_date: Some(start_date),
        end_date: Some(end_date),
        discount_in_cents: Some(None),
        ..Default::default()
    };
    let result = code.update(update_patch, None, db.get_connection());
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

                assert!(errors.contains_key("discount_in_cents"));
                assert_eq!(errors["discount_in_cents"].len(), 1);
                assert_eq!(errors["discount_in_cents"][0].code, "required");
                assert_eq!(
                    &errors["discount_in_cents"][0].message.clone().unwrap().into_owned(),
                    "Discount required for Discount code type"
                );
            }
            _ => panic!("Expected validation error"),
        },
    }

    // Dupe redemption code
    let code2 = db.create_code().with_event(&event).finish();
    let update_patch = UpdateCodeAttributes {
        redemption_code: Some(code2.redemption_code),
        ..Default::default()
    };
    let result = code.update(update_patch, None, db.get_connection());
    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("redemption_code"));
                assert_eq!(errors["redemption_code"].len(), 1);
                assert_eq!(errors["redemption_code"][0].code, "uniqueness");
                assert_eq!(
                    &errors["redemption_code"][0].message.clone().unwrap().into_owned(),
                    "Redemption code must be unique"
                );
            }
            _ => panic!("Expected validation error"),
        },
    }

    // Dupe redemption code used by hold
    let hold = db.create_hold().with_event(&event).finish();
    let update_patch = UpdateCodeAttributes {
        redemption_code: hold.redemption_code,
        ..Default::default()
    };
    let result = code.update(update_patch, None, db.get_connection());
    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("redemption_code"));
                assert_eq!(errors["redemption_code"].len(), 1);
                assert_eq!(errors["redemption_code"][0].code, "uniqueness");
                assert_eq!(
                    &errors["redemption_code"][0].message.clone().unwrap().into_owned(),
                    "Redemption code must be unique"
                );
            }
            _ => panic!("Expected validation error"),
        },
    }
}

#[test]
fn find_by_redemption_code() {
    let db = TestProject::new();
    let connection = db.get_connection();
    let code = db.create_code().finish();
    let found_code = Code::find_by_redemption_code_with_availability(&code.redemption_code, None, connection).unwrap();
    assert_eq!(code, found_code.code);
}

#[test]
fn find() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let code = project.create_code().finish();

    // Record found
    let found_code = Code::find(code.id, connection).unwrap();
    assert_eq!(code, found_code);

    // Code does not exist so returns error
    assert!(Code::find(Uuid::new_v4(), connection).is_err());
}

#[test]
fn for_display() {
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
    let code = project
        .create_code()
        .with_event(&event)
        .for_ticket_type(&ticket_type)
        .for_ticket_type(&ticket_type2)
        .finish();

    let mut display_code: DisplayCodeAvailability = code.for_display(connection).unwrap();
    assert_eq!(code.id, display_code.display_code.id);
    assert_eq!(code.name, display_code.display_code.name);
    assert_eq!(vec![code.redemption_code], display_code.display_code.redemption_codes);
    assert_eq!(
        display_code.display_code.ticket_type_ids.sort(),
        vec![ticket_type.id, ticket_type2.id].sort()
    );
}

#[test]
fn update_ticket_types() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project
        .create_event()
        .with_ticket_pricing()
        .with_ticket_type_count(3)
        .finish();
    let ticket_types = event.ticket_types(true, None, &connection).unwrap();
    let ticket_type = &ticket_types[0];
    let ticket_type2 = &ticket_types[1];
    let ticket_type3 = &ticket_types[2];
    let code = project
        .create_code()
        .with_event(&event)
        .for_ticket_type(&ticket_type)
        .for_ticket_type(&ticket_type2)
        .finish();
    let mut display_code = code.for_display(&connection).unwrap();
    assert_eq!(
        display_code.display_code.ticket_type_ids.sort(),
        vec![ticket_type.id, ticket_type2.id].sort()
    );

    code.update_ticket_types(vec![ticket_type.id, ticket_type3.id], connection)
        .unwrap();
    let mut display_code = code.for_display(&connection).unwrap();
    assert_eq!(
        display_code.display_code.ticket_type_ids.sort(),
        vec![ticket_type.id, ticket_type3.id].sort()
    );
}

#[test]
fn event() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project.create_event().finish();
    let code = project.create_code().with_event(&event).finish();

    assert_eq!(code.event(connection).unwrap(), event);
}

#[test]
fn organization() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project.create_event().finish();
    let code = project.create_code().with_event(&event).finish();

    let organization = code.organization(connection).unwrap();
    assert_eq!(event.organization(connection).unwrap(), organization);
}

#[test]
fn destroy() {
    let project = TestProject::new();
    let code = project.create_code().finish();
    assert!(code.destroy(None, project.get_connection()).unwrap() > 0);
    assert!(Code::find(code.id, project.get_connection()).is_err());
}

#[test]
pub fn find_for_event() {
    let db = TestProject::new();
    let event = db.create_event().with_ticket_pricing().finish();
    let code = db
        .create_code()
        .with_name("Discount 1".into())
        .with_event(&event)
        .with_code_type(CodeTypes::Discount)
        .finish();
    let code2 = db
        .create_code()
        .with_name("Discount 2".into())
        .with_event(&event)
        .with_code_type(CodeTypes::Discount)
        .finish();
    let code3 = db
        .create_code()
        .with_name("Access 1".into())
        .with_event(&event)
        .with_code_type(CodeTypes::Access)
        .finish();
    let code4 = db
        .create_code()
        .with_name("Access 2".into())
        .with_event(&event)
        .with_code_type(CodeTypes::Access)
        .finish();

    let codes = Code::find_for_event(event.id, Some(CodeTypes::Discount), db.get_connection()).unwrap();
    assert_eq!(
        vec![
            code.for_display(db.get_connection()).unwrap(),
            code2.for_display(db.get_connection()).unwrap()
        ],
        codes
    );

    let codes = Code::find_for_event(event.id, Some(CodeTypes::Access), db.get_connection()).unwrap();
    assert_eq!(
        vec![
            code3.for_display(db.get_connection()).unwrap(),
            code4.for_display(db.get_connection()).unwrap()
        ],
        codes
    );
}

#[test]
pub fn find_number_of_uses() {
    let project = TestProject::new();
    let conn = project.get_connection();
    let user = project.create_user().finish();
    let event = project
        .create_event()
        .with_ticket_pricing()
        .with_ticket_type_count(2)
        .finish();
    let ticket_types = event.ticket_types(true, None, &conn).unwrap();
    let ticket_type = &ticket_types[0];
    let ticket_type2 = &ticket_types[1];
    let code = project
        .create_code()
        .with_name("Discount 1".into())
        .with_event(&event)
        .with_max_uses(2)
        .with_code_type(CodeTypes::Discount)
        .finish();

    let code_availability =
        Code::find_by_redemption_code_with_availability(code.redemption_code.clone().as_str(), Some(event.id), conn)
            .unwrap();
    assert_eq!(code_availability.available, Some(2));
    assert_eq!(Code::find_number_of_uses(code.id, None, conn), Ok(0));

    // Add some tickets to the cart
    let mut cart = Order::find_or_create_cart(&user, conn).unwrap();

    cart.update_quantities(
        user.id,
        &[
            UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 2,
                redemption_code: Some(code.redemption_code.clone()),
            },
            UpdateOrderItem {
                ticket_type_id: ticket_type2.id,
                quantity: 2,
                redemption_code: None,
            },
        ],
        false,
        false,
        conn,
    )
    .unwrap();

    let code_availability =
        Code::find_by_redemption_code_with_availability(code.redemption_code.clone().as_str(), Some(event.id), conn)
            .unwrap();
    assert_eq!(code_availability.available, Some(1));
    assert_eq!(Code::find_number_of_uses(code.id, None, conn), Ok(1));

    // Make cart expire
    // 1 minute ago expires
    let order = Order::find(cart.id, conn).unwrap();
    let one_minute_ago = NaiveDateTime::from(Utc::now().naive_utc() - Duration::minutes(1));
    let _order = diesel::update(orders::table.filter(orders::id.eq(order.id)))
        .set((orders::expires_at.eq(one_minute_ago), orders::updated_at.eq(dsl::now)))
        .get_result::<Order>(conn)
        .unwrap();

    let code_availability =
        Code::find_by_redemption_code_with_availability(code.redemption_code.clone().as_str(), Some(event.id), conn)
            .unwrap();
    assert_eq!(code_availability.available, Some(2));
    assert_eq!(Code::find_number_of_uses(code.id, None, conn), Ok(0));

    // Now buy the tickets and make the order expire
    cart.update_quantities(
        user.id,
        &[
            UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 2,
                redemption_code: Some(code.redemption_code.clone()),
            },
            UpdateOrderItem {
                ticket_type_id: ticket_type2.id,
                quantity: 2,
                redemption_code: None,
            },
        ],
        false,
        false,
        conn,
    )
    .unwrap();

    let total = cart.calculate_total(conn).unwrap();
    assert_eq!(total, 600);
    cart.add_external_payment(
        Some("Test".to_string()),
        ExternalPaymentType::CreditCard,
        user.id,
        total,
        conn,
    )
    .unwrap();

    // expire order
    let order = Order::find(cart.id, conn).unwrap();
    assert_eq!(order.status, OrderStatus::Paid);
    let one_minute_ago = NaiveDateTime::from(Utc::now().naive_utc() - Duration::minutes(1));
    let _order = diesel::update(orders::table.filter(orders::id.eq(order.id)))
        .set(orders::expires_at.eq(one_minute_ago))
        .get_result::<Order>(conn)
        .unwrap();

    let code_availability =
        Code::find_by_redemption_code_with_availability(code.redemption_code.clone().as_str(), Some(event.id), conn)
            .unwrap();
    assert_eq!(code_availability.available, Some(1));
    assert_eq!(Code::find_number_of_uses(code.id, None, conn), Ok(1));

    // Refund 1 of the 2 purchased tickets
    let items = cart.items(&conn).unwrap();
    let order_item = items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket_type.id) && i.code_id.is_some())
        .unwrap();
    let tickets = TicketInstance::find_for_order_item(order_item.id, conn).unwrap();
    let ticket = &tickets[0];
    let ticket2 = &tickets[1];
    let refund_items = vec![RefundItemRequest {
        order_item_id: order_item.id,
        ticket_instance_id: Some(ticket.id),
    }];
    cart.refund(&refund_items, user.id, None, false, conn).unwrap();

    let code_availability =
        Code::find_by_redemption_code_with_availability(code.redemption_code.clone().as_str(), Some(event.id), conn)
            .unwrap();
    assert_eq!(code_availability.available, Some(1));
    assert_eq!(Code::find_number_of_uses(code.id, None, conn), Ok(1));

    // Refund remaining ticket
    let refund_items = vec![RefundItemRequest {
        order_item_id: order_item.id,
        ticket_instance_id: Some(ticket2.id),
    }];
    cart.refund(&refund_items, user.id, None, false, conn).unwrap();

    let code_availability =
        Code::find_by_redemption_code_with_availability(code.redemption_code.clone().as_str(), Some(event.id), conn)
            .unwrap();
    assert_eq!(code_availability.available, Some(2));
    assert_eq!(Code::find_number_of_uses(code.id, None, conn), Ok(0));

    // Place two purchases
    project
        .create_order()
        .for_event(&event)
        .quantity(10)
        .with_redemption_code(code.redemption_code.clone())
        .is_paid()
        .finish();
    let code_availability =
        Code::find_by_redemption_code_with_availability(code.redemption_code.clone().as_str(), Some(event.id), conn)
            .unwrap();
    assert_eq!(code_availability.available, Some(1));
    assert_eq!(Code::find_number_of_uses(code.id, None, conn), Ok(1));

    project
        .create_order()
        .for_event(&event)
        .quantity(10)
        .with_redemption_code(code.redemption_code.clone())
        .is_paid()
        .finish();
    let code_availability =
        Code::find_by_redemption_code_with_availability(code.redemption_code.clone().as_str(), Some(event.id), conn)
            .unwrap();
    assert_eq!(code_availability.available, Some(0));
    assert_eq!(Code::find_number_of_uses(code.id, None, conn), Ok(2));
}

#[test]
pub fn find_for_event_access_code() {
    let db = TestProject::new();
    let conn = db.get_connection();
    let event = db.create_event().with_ticket_pricing().finish();

    let ticket_type_ids = event.ticket_types(true, None, conn).unwrap();

    let code1 = db
        .create_code()
        .with_name("Access 1".into())
        .with_redemption_code("ACCESS1".into())
        .with_event(&event)
        .with_code_type(CodeTypes::Access)
        .for_ticket_type(&ticket_type_ids[0])
        .finish();

    assert_eq!(event.ticket_types(true, None, conn).unwrap().len(), 0);
    assert_eq!(
        event
            .ticket_types(true, Some(code1.redemption_code.clone()), conn)
            .unwrap()
            .len(),
        1
    );

    code1.destroy(None, conn).unwrap();

    assert_eq!(event.ticket_types(true, None, conn).unwrap().len(), 1);
}
