use chrono::prelude::*;
use chrono::Duration;
use db::dev::TestProject;
use db::models::*;
use db::utils::dates;
use db::utils::errors::ErrorCode::ValidationError;
use uuid::Uuid;

#[test]
fn all() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let hold = project.create_hold().with_name("Hold1".to_string()).finish();
    let hold2 = project.create_hold().with_name("Hold2".to_string()).finish();
    let hold3 = project.create_hold().with_name("Hold3".to_string()).finish();
    assert_eq!(vec![hold, hold2, hold3], Hold::all(connection).unwrap());
}

#[test]
fn update_automatic_clear_domain_action() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let hold = project.create_hold().finish();
    let hold2 = project
        .create_hold()
        .with_end_at(dates::now().add_hours(2).finish())
        .finish();
    let hold3 = project
        .create_hold()
        .with_end_at(dates::now().add_hours(-2).finish())
        .finish();

    // Domain event should not be present as hold created has no end date
    let domain_action = DomainAction::upcoming_domain_action(
        Some(Tables::Holds),
        Some(hold.id),
        DomainActionTypes::ReleaseHoldInventory,
        connection,
    )
    .unwrap();
    assert!(domain_action.is_none());

    // Domain event should be present as hold created has end date
    let domain_action = DomainAction::upcoming_domain_action(
        Some(Tables::Holds),
        Some(hold2.id),
        DomainActionTypes::ReleaseHoldInventory,
        connection,
    )
    .unwrap()
    .unwrap();
    assert_eq!(domain_action.scheduled_at, hold2.end_at.unwrap());

    // Domain event should be present as hold created has end date but scheduled immediately
    let domain_action = DomainAction::upcoming_domain_action(
        Some(Tables::Holds),
        Some(hold3.id),
        DomainActionTypes::ReleaseHoldInventory,
        connection,
    )
    .unwrap()
    .unwrap();
    assert_ne!(domain_action.scheduled_at, hold3.end_at.unwrap());
    let now = Utc::now().naive_utc();
    assert_eq!(now.signed_duration_since(domain_action.scheduled_at).num_minutes(), 0);

    // Running the update logic does not create additional domain events (none for first hold as well)
    hold.update_automatic_clear_domain_action(connection).unwrap();
    hold2.update_automatic_clear_domain_action(connection).unwrap();
    hold3.update_automatic_clear_domain_action(connection).unwrap();
    let domain_actions = DomainAction::find_by_resource(
        Some(Tables::Holds),
        Some(hold.id),
        DomainActionTypes::ReleaseHoldInventory,
        DomainActionStatus::Pending,
        connection,
    )
    .unwrap();
    assert_eq!(0, domain_actions.len());
    let domain_actions = DomainAction::find_by_resource(
        Some(Tables::Holds),
        Some(hold2.id),
        DomainActionTypes::ReleaseHoldInventory,
        DomainActionStatus::Pending,
        connection,
    )
    .unwrap();
    assert_eq!(1, domain_actions.len());
    let domain_actions = DomainAction::find_by_resource(
        Some(Tables::Holds),
        Some(hold3.id),
        DomainActionTypes::ReleaseHoldInventory,
        DomainActionStatus::Pending,
        connection,
    )
    .unwrap();
    assert_eq!(1, domain_actions.len());
}

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
    let ticket_types = &event.ticket_types(true, None, connection).unwrap();
    let hold = project
        .create_hold()
        .with_hold_type(HoldTypes::Discount)
        .with_quantity(100)
        .with_max_per_user(10)
        .with_ticket_type_id(ticket_types[0].id)
        .finish();

    project
        .create_order()
        .for_event(&event)
        .quantity(10)
        .with_redemption_code(hold.redemption_code.clone().unwrap())
        .for_user(&user)
        .is_paid()
        .finish();
    assert_eq!(hold.purchased_ticket_count(&user, connection), Ok(10));
    assert_eq!(hold.purchased_ticket_count(&user2, connection), Ok(0));

    project
        .create_order()
        .for_event(&event)
        .quantity(5)
        .with_redemption_code(hold.redemption_code.clone().unwrap())
        .for_user(&user2)
        .is_paid()
        .finish();
    assert_eq!(hold.purchased_ticket_count(&user, connection), Ok(10));
    assert_eq!(hold.purchased_ticket_count(&user2, connection), Ok(5));
}

