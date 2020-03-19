use crate::functional::base;
use crate::support::database::TestDatabase;
use crate::support::test_request::TestRequest;
use crate::support::{self, *};
use actix_web::{
    http::StatusCode,
    web::{Path, Query},
    FromRequest, HttpResponse,
};
use api::controllers;
use api::controllers::cart;
use api::controllers::cart::*;
use api::domain_events::executors::ProcessPaymentIPNExecutor;
use api::extractors::*;
use api::models::*;
use chrono::prelude::*;
use chrono::Duration;
use db::models::*;
use db::schema::{orders, ticket_instances};
use db::utils::dates;
use diesel;
use diesel::prelude::*;
use diesel::sql_types;
use globee::Customer;
use globee::Email;
use globee::GlobeeIpnRequest;
use globee::PaymentDetails;
use serde_json;
use uuid::Uuid;

#[cfg(test)]
mod update_box_office_pricing_tests {
    use super::*;
    #[actix_rt::test]
    async fn update_box_office_pricing_org_member() {
        base::cart::update_box_office_pricing(Roles::OrgMember, true).await;
    }
    #[actix_rt::test]
    async fn update_box_office_pricing_admin() {
        base::cart::update_box_office_pricing(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn update_box_office_pricing_user() {
        base::cart::update_box_office_pricing(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn update_box_office_pricing_org_owner() {
        base::cart::update_box_office_pricing(Roles::OrgOwner, true).await;
    }
}

#[cfg(test)]
mod replace_box_office_pricing_tests {
    use super::*;
    #[actix_rt::test]
    async fn replace_box_office_pricing_org_member() {
        base::cart::replace_box_office_pricing(Roles::OrgMember, true).await;
    }
    #[actix_rt::test]
    async fn replace_box_office_pricing_admin() {
        base::cart::replace_box_office_pricing(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn replace_box_office_pricing_user() {
        base::cart::replace_box_office_pricing(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn replace_box_office_pricing_org_owner() {
        base::cart::replace_box_office_pricing(Roles::OrgOwner, true).await;
    }
}

#[actix_rt::test]
async fn duplicate() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let event = database.create_event().with_tickets().with_ticket_pricing().finish();

    let order = database.create_order().quantity(5).for_event(&event).finish();
    let user = order.user(connection).unwrap();
    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);
    let tickets: Vec<Uuid> = order.tickets(None, connection).unwrap().iter().map(|t| t.id).collect();
    diesel::sql_query(
        r#"
        UPDATE orders
        SET created_at = $2, expires_at = $2
        WHERE id = $1;
        "#,
    )
    .bind::<sql_types::Uuid, _>(order.id)
    .bind::<sql_types::Timestamp, _>(dates::now().add_days(-7).finish())
    .execute(connection)
    .unwrap();
    diesel::sql_query(
        r#"
        UPDATE ticket_instances
        SET reserved_until = $2
        WHERE id = ANY($1);
        "#,
    )
    .bind::<sql_types::Array<sql_types::Uuid>, _>(tickets)
    .bind::<sql_types::Timestamp, _>(dates::now().add_days(-7).finish())
    .execute(connection)
    .unwrap();

    // Current cart is empty
    let cart = Order::find_or_create_cart(&user, connection).unwrap();
    let items = cart.items(&connection).unwrap();
    assert_eq!(items.len(), 0);

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = order.id;
    let response = cart::duplicate((database.connection.clone().into(), path, auth_user))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Cart now matches old order for items
    assert_ne!(cart.id, order.id);
    assert_eq!(cart.user_id, order.user_id);
    let order_items = order.items(connection).unwrap();
    let cart_items = cart.items(connection).unwrap();
    assert_eq!(order_items.len(), cart_items.len());
    let order_item = order_items
        .iter()
        .find(|i| i.item_type == OrderItemTypes::Tickets)
        .unwrap();
    let cart_item = cart_items
        .iter()
        .find(|i| i.item_type == OrderItemTypes::Tickets)
        .unwrap();
    assert_eq!(order_item.quantity, cart_item.quantity);
    assert_eq!(order_item.ticket_type_id, cart_item.ticket_type_id);
}

#[actix_rt::test]
async fn duplicate_fails_no_longer_available() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let event = database.create_event().with_tickets().with_ticket_pricing().finish();

    let order = database.create_order().quantity(5).for_event(&event).finish();
    let user = order.user(connection).unwrap();
    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);
    let tickets: Vec<Uuid> = order.tickets(None, connection).unwrap().iter().map(|t| t.id).collect();
    diesel::sql_query(
        r#"
        UPDATE orders
        SET created_at = $2, expires_at = $2
        WHERE id = $1;
        "#,
    )
    .bind::<sql_types::Uuid, _>(order.id)
    .bind::<sql_types::Timestamp, _>(dates::now().add_days(-7).finish())
    .execute(connection)
    .unwrap();
    diesel::sql_query(
        r#"
        UPDATE ticket_instances
        SET reserved_until = $2
        WHERE id = ANY($1);
        "#,
    )
    .bind::<sql_types::Array<sql_types::Uuid>, _>(tickets)
    .bind::<sql_types::Timestamp, _>(dates::now().add_days(-7).finish())
    .execute(connection)
    .unwrap();

    // Mark event as deleted to trigger failure to duplicate
    diesel::sql_query(
        r#"
        UPDATE events
        SET deleted_at = '1999-01-01 0:0:0'
        WHERE id = $1;
        "#,
    )
    .bind::<sql_types::Uuid, _>(event.id)
    .execute(connection)
    .unwrap();

    let cart = Order::find_or_create_cart(&user, connection).unwrap();
    let items = cart.items(&connection).unwrap();
    assert_eq!(items.len(), 0);

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = order.id;
    let response: HttpResponse = cart::duplicate((database.connection.clone().into(), path, auth_user))
        .await
        .into();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let expected_json = HttpResponse::UnprocessableEntity().json(json!({
        "error": "Order is invalid for duplication"
    }));
    let expected_text = unwrap_body_to_string(&expected_json).unwrap();
    let body = unwrap_body_to_string(&response).unwrap();
    assert_eq!(body, expected_text);
}

#[actix_rt::test]
async fn duplicate_fails_for_unowned_order() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let event = database.create_event().with_tickets().with_ticket_pricing().finish();

    let user = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);
    let order = database.create_order().quantity(5).for_event(&event).finish();
    let tickets: Vec<Uuid> = order.tickets(None, connection).unwrap().iter().map(|t| t.id).collect();
    diesel::sql_query(
        r#"
        UPDATE orders
        SET created_at = $2, expires_at = $2
        WHERE id = $1;
        "#,
    )
    .bind::<sql_types::Uuid, _>(order.id)
    .bind::<sql_types::Timestamp, _>(dates::now().add_days(-7).finish())
    .execute(connection)
    .unwrap();
    diesel::sql_query(
        r#"
        UPDATE ticket_instances
        SET reserved_until = $2
        WHERE id = ANY($1);
        "#,
    )
    .bind::<sql_types::Array<sql_types::Uuid>, _>(tickets)
    .bind::<sql_types::Timestamp, _>(dates::now().add_days(-7).finish())
    .execute(connection)
    .unwrap();

    let cart = Order::find_or_create_cart(&user, connection).unwrap();
    let items = cart.items(&connection).unwrap();
    assert_eq!(items.len(), 0);

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = order.id;
    let response: HttpResponse = cart::duplicate((database.connection.clone().into(), path, auth_user))
        .await
        .into();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let expected_json = HttpResponse::Forbidden().json(json!({
        "error": "This cart does not belong to you"
    }));
    let expected_text = unwrap_body_to_string(&expected_json).unwrap();
    let body = unwrap_body_to_string(&response).unwrap();
    assert_eq!(body, expected_text);
}

#[actix_rt::test]
async fn show() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let cart = Order::find_or_create_cart(&user, connection).unwrap();

    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);
    let response: HttpResponse = cart::show((database.connection.clone(), auth_user)).await.into();
    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    let cart_response: DisplayOrder = serde_json::from_str(&body).unwrap();
    assert_eq!(cart.id, cart_response.id);
}

