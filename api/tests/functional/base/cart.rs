use crate::support;
use crate::support::database::TestDatabase;
use actix_web::{http::StatusCode, HttpResponse};
use bigneon_api::controllers::cart;
use bigneon_api::extractors::*;
use bigneon_api::models::*;
use bigneon_db::models::*;
use chrono::prelude::*;

pub async fn update_box_office_pricing(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let organization = database.create_organization().finish();
    let event = database
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();

    let user = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    let box_office_pricing = ticket_type
        .add_ticket_pricing(
            "Box office".into(),
            NaiveDate::from_ymd(2016, 7, 8).and_hms(7, 8, 10),
            NaiveDate::from_ymd(9999, 7, 8).and_hms(7, 8, 10),
            5000,
            true,
            None,
            None,
            connection,
        )
        .unwrap();

    let ticket_type_id = ticket_type.id;

    let input = Json(cart::UpdateCartRequest {
        box_office_pricing: Some(true),
        items: vec![cart::CartItem {
            ticket_type_id,
            quantity: 2,
            redemption_code: None,
        }],
        tracking_data: None,
    });

    let response: HttpResponse = cart::update_cart((
        database.connection.clone().into(),
        input,
        auth_user,
        RequestInfo { user_agent: None },
    ))
    .await
    .into();

    if should_test_succeed {
        assert_eq!(response.status(), StatusCode::OK);
        let cart = Order::find_cart_for_user(user.id, &connection).unwrap().unwrap();
        let items = cart.items(&connection).unwrap();
        let order_item = items.iter().find(|i| i.ticket_type_id == Some(ticket_type_id)).unwrap();

        assert_eq!(order_item.quantity, 2);
        assert!(order_item.find_fee_item(connection).unwrap().is_none());
        assert_eq!(order_item.unit_price_in_cents, box_office_pricing.price_in_cents);
    } else {
        support::expects_unauthorized(&response);
    }
}

pub async fn replace_box_office_pricing(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let organization = database.create_organization().finish();
    let event = database
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let event2 = database
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    let old_ticket_type = &event2.ticket_types(true, None, connection).unwrap()[0];

    let user = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);
    let box_office_pricing = ticket_type
        .add_ticket_pricing(
            "Box office".into(),
            NaiveDate::from_ymd(2016, 7, 8).and_hms(7, 8, 10),
            NaiveDate::from_ymd(9999, 7, 8).and_hms(7, 8, 10),
            5000,
            true,
            None,
            None,
            connection,
        )
        .unwrap();

    let ticket_type_id = ticket_type.id;

    // Existing cart
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    // Add normal tickets to cart (box_office_pricing = false)
    cart.update_quantities(
        user.id,
        &vec![UpdateOrderItem {
            ticket_type_id: old_ticket_type.id,
            quantity: 10,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();

    let items = cart.items(&connection).unwrap();
    assert!(items
        .iter()
        .find(|i| i.ticket_type_id == Some(old_ticket_type.id))
        .is_some());
    assert!(items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket_type.id))
        .is_none());

    let input = Json(cart::UpdateCartRequest {
        box_office_pricing: Some(true),
        items: vec![cart::CartItem {
            ticket_type_id,
            quantity: 2,
            redemption_code: None,
        }],
        tracking_data: None,
    });

    let response: HttpResponse = cart::replace_cart((
        database.connection.clone().into(),
        input,
        auth_user,
        RequestInfo { user_agent: None },
    ))
    .await
    .into();

    if should_test_succeed {
        assert_eq!(response.status(), StatusCode::OK);
        let cart = Order::find_cart_for_user(user.id, &connection).unwrap().unwrap();
        let items = cart.items(&connection).unwrap();
        assert!(items
            .iter()
            .find(|i| i.ticket_type_id == Some(old_ticket_type.id))
            .is_none());
        let order_item = items.iter().find(|i| i.ticket_type_id == Some(ticket_type_id)).unwrap();

        assert_eq!(order_item.quantity, 2);
        assert!(order_item.find_fee_item(connection).unwrap().is_none());
        assert_eq!(order_item.unit_price_in_cents, box_office_pricing.price_in_cents);
    } else {
        support::expects_unauthorized(&response);
    }
}