#[test]
fn quantity_and_children_quantity() {
    let db = TestProject::new();
    let connection = db.get_connection();
    let event = db.create_event().with_ticket_pricing().finish();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    let hold = db
        .create_hold()
        .with_name("Hold 1".to_string())
        .with_event(&event)
        .with_quantity(10)
        .finish();
    let hold2 = db
        .create_hold()
        .with_name("Hold 2".to_string())
        .with_event(&event)
        .with_quantity(10)
        .finish();
    let comp = db
        .create_comp()
        .with_name("Comp 1".to_string())
        .with_hold(&hold)
        .with_quantity(2)
        .finish();
    let comp2 = db
        .create_comp()
        .with_name("Comp 2".to_string())
        .with_hold(&comp)
        .with_quantity(1)
        .finish();

    // Purchase 1 of first hold's remaining 9
    let user = db.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 1,
            redemption_code: hold.redemption_code.clone(),
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

    let (quantity, available) = hold.quantity(connection).unwrap();
    assert_eq!(quantity, 8);
    assert_eq!(available, 7);

    let display_hold = hold.clone().into_display(connection).unwrap();
    assert_eq!(display_hold.quantity, quantity);
    assert_eq!(display_hold.available, available);
    assert_eq!(display_hold.id, hold.id);

    let (quantity, available) = hold.children_quantity(connection).unwrap();
    assert_eq!(quantity, 2);
    assert_eq!(available, 2);

    let (quantity, available) = hold2.quantity(connection).unwrap();
    assert_eq!(quantity, 10);
    assert_eq!(available, 10);
    let (quantity, available) = hold2.children_quantity(connection).unwrap();
    assert_eq!(quantity, 0);
    assert_eq!(available, 0);

    let (quantity, available) = comp.quantity(connection).unwrap();
    assert_eq!(quantity, 1);
    assert_eq!(available, 1);
    let (quantity, available) = comp.children_quantity(connection).unwrap();
    assert_eq!(quantity, 1);
    assert_eq!(available, 1);

    let (quantity, available) = comp2.quantity(connection).unwrap();
    assert_eq!(quantity, 1);
    assert_eq!(available, 1);
    let (quantity, available) = comp2.children_quantity(connection).unwrap();
    assert_eq!(quantity, 0);
    assert_eq!(available, 0);
}

#[test]
fn create() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project.create_event().with_tickets().finish();
    let name = "test".to_string();
    let code = "IHAVEACODE".to_string();
    let hold = Hold::create_hold(
        name.clone(),
        event.id,
        Some(code.clone()),
        Some(0),
        Some(dates::now().add_hours(1).finish()),
        Some(4),
        HoldTypes::Discount,
        event.ticket_types(true, None, connection).unwrap()[0].id,
    )
    .commit(None, connection)
    .unwrap();
    assert_eq!(name, hold.name);
    assert_eq!(code, hold.redemption_code.unwrap());
    assert_eq!(event.id, hold.event_id);

    let domain_action = DomainAction::upcoming_domain_action(
        Some(Tables::Holds),
        Some(hold.id),
        DomainActionTypes::ReleaseHoldInventory,
        connection,
    )
    .unwrap()
    .unwrap();
    assert_eq!(domain_action.scheduled_at, hold.end_at.unwrap());
}