#[actix_rt::test]
async fn show_no_cart() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);
    let response: HttpResponse = cart::show((database.connection, auth_user)).await.into();
    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(body, "{}");
}

#[actix_rt::test]
async fn show_expired_cart() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let cart = Order::find_or_create_cart(&user, connection).unwrap();
    let one_minute_ago = NaiveDateTime::from(Utc::now().naive_utc() - Duration::minutes(1));
    diesel::update(&cart)
        .set(orders::expires_at.eq(one_minute_ago))
        .get_result::<Order>(connection)
        .unwrap();

    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);
    let response: HttpResponse = cart::show((database.connection.clone().into(), auth_user)).await.into();
    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(body, "{}");
}

#[actix_rt::test]
async fn destroy() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let event = database.create_event().with_tickets().with_ticket_pricing().finish();

    let user = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);
    let ticket_type_id = event.ticket_types(true, None, connection).unwrap()[0].id;

    // Cart has existing items
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    cart.update_quantities(
        user.id,
        &vec![UpdateOrderItem {
            ticket_type_id,
            quantity: 10,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    let items = cart.items(&connection).unwrap();
    assert!(items.len() > 0);

    let response = cart::destroy((database.connection.clone().into(), auth_user))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Cart is cleared
    let cart = Order::find_or_create_cart(&user, connection).unwrap();
    assert!(cart.expires_at.is_none());
    let items = cart.items(&connection).unwrap();
    assert_eq!(0, items.len());
}

#[actix_rt::test]
async fn destroy_without_existing_cart() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);

    let response = cart::destroy((database.connection.clone().into(), auth_user))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Cart is cleared
    let cart = Order::find_cart_for_user(user.id, connection).unwrap().unwrap();
    assert!(cart.expires_at.is_none());
    let items = cart.items(&connection).unwrap();
    assert_eq!(0, items.len());
}