#[test]
fn create_with_validation_errors() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project.create_event().with_tickets().finish();
    let result = Hold::create_hold(
        "test".to_string(),
        event.id,
        Some("IHAVEACODE".to_string()),
        None,
        None,
        Some(4),
        HoldTypes::Discount,
        event.ticket_types(true, None, connection).unwrap()[0].id,
    )
    .commit(None, connection);

    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("discount_in_cents"));
                assert_eq!(errors["discount_in_cents"].len(), 1);
                assert_eq!(errors["discount_in_cents"][0].code, "required");
            }
            _ => panic!("Expected validation error"),
        },
    }

    // Dupe redemption code
    let hold = project.create_hold().with_event(&event).finish();
    let result = Hold::create_hold(
        "test".to_string(),
        event.id,
        hold.redemption_code,
        Some(0),
        None,
        Some(4),
        HoldTypes::Discount,
        event.ticket_types(true, None, connection).unwrap()[0].id,
    )
    .commit(None, connection);
    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("redemption_code"));
                assert_eq!(errors["redemption_code"].len(), 1);
                assert_eq!(errors["redemption_code"][0].code, "uniqueness");
            }
            _ => panic!("Expected validation error"),
        },
    }

    // Redemption code used by a code
    let code = project.create_code().with_event(&event).finish();
    let result = Hold::create_hold(
        "test".to_string(),
        event.id,
        Some(code.redemption_code),
        Some(0),
        None,
        Some(4),
        HoldTypes::Discount,
        event.ticket_types(true, None, connection).unwrap()[0].id,
    )
    .commit(None, connection);
    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("redemption_code"));
                assert_eq!(errors["redemption_code"].len(), 1);
                assert_eq!(errors["redemption_code"][0].code, "uniqueness");
            }
            _ => panic!("Expected validation error"),
        },
    }
}

#[test]
pub fn confirm_hold_valid() {
    let db = TestProject::new();
    let hold = db.create_hold().finish();
    assert!(hold.confirm_hold_valid().is_ok());

    let end_at = NaiveDateTime::from(Utc::now().naive_utc() - Duration::days(2));
    let hold = db.create_hold().with_end_at(end_at).finish();
    let result = hold.confirm_hold_valid();

    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("hold_id"));
                assert_eq!(errors["hold_id"].len(), 1);
                assert_eq!(errors["hold_id"][0].code, "invalid");
                assert_eq!(
                    &errors["hold_id"][0].message.clone().unwrap().into_owned(),
                    "Hold not valid for current datetime"
                );
            }
            _ => panic!("Expected validation error"),
        },
    }
}

#[test]
fn update() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let hold = project.create_hold().finish();

    let update_patch = UpdateHoldAttributes {
        discount_in_cents: Some(Some(10)),
        max_per_user: Some(None),
        end_at: Some(None),
        name: Some("New name".to_string()),
        ..Default::default()
    };
    // No release inventory event
    let domain_action = DomainAction::upcoming_domain_action(
        Some(Tables::Holds),
        Some(hold.id),
        DomainActionTypes::ReleaseHoldInventory,
        connection,
    )
    .unwrap();
    assert!(domain_action.is_none());

    let hold = hold.update(update_patch, connection).unwrap();
    assert_eq!(hold.name, "New name".to_string());
    assert_eq!(hold.max_per_user, None);
    assert_eq!(hold.end_at, None);
    assert_eq!(hold.discount_in_cents, Some(10));

    // No release inventory event
    let domain_action = DomainAction::upcoming_domain_action(
        Some(Tables::Holds),
        Some(hold.id),
        DomainActionTypes::ReleaseHoldInventory,
        connection,
    )
    .unwrap();
    assert!(domain_action.is_none());

    // With end at set, release inventory action created
    let update_patch = UpdateHoldAttributes {
        discount_in_cents: Some(Some(10)),
        hold_type: Some(HoldTypes::Comp),
        end_at: Some(Some(dates::now().add_hours(2).finish())),
        ..Default::default()
    };
    let hold = hold.update(update_patch, connection).unwrap();
    assert_eq!(hold.discount_in_cents, None);
    assert_eq!(hold.hold_type, HoldTypes::Comp);
    let domain_action = DomainAction::upcoming_domain_action(
        Some(Tables::Holds),
        Some(hold.id),
        DomainActionTypes::ReleaseHoldInventory,
        connection,
    )
    .unwrap()
    .unwrap();
    assert_eq!(domain_action.scheduled_at, hold.end_at.unwrap());

    // With end at removed, domain action is also removed
    let update_patch = UpdateHoldAttributes {
        end_at: Some(None),
        ..Default::default()
    };
    let hold = hold.update(update_patch, connection).unwrap();
    let domain_action = DomainAction::upcoming_domain_action(
        Some(Tables::Holds),
        Some(hold.id),
        DomainActionTypes::ReleaseHoldInventory,
        connection,
    )
    .unwrap();
    assert!(domain_action.is_none());
}