#[actix_rt::test]
async fn update() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let event = database.create_event().with_tickets().with_ticket_pricing().finish();

    let user = database.create_user().finish();
    let ticket_type_id = event.ticket_types(true, None, connection).unwrap()[0].id;

    let input = Json(cart::UpdateCartRequest {
        box_office_pricing: None,
        items: vec![cart::CartItem {
            ticket_type_id,
            quantity: 2,
            redemption_code: None,
        }],
        tracking_data: None,
    });

    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);
    let response = cart::update_cart((
        database.connection.clone().into(),
        input,
        auth_user,
        RequestInfo { user_agent: None },
    ))
    .await
    .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let cart = Order::find_cart_for_user(user.id, &connection).unwrap().unwrap();
    let items = cart.items(&connection).unwrap();
    let order_item = items.iter().find(|i| i.ticket_type_id == Some(ticket_type_id)).unwrap();

    let ticket_pricing = TicketPricing::find(order_item.ticket_pricing_id.unwrap(), connection).unwrap();
    assert_eq!(order_item.quantity, 2);
    let fee_item = order_item.find_fee_item(&connection).unwrap().unwrap();
    let fee_schedule_range = FeeScheduleRange::find(fee_item.fee_schedule_range_id.unwrap(), connection).unwrap();
    assert_eq!(fee_item.unit_price_in_cents, fee_schedule_range.fee_in_cents);
    assert_eq!(fee_item.quantity, 2);
    assert_eq!(order_item.unit_price_in_cents, ticket_pricing.price_in_cents);
}

#[actix_rt::test]
async fn update_with_draft_event() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let event = database
        .create_event()
        .with_status(EventStatus::Draft)
        .with_tickets()
        .with_ticket_pricing()
        .finish();

    let user = database.create_user().finish();
    let ticket_type_id = event.ticket_types(true, None, connection).unwrap()[0].id;

    let input = Json(cart::UpdateCartRequest {
        items: vec![cart::CartItem {
            ticket_type_id,
            quantity: 2,
            redemption_code: None,
        }],
        tracking_data: None,
        box_office_pricing: None,
    });

    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);
    let response = cart::update_cart((
        database.connection.clone().into(),
        input,
        auth_user,
        RequestInfo { user_agent: None },
    ))
    .await
    .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[actix_rt::test]
async fn update_multiple() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let event = database
        .create_event()
        .with_tickets()
        .with_ticket_type_count(2)
        .with_ticket_pricing()
        .finish();

    let user = database.create_user().finish();
    let ticket_types = event.ticket_types(true, None, connection).unwrap();
    let ticket_type_id = ticket_types[0].id;
    let ticket_type_id2 = ticket_types[1].id;
    let input = Json(cart::UpdateCartRequest {
        box_office_pricing: None,
        tracking_data: None,
        items: vec![
            cart::CartItem {
                ticket_type_id,
                quantity: 2,
                redemption_code: None,
            },
            cart::CartItem {
                ticket_type_id: ticket_type_id2,
                quantity: 3,
                redemption_code: None,
            },
        ],
    });
    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);
    let response = cart::update_cart((
        database.connection.clone().into(),
        input,
        auth_user,
        RequestInfo { user_agent: None },
    ))
    .await
    .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let cart = Order::find_cart_for_user(user.id, connection).unwrap().unwrap();
    let cart_items = cart
        .items(&connection)
        .unwrap()
        .into_iter()
        .filter(|c| c.parent_id.is_none())
        .collect::<Vec<OrderItem>>();
    let order_item = &cart_items[0];
    let order_item2 = &cart_items[1];
    let ticket_pricing = TicketPricing::find(order_item.ticket_pricing_id.unwrap(), connection).unwrap();
    let ticket_pricing2 = TicketPricing::find(order_item2.ticket_pricing_id.unwrap(), connection).unwrap();
    assert_eq!(order_item.quantity, 2);
    assert_eq!(order_item2.quantity, 3);
    let fee_item = order_item.find_fee_item(connection).unwrap().unwrap();

    let fee_item2 = order_item2.find_fee_item(connection).unwrap().unwrap();

    let fee_schedule_range = FeeScheduleRange::find(fee_item.fee_schedule_range_id.unwrap(), connection).unwrap();

    let fee_schedule_range2 = FeeScheduleRange::find(fee_item2.fee_schedule_range_id.unwrap(), connection).unwrap();
    assert_eq!(fee_item.unit_price_in_cents, fee_schedule_range.fee_in_cents);
    assert_eq!(fee_item.quantity, 2);
    assert_eq!(order_item.unit_price_in_cents, ticket_pricing.price_in_cents);
    assert_eq!(fee_item2.unit_price_in_cents, fee_schedule_range2.fee_in_cents);
    assert_eq!(fee_item2.quantity, 3);
    assert_eq!(order_item2.unit_price_in_cents, ticket_pricing2.price_in_cents);
}

#[actix_rt::test]
async fn add_with_increment() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let event = database.create_event().with_tickets().with_ticket_pricing().finish();

    let user = database.create_user().finish();
    let ticket_type = event.ticket_types(true, None, connection).unwrap().remove(0);
    let update_parameters = TicketTypeEditableAttributes {
        increment: Some(4),
        ..Default::default()
    };
    let ticket_type = ticket_type.update(update_parameters, None, connection).unwrap();
    let ticket_type_id = ticket_type.id;

    let input = Json(cart::UpdateCartRequest {
        box_office_pricing: None,
        tracking_data: None,
        items: vec![cart::CartItem {
            ticket_type_id,
            quantity: 4,
            redemption_code: None,
        }],
    });

    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);
    let response = cart::update_cart((
        database.connection.clone(),
        input,
        auth_user,
        RequestInfo { user_agent: None },
    ))
    .await
    .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let cart = Order::find_cart_for_user(user.id, connection).unwrap().unwrap();
    let order_item = cart
        .find_item_by_type(ticket_type_id, OrderItemTypes::Tickets, connection)
        .unwrap();
    let ticket_pricing = TicketPricing::find(order_item.ticket_pricing_id.unwrap(), connection).unwrap();
    assert_eq!(order_item.quantity, 4);
    let fee_item = order_item.find_fee_item(connection).unwrap().unwrap();
    let fee_schedule_range = FeeScheduleRange::find(fee_item.fee_schedule_range_id.unwrap(), connection).unwrap();
    assert_eq!(fee_item.unit_price_in_cents, fee_schedule_range.fee_in_cents);
    assert_eq!(fee_item.quantity, 4);
    assert_eq!(order_item.unit_price_in_cents, ticket_pricing.price_in_cents);
}

#[actix_rt::test]
async fn update_with_increment_failure_invalid_quantity() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let event = database.create_event().with_tickets().with_ticket_pricing().finish();

    let user = database.create_user().finish();
    let ticket_type = event.ticket_types(true, None, connection).unwrap().remove(0);
    let update_parameters = TicketTypeEditableAttributes {
        increment: Some(4),
        ..Default::default()
    };
    let ticket_type = ticket_type.update(update_parameters, None, connection).unwrap();
    let ticket_type_id = ticket_type.id;

    let input = Json(cart::UpdateCartRequest {
        box_office_pricing: None,
        tracking_data: None,
        items: vec![cart::CartItem {
            ticket_type_id,
            quantity: 2,
            redemption_code: None,
        }],
    });

    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);
    let response: HttpResponse = cart::update_cart((
        database.connection.clone().into(),
        input,
        auth_user,
        RequestInfo { user_agent: None },
    ))
    .await
    .into();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let validation_response = support::validation_response_from_response(&response).unwrap();
    let quantity = validation_response.fields.get("quantity").unwrap();
    assert_eq!(quantity[0].code, "quantity_invalid_increment");
    assert_eq!(
        &quantity[0].message.clone().unwrap().into_owned(),
        "Order item quantity invalid for ticket pricing increment"
    );
}

#[actix_rt::test]
async fn update_with_existing_cart() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let event = database.create_event().with_tickets().with_ticket_pricing().finish();

    let user = database.create_user().finish();
    let ticket_type_id = event.ticket_types(true, None, connection).unwrap()[0].id;
    let cart = Order::find_or_create_cart(&user, connection).unwrap();

    let input = Json(cart::UpdateCartRequest {
        box_office_pricing: None,
        tracking_data: None,
        items: vec![cart::CartItem {
            ticket_type_id,
            quantity: 2,
            redemption_code: None,
        }],
    });

    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);
    let response = cart::update_cart((
        database.connection.clone().into(),
        input,
        auth_user,
        RequestInfo { user_agent: None },
    ))
    .await
    .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let items = cart.items(connection).unwrap();
    let order_item = items.iter().find(|i| i.ticket_type_id == Some(ticket_type_id)).unwrap();

    let ticket_pricing = TicketPricing::find(order_item.ticket_pricing_id.unwrap(), connection).unwrap();
    assert_eq!(order_item.quantity, 2);
    let fee_item = order_item.find_fee_item(connection).unwrap().unwrap();
    let fee_schedule_range = FeeScheduleRange::find(fee_item.fee_schedule_range_id.unwrap(), connection).unwrap();
    assert_eq!(fee_item.unit_price_in_cents, fee_schedule_range.fee_in_cents);
    assert_eq!(fee_item.quantity, 2);
    assert_eq!(order_item.unit_price_in_cents, ticket_pricing.price_in_cents);
}

#[actix_rt::test]
async fn reduce() {
    let database = TestDatabase::new();
    let connection = &database.connection.get();
    let event = database.create_event().with_tickets().with_ticket_pricing().finish();
    let ticket_type_id = event.ticket_types(true, None, connection).unwrap()[0].id;
    let user = database.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    cart.update_quantities(
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
    let items = cart.items(connection).unwrap();
    let order_item = items.iter().find(|i| i.ticket_type_id == Some(ticket_type_id)).unwrap();
    let ticket_pricing = TicketPricing::find(order_item.ticket_pricing_id.unwrap(), connection).unwrap();
    assert_eq!(order_item.quantity, 10);
    let fee_item = order_item.find_fee_item(connection).unwrap().unwrap();
    let fee_schedule_range = FeeScheduleRange::find(fee_item.fee_schedule_range_id.unwrap(), connection).unwrap();
    assert_eq!(fee_item.unit_price_in_cents, fee_schedule_range.fee_in_cents);
    assert_eq!(fee_item.quantity, 10);
    assert_eq!(order_item.unit_price_in_cents, ticket_pricing.price_in_cents);

    let input = Json(cart::UpdateCartRequest {
        box_office_pricing: None,
        tracking_data: None,
        items: vec![cart::CartItem {
            ticket_type_id,
            quantity: 6,
            redemption_code: None,
        }],
    });

    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);
    let response = cart::update_cart((
        database.connection.clone().into(),
        input,
        auth_user,
        RequestInfo { user_agent: None },
    ))
    .await
    .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Contains additional item quantity so cart response still includes cart object
    let body = support::unwrap_body_to_string(&response).unwrap();
    let cart_response: DisplayOrder = serde_json::from_str(&body).unwrap();
    assert_eq!(cart.id, cart_response.id);
    let items = cart.items(connection).unwrap();
    let order_item = items.iter().find(|i| i.ticket_type_id == Some(ticket_type_id)).unwrap();

    assert_eq!(order_item.quantity, 6);
    let fee_item = order_item.find_fee_item(connection).unwrap().unwrap();
    assert_eq!(fee_item.unit_price_in_cents, fee_schedule_range.fee_in_cents);
    assert_eq!(fee_item.quantity, 6);
    assert_eq!(order_item.unit_price_in_cents, ticket_pricing.price_in_cents);
}