#[test]
fn update_with_validation_errors() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project.create_event().with_tickets().finish();
    let hold = project
        .create_hold()
        .with_event(&event)
        .with_hold_type(HoldTypes::Comp)
        .finish();
    assert!(hold.discount_in_cents.is_none());

    let update_patch = UpdateHoldAttributes {
        hold_type: Some(HoldTypes::Discount),
        ..Default::default()
    };
    let result = hold.update(update_patch, connection);
    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("discount_in_cents"));
                assert_eq!(errors["discount_in_cents"].len(), 1);
                assert_eq!(errors["discount_in_cents"][0].code, "required");
            }
            _ => panic!("Expected validation error"),
        },
    }

    // Dupe redemption code
    let hold2 = project.create_hold().with_event(&event).finish();
    let update_patch = UpdateHoldAttributes {
        redemption_code: Some(hold2.redemption_code),
        ..Default::default()
    };
    let result = hold.update(update_patch, connection);
    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("redemption_code"));
                assert_eq!(errors["redemption_code"].len(), 1);
                assert_eq!(errors["redemption_code"][0].code, "uniqueness");
            }
            _ => panic!("Expected validation error"),
        },
    }

    // Dupe redemption code used by code
    let code = project.create_code().with_event(&event).finish();
    let update_patch = UpdateHoldAttributes {
        redemption_code: Some(Some(code.redemption_code)),
        ..Default::default()
    };
    let result = hold.update(update_patch, connection);
    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("redemption_code"));
                assert_eq!(errors["redemption_code"].len(), 1);
                assert_eq!(errors["redemption_code"][0].code, "uniqueness");
            }
            _ => panic!("Expected validation error"),
        },
    }

    // Create and assign comp to active cart
    let organization = project.create_organization().with_fees().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let hold = project.create_hold().with_event(&event).finish();
    let comp = project.create_comp().with_quantity(10).with_hold(&hold).finish();

    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    cart.update_quantities(
        user.id,
        &vec![UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 10,
            redemption_code: comp.redemption_code,
        }],
        false,
        false,
        connection,
    )
    .unwrap();
}

#[test]
fn find_by_ticket_type() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let hold = project.create_hold().finish();
    let ticket_type = TicketType::find(hold.ticket_type_id, connection).unwrap();

    assert_eq!(
        vec![hold],
        Hold::find_by_ticket_type(ticket_type.id, connection).unwrap()
    );
}

#[test]
fn remove_available_quantity() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let hold = project.create_hold().finish();

    let child_hold = Hold::create_comp_for_person(
        "Child".into(),
        None,
        hold.id,
        None,
        None,
        "ChildCode".into(),
        None,
        None,
        2,
        connection,
    )
    .unwrap();
    let ticket_type = TicketType::find(hold.ticket_type_id, connection).unwrap();

    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    cart.update_quantities(
        user.id,
        &vec![
            UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 4,
                redemption_code: hold.redemption_code.clone(),
            },
            UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 1,
                redemption_code: child_hold.redemption_code.clone(),
            },
        ],
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

    // Add additional cart item from existing unsold quantity (is removed from hold)
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    cart.update_quantities(
        user.id,
        &vec![UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 1,
            redemption_code: hold.redemption_code.clone(),
        }],
        false,
        false,
        connection,
    )
    .unwrap();

    assert_eq!(8, hold.quantity(connection).unwrap().0);
    assert_eq!(2, child_hold.quantity(connection).unwrap().0);

    hold.remove_available_quantity(None, connection).unwrap();
    assert_eq!(4, hold.quantity(connection).unwrap().0);
    assert_eq!(1, child_hold.quantity(connection).unwrap().0);
}