#[actix_rt::test]
async fn remove() {
    let database = TestDatabase::new();
    let connection = &database.connection.get();
    let event = database.create_event().with_tickets().with_ticket_pricing().finish();
    let ticket_type_id = event.ticket_types(true, None, connection).unwrap()[0].id;
    let user = database.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    cart.update_quantities(
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
    let items = cart.items(connection).unwrap();
    let order_item = items.iter().find(|i| i.ticket_type_id == Some(ticket_type_id)).unwrap();

    let ticket_pricing = TicketPricing::find(order_item.ticket_pricing_id.unwrap(), connection).unwrap();
    assert_eq!(order_item.quantity, 10);
    let fee_item = order_item.find_fee_item(connection).unwrap().unwrap();
    let fee_schedule_range = FeeScheduleRange::find(fee_item.fee_schedule_range_id.unwrap(), connection).unwrap();
    assert_eq!(fee_item.unit_price_in_cents, fee_schedule_range.fee_in_cents);
    assert_eq!(fee_item.quantity, 10);
    assert_eq!(order_item.unit_price_in_cents, ticket_pricing.price_in_cents);

    let input = Json(cart::UpdateCartRequest {
        box_office_pricing: None,
        tracking_data: None,
        items: vec![cart::CartItem {
            ticket_type_id,
            quantity: 0,
            redemption_code: None,
        }],
    });

    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);
    let response = cart::update_cart((
        database.connection.clone().into(),
        input,
        auth_user,
        RequestInfo { user_agent: None },
    ))
    .await
    .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Contains additional item quantity so cart response still includes cart object
    let body = support::unwrap_body_to_string(&response).unwrap();
    let cart_response: DisplayOrder = serde_json::from_str(&body).unwrap();
    assert_eq!(cart.id, cart_response.id);

    let items = cart.items(connection).unwrap();

    assert_eq!(items.len(), 0);
}

#[actix_rt::test]
async fn remove_with_increment() {
    let database = TestDatabase::new();
    let connection = &database.connection.get();
    let event = database.create_event().with_tickets().with_ticket_pricing().finish();
    let ticket_type = event.ticket_types(true, None, connection).unwrap().remove(0);
    let update_parameters = TicketTypeEditableAttributes {
        increment: Some(4),
        ..Default::default()
    };
    let ticket_type = ticket_type.update(update_parameters, None, connection).unwrap();
    let ticket_type_id = ticket_type.id;
    let user = database.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id,
            quantity: 12,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    let items = cart.items(connection).unwrap();
    let order_item = items.iter().find(|i| i.ticket_type_id == Some(ticket_type_id)).unwrap();

    let ticket_pricing = TicketPricing::find(order_item.ticket_pricing_id.unwrap(), connection).unwrap();
    assert_eq!(order_item.quantity, 12);
    let fee_item = order_item.find_fee_item(connection).unwrap().unwrap();
    let fee_schedule_range = FeeScheduleRange::find(fee_item.fee_schedule_range_id.unwrap(), connection).unwrap();
    assert_eq!(fee_item.unit_price_in_cents, fee_schedule_range.fee_in_cents);
    assert_eq!(fee_item.quantity, 12);
    assert_eq!(order_item.unit_price_in_cents, ticket_pricing.price_in_cents);

    let input = Json(cart::UpdateCartRequest {
        box_office_pricing: None,
        tracking_data: None,
        items: vec![cart::CartItem {
            ticket_type_id,
            quantity: 8,
            redemption_code: None,
        }],
    });

    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);
    let response = cart::update_cart((
        database.connection.clone().into(),
        input,
        auth_user,
        RequestInfo { user_agent: None },
    ))
    .await
    .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Contains additional item quantity so cart response still includes cart object
    let body = support::unwrap_body_to_string(&response).unwrap();
    let cart_response: DisplayOrder = serde_json::from_str(&body).unwrap();
    assert_eq!(cart.id, cart_response.id);
    let items = cart.items(connection).unwrap();
    let order_item = items.iter().find(|i| i.ticket_type_id == Some(ticket_type_id)).unwrap();
    assert_eq!(order_item.quantity, 8);
    let fee_item = order_item.find_fee_item(connection).unwrap().unwrap();
    assert_eq!(fee_item.unit_price_in_cents, fee_schedule_range.fee_in_cents);
    assert_eq!(fee_item.quantity, 8);
    assert_eq!(order_item.unit_price_in_cents, ticket_pricing.price_in_cents);
}

#[actix_rt::test]
async fn remove_with_increment_failure_invalid_quantity() {
    let database = TestDatabase::new();
    let connection = &database.connection.get();
    let event = database.create_event().with_tickets().with_ticket_pricing().finish();
    let ticket_type = event.ticket_types(true, None, connection).unwrap().remove(0);
    let update_parameters = TicketTypeEditableAttributes {
        increment: Some(4),
        ..Default::default()
    };
    let ticket_type = ticket_type.update(update_parameters, None, connection).unwrap();
    let ticket_type_id = ticket_type.id;
    let user = database.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id,
            quantity: 12,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    let items = cart.items(connection).unwrap();
    let order_item = items.iter().find(|i| i.ticket_type_id == Some(ticket_type_id)).unwrap();

    let ticket_pricing = TicketPricing::find(order_item.ticket_pricing_id.unwrap(), connection).unwrap();
    assert_eq!(order_item.quantity, 12);
    let fee_item = order_item.find_fee_item(connection).unwrap().unwrap();
    let fee_schedule_range = FeeScheduleRange::find(fee_item.fee_schedule_range_id.unwrap(), connection).unwrap();
    assert_eq!(fee_item.unit_price_in_cents, fee_schedule_range.fee_in_cents);
    assert_eq!(fee_item.quantity, 12);
    assert_eq!(order_item.unit_price_in_cents, ticket_pricing.price_in_cents);

    let input = Json(cart::UpdateCartRequest {
        box_office_pricing: None,
        tracking_data: None,
        items: vec![cart::CartItem {
            ticket_type_id,
            quantity: 5,
            redemption_code: None,
        }],
    });

    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);
    let response: HttpResponse = cart::update_cart((
        database.connection.clone().into(),
        input,
        auth_user,
        RequestInfo { user_agent: None },
    ))
    .await
    .into();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let validation_response = support::validation_response_from_response(&response).unwrap();
    let quantity = validation_response.fields.get("quantity").unwrap();
    assert_eq!(quantity[0].code, "quantity_invalid_increment");
    assert_eq!(
        &quantity[0].message.clone().unwrap().into_owned(),
        "Order item quantity invalid for ticket pricing increment"
    );
}

#[actix_rt::test]
async fn checkout_external() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let event = database.create_event().with_tickets().with_ticket_pricing().finish();

    let user = database.create_user().finish();

    let order = database.create_cart().for_user(&user).for_event(&event).finish();
    let request = TestRequest::create();

    let input = Json(cart::CheckoutCartRequest {
        tracking_data: None,
        method: PaymentRequest::External {
            reference: Some("TestRef".to_string()),
            external_payment_type: ExternalPaymentType::Voucher,
            first_name: "First".to_string(),
            last_name: "Last".to_string(),
            email: Some("easdf@test.com".to_string()),
            phone: None,
            note: Some("Example note".to_string()),
        },
    });

    // Must be admin to check out external
    let user = support::create_auth_user_from_user(&user, Roles::Admin, None, &database);

    let response = cart::checkout((
        database.connection.clone().into(),
        input,
        user.clone(),
        request.extract_state().await,
        RequestInfo { user_agent: None },
    ))
    .await
    .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Reload order
    let order = Order::find(order.id, connection).unwrap();
    assert_eq!(order.status, OrderStatus::Paid);
    assert_eq!(order.external_payment_type, Some(ExternalPaymentType::Voucher));

    // Confirm note saved correctly
    let note = &Note::find_for_table(Tables::Orders, order.id, true, 0, 1, connection)
        .unwrap()
        .data[0];
    assert_eq!(user.id(), note.created_by);
    assert_eq!("Example note".to_string(), note.note);
}

#[actix_rt::test]
async fn checkout_external_with_free_cart() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let event = database.create_event().with_tickets().with_ticket_pricing().finish();

    let user = database.create_user().finish();
    let order = database
        .create_cart()
        .with_free_items()
        .for_user(&user)
        .for_event(&event)
        .finish();
    let request = TestRequest::create();

    let input = Json(cart::CheckoutCartRequest {
        tracking_data: None,
        method: PaymentRequest::External {
            reference: Some("TestRef".to_string()),
            external_payment_type: ExternalPaymentType::Cash,
            first_name: "First".to_string(),
            last_name: "Last".to_string(),
            email: Some("easdf@test.com".to_string()),
            phone: None,
            note: None,
        },
    });

    // Must be admin to check out external
    let user = support::create_auth_user_from_user(&user, Roles::Admin, None, &database);

    let response = cart::checkout((
        database.connection.clone().into(),
        input,
        user,
        request.extract_state().await,
        RequestInfo { user_agent: None },
    ))
    .await
    .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Reload order
    let order = Order::find(order.id, connection).unwrap();
    assert_eq!(order.status, OrderStatus::Paid);

    // Free payment
    let payments = order.payments(connection).unwrap();
    assert_eq!(1, payments.len());
    let payment = &payments[0];
    assert_eq!(payment.payment_method, PaymentMethods::Free);
    assert_eq!(payment.provider, PaymentProviders::External);
}