#[test]
fn destroy() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let hold = project.create_hold().finish();
    assert!(hold.clone().destroy(None, connection).is_ok());
    assert!(Hold::find(hold.id, connection).unwrap().deleted_at.is_some());

    // Destroy hold with comps
    let comp = project.create_comp().finish();
    let hold = Hold::find(comp.id, connection).unwrap();
    hold.clone().destroy(None, connection).unwrap();
    assert!(Hold::find(hold.id, connection).unwrap().deleted_at.is_some());
}

#[test]
fn set_quantity() {
    let db = TestProject::new();
    let event = db.create_event().with_tickets().finish();
    let hold = db.create_hold().with_event(&event).finish();
    hold.set_quantity(None, 30, db.get_connection()).unwrap();

    assert_eq!(hold.quantity(db.get_connection()).unwrap(), (30, 30));
}

#[test]
fn event() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project.create_event().with_ticket_pricing().finish();
    let hold = project.create_hold().with_event(&event).finish();

    assert_eq!(hold.event(connection).unwrap(), event);
}

#[test]
fn organization() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project.create_event().with_ticket_pricing().finish();
    let hold = project.create_hold().with_event(&event).finish();

    let organization = hold.organization(connection).unwrap();
    assert_eq!(event.organization(connection).unwrap(), organization);
}

#[test]
fn find() {
    let db = TestProject::new();
    let hold = db.create_hold().finish();

    let db_hold = Hold::find(hold.id, db.get_connection()).unwrap();

    assert_eq!(hold, db_hold);
}

#[test]
fn split() {
    let db = TestProject::new();
    let connection = db.get_connection();
    let hold = db.create_hold().finish();
    let name = "Name1".to_string();
    let redemption_code = "ABCD34927262".to_string();
    let child = true;
    let end_at = None;
    let max_per_user = None;
    let discount_in_cents = None;
    let email = Some("email@domain.com".to_string());
    let phone = Some("11111111111111".to_string());
    let quantity = 2;
    let hold_type = HoldTypes::Comp;

    // Split off a comp that uses the current hold as its new parent
    let new_hold = hold
        .split(
            None,
            name.clone(),
            email.clone(),
            phone.clone(),
            redemption_code.clone(),
            quantity,
            discount_in_cents,
            hold_type,
            end_at,
            max_per_user,
            child,
            connection,
        )
        .unwrap();

    assert_eq!(name, new_hold.name);
    assert_eq!(email.clone(), new_hold.email);
    assert_eq!(phone.clone(), new_hold.phone);
    assert_eq!(Some(redemption_code), new_hold.redemption_code);
    assert_eq!(quantity, new_hold.quantity(connection).unwrap().0);
    assert_eq!(discount_in_cents.map(|n| n as i64), new_hold.discount_in_cents);
    assert_eq!(hold_type, new_hold.hold_type);
    assert_eq!(end_at, new_hold.end_at);
    assert_eq!(max_per_user.map(|n| n as i64), new_hold.max_per_user);
    assert_eq!(Some(hold.id), new_hold.parent_hold_id);

    // Split off a discount hold that uses the current hold as its parent
    let hold_type = HoldTypes::Discount;
    let discount_in_cents = Some(33);
    let name = "Name2".to_string();
    let redemption_code = "ABCD34927263".to_string();
    let new_hold = hold
        .split(
            None,
            name.clone(),
            email.clone(),
            phone.clone(),
            redemption_code.clone(),
            quantity,
            discount_in_cents,
            hold_type,
            end_at,
            max_per_user,
            child,
            connection,
        )
        .unwrap();

    assert_eq!(name, new_hold.name);
    assert_eq!(email.clone(), new_hold.email);
    assert_eq!(phone.clone(), new_hold.phone);
    assert_eq!(Some(redemption_code), new_hold.redemption_code);
    assert_eq!(quantity, new_hold.quantity(connection).unwrap().0);
    assert_eq!(discount_in_cents.map(|n| n as i64), new_hold.discount_in_cents);
    assert_eq!(hold_type, new_hold.hold_type);
    assert_eq!(end_at, new_hold.end_at);
    assert_eq!(max_per_user.map(|n| n as i64), new_hold.max_per_user);
    assert_eq!(Some(hold.id), new_hold.parent_hold_id);

    // Split off Discount that isn't a child of existing hold
    let child = false;
    let name = "Name3".to_string();
    let redemption_code = "ABCD34927264".to_string();
    let new_hold = hold
        .split(
            None,
            name.clone(),
            email.clone(),
            phone.clone(),
            redemption_code.clone(),
            quantity,
            discount_in_cents,
            hold_type,
            end_at,
            max_per_user,
            child,
            connection,
        )
        .unwrap();

    assert_eq!(name, new_hold.name);
    assert_eq!(email.clone(), new_hold.email);
    assert_eq!(phone.clone(), new_hold.phone);
    assert_eq!(Some(redemption_code), new_hold.redemption_code);
    assert_eq!(quantity, new_hold.quantity(connection).unwrap().0);
    assert_eq!(discount_in_cents.map(|n| n as i64), new_hold.discount_in_cents);
    assert_eq!(hold_type, new_hold.hold_type);
    assert_eq!(end_at, new_hold.end_at);
    assert_eq!(max_per_user.map(|n| n as i64), new_hold.max_per_user);
    assert_eq!(hold.parent_hold_id, new_hold.parent_hold_id);
}