#[actix_rt::test]
async fn checkout_paid_fails_with_free_cart() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let event = database.create_event().with_tickets().with_ticket_pricing().finish();

    let user = database.create_user().finish();
    let order = database
        .create_cart()
        .with_free_items()
        .for_user(&user)
        .for_event(&event)
        .finish();
    let request = TestRequest::create();

    let input = Json(cart::CheckoutCartRequest {
        tracking_data: None,
        method: PaymentRequest::Card {
            token: "abc".into(),
            provider: PaymentProviders::Stripe,
            save_payment_method: false,
            set_default: false,
        },
    });

    // Must be admin to check out external
    let user = support::create_auth_user_from_user(&user, Roles::Admin, None, &database);

    let response: HttpResponse = cart::checkout((
        database.connection.clone().into(),
        input,
        user,
        request.extract_state().await,
        RequestInfo { user_agent: None },
    ))
    .await
    .into();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let expected_json = HttpResponse::UnprocessableEntity().json(json!({
        "error": "Could not complete this cart; only paid orders require payment processing"
    }));
    let expected_text = unwrap_body_to_string(&expected_json).unwrap();
    let body = unwrap_body_to_string(&response).unwrap();
    assert_eq!(body, expected_text);

    // Reload order
    let order = Order::find(order.id, connection).unwrap();
    assert_eq!(order.status, OrderStatus::Draft);
}

#[actix_rt::test]
async fn checkout_free() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let event = database.create_event().with_tickets().with_ticket_pricing().finish();

    let user = database.create_user().finish();

    let order = database
        .create_cart()
        .with_free_items()
        .for_user(&user)
        .for_event(&event)
        .finish();
    let request = TestRequest::create();

    let input = Json(cart::CheckoutCartRequest {
        tracking_data: None,
        method: PaymentRequest::Free,
    });

    let user = support::create_auth_user_from_user(&user, Roles::User, None, &database);

    let response = cart::checkout((
        database.connection.clone().into(),
        input,
        user,
        request.extract_state().await,
        RequestInfo { user_agent: None },
    ))
    .await
    .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Reload order
    let order = Order::find(order.id, connection).unwrap();
    assert_eq!(order.status, OrderStatus::Paid);

    // Free payment
    let payments = order.payments(connection).unwrap();
    assert_eq!(1, payments.len());
    let payment = &payments[0];
    assert_eq!(payment.payment_method, PaymentMethods::Free);
    assert_eq!(payment.provider, PaymentProviders::Free);
}

#[actix_rt::test]
async fn checkout_free_for_paid_items() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let event = database.create_event().with_tickets().with_ticket_pricing().finish();

    let user = database.create_user().finish();

    let order = database.create_cart().for_user(&user).for_event(&event).finish();
    let request = TestRequest::create();

    let input = Json(cart::CheckoutCartRequest {
        tracking_data: None,
        method: PaymentRequest::Free,
    });

    let user = support::create_auth_user_from_user(&user, Roles::User, None, &database);

    let response: HttpResponse = cart::checkout((
        database.connection.clone().into(),
        input,
        user,
        request.extract_state().await,
        RequestInfo { user_agent: None },
    ))
    .await
    .into();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let expected_json = HttpResponse::UnprocessableEntity().json(json!({
        "error": "Could not use free payment method this cart because it has a total greater than zero"
    }));
    let expected_text = unwrap_body_to_string(&expected_json).unwrap();
    let body = unwrap_body_to_string(&response).unwrap();
    assert_eq!(body, expected_text);

    // Reload order
    let order = Order::find(order.id, connection).unwrap();
    assert_eq!(order.status, OrderStatus::Draft);
}

#[actix_rt::test]
async fn clear_invalid_items() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let event = database.create_event().with_tickets().with_ticket_pricing().finish();
    let ticket_type = event.ticket_types(true, None, connection).unwrap().remove(0);

    let user = database.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();

    // Order item with past reserved until date
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
    let items = cart.items(connection).unwrap();
    let order_item = items.iter().find(|i| i.ticket_type_id == Some(ticket_type.id)).unwrap();
    let one_minute_ago = NaiveDateTime::from(Utc::now().naive_utc() - Duration::minutes(1));
    diesel::update(ticket_instances::table.filter(ticket_instances::order_item_id.eq(order_item.id)))
        .set((ticket_instances::reserved_until.eq(one_minute_ago),))
        .execute(connection)
        .unwrap();

    // Must be admin to check out external
    let user = support::create_auth_user_from_user(&user, Roles::Admin, None, &database);

    // Not currently valid
    assert!(!cart.items_valid_for_purchase(connection).unwrap());

    let response: HttpResponse = cart::clear_invalid_items((database.connection.clone().into(), user))
        .await
        .into();
    assert_eq!(response.status(), StatusCode::OK);

    // Cart no longer contains invalid items
    assert!(cart.items_valid_for_purchase(connection).unwrap());
}