#[test]
fn find_for_event() {
    let db = TestProject::new();
    let event = db.create_event().with_ticket_pricing().finish();
    let hold = db
        .create_hold()
        .with_name("Hold 1".to_string())
        .with_event(&event)
        .finish();
    let hold2 = db
        .create_hold()
        .with_name("Hold 2".to_string())
        .with_event(&event)
        .finish();
    let comp = db.create_comp().with_name("Comp".to_string()).with_hold(&hold).finish();

    // Only parent holds
    let holds = Hold::find_for_event(event.id, false, db.get_connection()).unwrap();
    assert_eq!(vec![hold.clone(), hold2.clone()], holds);

    // Include children
    let holds = Hold::find_for_event(event.id, true, db.get_connection()).unwrap();
    assert_eq!(vec![comp, hold, hold2], holds);
}

#[test]
fn find_by_parent_hold_id() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let comp = project.create_comp().finish();

    // Record found
    let found_comp =
        Hold::find_by_parent_id(comp.parent_hold_id.unwrap(), Some(HoldTypes::Comp), 0, 1000, connection).unwrap();
    assert_eq!(comp, found_comp.data[0]);

    // Comp does not exist for hold so returns error
    assert!(
        Hold::find_by_parent_id(Uuid::new_v4(), Some(HoldTypes::Comp), 0, 1000, connection)
            .unwrap()
            .is_empty()
    );

    assert!(Hold::find_by_parent_id(
        comp.parent_hold_id.unwrap(),
        Some(HoldTypes::Discount),
        0,
        1000,
        connection
    )
    .unwrap()
    .is_empty());
    let parent_id = comp.parent_hold_id.unwrap();

    // Record found when not filtering by types
    let found_comp = Hold::find_by_parent_id(parent_id, None, 0, 1000, connection).unwrap();
    assert_eq!(comp, found_comp.data[0]);

    let parent = Hold::find(parent_id, connection).unwrap();
    assert_eq!(parent.comps(connection).unwrap(), vec![comp.clone()]);

    // Should be removed from results once destroyed
    comp.destroy(None, connection).unwrap();
    let found_comp = Hold::find_by_parent_id(parent_id, None, 0, 1000, connection).unwrap();
    assert_eq!(0, found_comp.data.len());
    assert!(parent.comps(connection).unwrap().is_empty());
}