#[actix_rt::test]
async fn checkout_fails_for_invalid_items() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let event = database.create_event().with_tickets().with_ticket_pricing().finish();
    let ticket_type = event.ticket_types(true, None, connection).unwrap().remove(0);

    let user = database.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();

    // Order item with past reserved until date
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
    let items = cart.items(connection).unwrap();
    let order_item = items.iter().find(|i| i.ticket_type_id == Some(ticket_type.id)).unwrap();
    let one_minute_ago = NaiveDateTime::from(Utc::now().naive_utc() - Duration::minutes(1));
    diesel::update(ticket_instances::table.filter(ticket_instances::order_item_id.eq(order_item.id)))
        .set((ticket_instances::reserved_until.eq(one_minute_ago),))
        .execute(connection)
        .unwrap();

    let request = TestRequest::create();

    let input = Json(cart::CheckoutCartRequest {
        tracking_data: None,
        method: PaymentRequest::External {
            reference: Some("TestRef".to_string()),
            external_payment_type: ExternalPaymentType::Cash,
            first_name: "First".to_string(),
            last_name: "Last".to_string(),
            email: Some("easdf@test.com".to_string()),
            phone: None,
            note: None,
        },
    });

    // Must be admin to check out external
    let user = support::create_auth_user_from_user(&user, Roles::Admin, None, &database);

    let response: HttpResponse = cart::checkout((
        database.connection.clone().into(),
        input,
        user,
        request.extract_state().await,
        RequestInfo { user_agent: None },
    ))
    .await
    .into();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let expected_json = HttpResponse::UnprocessableEntity().json(json!({
        "error": "Could not complete this checkout because it contains invalid order items"
    }));
    let expected_text = unwrap_body_to_string(&expected_json).unwrap();
    let body = unwrap_body_to_string(&response).unwrap();
    assert_eq!(body, expected_text);

    // Reload cart
    let cart = Order::find(cart.id, connection).unwrap();
    assert_eq!(cart.status, OrderStatus::Draft);
}
#[actix_rt::test]
async fn checkout_provider_globee() {
    let database = TestDatabase::new();
    let conn = database.connection.get();
    let event = database.create_event().with_tickets().with_ticket_pricing().finish();

    let user = database.create_user().finish();

    database.create_cart().for_user(&user).for_event(&event).finish();
    let request = TestRequest::create();

    let input = Json(cart::CheckoutCartRequest {
        tracking_data: None,
        method: PaymentRequest::Provider {
            provider: PaymentProviders::Globee,
        },
    });

    // Must be admin to check out external
    let user = support::create_auth_user_from_user(&user, Roles::Admin, None, &database);

    let response = cart::checkout((
        database.connection.clone().into(),
        input,
        user.clone(),
        request.extract_state().await,
        RequestInfo { user_agent: None },
    ))
    .await
    .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = unwrap_body_to_string(&response).unwrap();
    let order: DisplayOrder = serde_json::from_str(body).unwrap();
    assert_eq!(order.status, OrderStatus::Draft);

    // User accepts
    let db_payment = &Order::find(order.id, conn).unwrap().payments(conn).unwrap()[0];

    let url = format!(
        "/payments/callback/{}/{}?success=true",
        db_payment.url_nonce.clone().unwrap(),
        order.id
    );
    let request = TestRequest::create_with_uri_custom_params(&url, vec!["nonce", "id"]);
    let query = Query::<controllers::payments::QueryParams>::extract(&request.request)
        .await
        .unwrap();
    let mut path = Path::<controllers::payments::PathParams>::extract(&request.request)
        .await
        .unwrap();
    path.nonce = db_payment.url_nonce.clone().unwrap();
    path.id = order.id;

    let response = controllers::payments::callback((
        query,
        path,
        database.connection.clone().into(),
        request.extract_state().await,
        OptionalUser(Some(user)),
    ))
    .await
    .unwrap();

    assert_eq!(response.status(), StatusCode::FOUND);

    let order = Order::find(order.id, conn).unwrap();
    assert_eq!(order.status, OrderStatus::PendingPayment);

    //let request = TestRequest::create_with_uri("/ipns/globee");

    let ipn = GlobeeIpnRequest {
        id: "".to_string(),
        status: Some("confirmed".to_string()),
        total: None,
        adjusted_total: None,
        currency: None,
        custom_payment_id: Some(order.id.to_string()),
        custom_store_reference: None,
        callback_data: None,
        customer: Customer {
            name: None,
            email: Email::new("something@test.com".to_string()),
        },
        payment_details: PaymentDetails {
            currency: Some("BTC".to_string()),
            received_amount: Some(order.calculate_total(conn).unwrap() as f64 / 100f64),
            received_difference: Some(0.0),
        },
        redirect_url: None,
        success_url: None,
        cancel_url: None,
        ipn_url: None,
        notification_email: None,
        confirmation_speed: None,
        expires_at: None,
        created_at: None,
    };

    let response = controllers::ipns::globee((Json(ipn), database.connection.clone().into()))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let mut domain_actions = DomainAction::find_pending(Some(DomainActionTypes::PaymentProviderIPN), conn).unwrap();

    assert_eq!(domain_actions.len(), 1);

    let domain_action = domain_actions.remove(0);
    assert_eq!(domain_action.main_table_id, Some(order.id));

    let processor = ProcessPaymentIPNExecutor::new(&request.config);
    processor
        .perform_job(&domain_action, &database.connection.clone())
        .unwrap();

    let order = Order::find(order.id, conn).unwrap();
    assert_eq!(order.status, OrderStatus::Paid);
}
