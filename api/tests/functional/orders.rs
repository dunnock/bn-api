use std::collections::HashMap;

use actix_web::{FromRequest, http::StatusCode, HttpResponse, web::{Path, Query}};
use chrono::prelude::*;
use diesel;
use diesel::prelude::*;
use serde_json;
use uuid::Uuid;

use crate::functional::base;
use crate::support;
use crate::support::database::TestDatabase;
use crate::support::test_request::TestRequest;
use bigneon_api::controllers::orders::{self, *};
use bigneon_api::extractors::Json;
use bigneon_api::models::PathParameters;
use bigneon_db::models::*;
use bigneon_db::schema;

#[test]
pub async fn show() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let mut order = database.create_order().for_user(&user).finish();
    let conn = database.connection.get();
    let total = order.calculate_total(conn).unwrap();
    order
        .add_external_payment(
            Some("test".to_string()),
            ExternalPaymentType::CreditCard,
            user.id,
            total,
            conn,
        )
        .unwrap();
    assert_eq!(order.status, OrderStatus::Paid);

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = order.id;

    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);
    let response: HttpResponse = orders::show((database.connection.clone(), path, auth_user)).await.into();
    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    let found_order: DisplayOrder = serde_json::from_str(&body).unwrap();
    assert_eq!(found_order.id, order.id);
}

#[test]
pub async fn show_for_box_office_purchased_user() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let box_office_user = database.create_user().finish();
    let mut order = database
        .create_order()
        .for_user(&box_office_user)
        .on_behalf_of_user(&user)
        .finish();
    let conn = database.connection.get();
    let total = order.calculate_total(conn).unwrap();
    order
        .add_external_payment(
            Some("test".to_string()),
            ExternalPaymentType::CreditCard,
            user.id,
            total,
            conn,
        )
        .unwrap();
    assert_eq!(order.status, OrderStatus::Paid);

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = order.id;

    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);
    let response: HttpResponse = orders::show((database.connection.clone(), path, auth_user)).await.into();
    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    let found_order: DisplayOrder = serde_json::from_str(&body).unwrap();
    assert_eq!(found_order.id, order.id);
}

#[test]
pub async fn resend_confirmation_on_draft_order() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let user2 = database.create_user().finish();
    let event = database
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let order = database.create_order().for_event(&event).for_user(&user2).finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::OrgOwner, Some(&organization), &database);
    assert_eq!(order.status, OrderStatus::Draft);

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = order.id;

    let response: HttpResponse = orders::resend_confirmation((
        database.connection.clone(),
        path,
        auth_user,
        test_request.extract_state(),
    ))
    .into();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[cfg(test)]
mod resend_confirmation_tests {
    use super::*;
    #[test]
    fn resend_confirmation_org_member() {
        base::orders::resend_confirmation(Roles::OrgMember, true);
    }
    #[test]
    fn resend_confirmation_admin() {
        base::orders::resend_confirmation(Roles::Admin, true);
    }
    #[test]
    fn resend_confirmation_user() {
        base::orders::resend_confirmation(Roles::User, false);
    }
    #[test]
    fn resend_confirmation_org_owner() {
        base::orders::resend_confirmation(Roles::OrgOwner, true);
    }
    #[test]
    fn resend_confirmation_door_person() {
        base::orders::resend_confirmation(Roles::DoorPerson, false);
    }
    #[test]
    fn resend_confirmation_promoter() {
        base::orders::resend_confirmation(Roles::Promoter, false);
    }
    #[test]
    fn resend_confirmation_promoter_read_only() {
        base::orders::resend_confirmation(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn resend_confirmation_org_admin() {
        base::orders::resend_confirmation(Roles::OrgAdmin, true);
    }
    #[test]
    fn resend_confirmation_box_office() {
        base::orders::resend_confirmation(Roles::OrgBoxOffice, true);
    }
}

#[cfg(test)]
mod activity_tests {
    use super::*;
    #[test]
    fn activity_org_member() {
        base::orders::activity(Roles::OrgMember, true);
    }
    #[test]
    fn activity_admin() {
        base::orders::activity(Roles::Admin, true);
    }
    #[test]
    fn activity_user() {
        base::orders::activity(Roles::User, false);
    }
    #[test]
    fn activity_org_owner() {
        base::orders::activity(Roles::OrgOwner, true);
    }
    #[test]
    fn activity_door_person() {
        base::orders::activity(Roles::DoorPerson, true);
    }
    #[test]
    fn activity_promoter() {
        base::orders::activity(Roles::Promoter, true);
    }
    #[test]
    fn activity_promoter_read_only() {
        base::orders::activity(Roles::PromoterReadOnly, true);
    }
    #[test]
    fn activity_org_admin() {
        base::orders::activity(Roles::OrgAdmin, true);
    }
    #[test]
    fn activity_box_office() {
        base::orders::activity(Roles::OrgBoxOffice, true);
    }
}

#[cfg(test)]
mod show_other_user_order_tests {
    use super::*;

    #[test]
    fn show_other_user_order_org_member() {
        base::orders::show_other_user_order(Roles::OrgMember, true);
    }
    #[test]
    fn show_other_user_order_admin() {
        base::orders::show_other_user_order(Roles::Admin, true);
    }
    #[test]
    fn show_other_user_order_user() {
        base::orders::show_other_user_order(Roles::User, false);
    }
    #[test]
    fn show_other_user_order_org_owner() {
        base::orders::show_other_user_order(Roles::OrgOwner, true);
    }
    #[test]
    fn show_other_user_order_door_person() {
        base::orders::show_other_user_order(Roles::DoorPerson, true);
    }
    #[test]
    fn show_other_user_order_promoter() {
        base::orders::show_other_user_order(Roles::Promoter, true);
    }
    #[test]
    fn show_other_user_order_promoter_read_only() {
        base::orders::show_other_user_order(Roles::PromoterReadOnly, true);
    }
    #[test]
    fn show_other_user_order_org_admin() {
        base::orders::show_other_user_order(Roles::OrgAdmin, true);
    }
    #[test]
    fn show_other_user_order_box_office() {
        base::orders::show_other_user_order(Roles::OrgBoxOffice, true);
    }
}

#[cfg(test)]
mod show_other_user_order_not_matching_users_organization_tests {
    use super::*;

    #[test]
    fn show_other_user_order_not_matching_users_organization_org_member() {
        base::orders::show_other_user_order_not_matching_users_organization(Roles::OrgMember, false);
    }
    #[test]
    fn show_other_user_order_not_matching_users_organization_admin() {
        base::orders::show_other_user_order_not_matching_users_organization(Roles::Admin, true);
    }
    #[test]
    fn show_other_user_order_not_matching_users_organization_user() {
        base::orders::show_other_user_order_not_matching_users_organization(Roles::User, false);
    }
    #[test]
    fn show_other_user_order_not_matching_users_organization_org_owner() {
        base::orders::show_other_user_order_not_matching_users_organization(Roles::OrgOwner, false);
    }
    #[test]
    fn show_other_user_order_not_matching_users_organization_door_person() {
        base::orders::show_other_user_order_not_matching_users_organization(Roles::DoorPerson, false);
    }
    #[test]
    fn show_other_user_order_not_matching_users_organization_promoter() {
        base::orders::show_other_user_order_not_matching_users_organization(Roles::Promoter, false);
    }
    #[test]
    fn show_other_user_order_not_matching_users_organization_promoter_read_only() {
        base::orders::show_other_user_order_not_matching_users_organization(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn show_other_user_order_not_matching_users_organization_org_admin() {
        base::orders::show_other_user_order_not_matching_users_organization(Roles::OrgAdmin, false);
    }
    #[test]
    fn show_other_user_order_not_matching_users_organization_box_office() {
        base::orders::show_other_user_order_not_matching_users_organization(Roles::OrgBoxOffice, false);
    }
}

#[test]
pub async fn show_for_draft_returns_forbidden() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let order = database.create_order().for_user(&user).finish();
    assert_eq!(order.status, OrderStatus::Draft);

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = order.id;

    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);
    let response: HttpResponse = orders::show((database.connection.clone(), path, auth_user)).await.into();
    support::expects_forbidden(&response, Some("You do not have access to this order"));
}

#[test]
pub async fn index() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let mut order1 = database.create_order().for_user(&user).finish();
    let date1 = NaiveDate::from_ymd(2017, 7, 8).and_hms(9, 10, 11);
    let date2 = NaiveDate::from_ymd(2017, 7, 9).and_hms(9, 10, 11);
    let conn = database.connection.get();
    let total = order1.calculate_total(conn).unwrap();
    order1
        .add_external_payment(
            Some("test".to_string()),
            ExternalPaymentType::CreditCard,
            user.id,
            total,
            conn,
        )
        .unwrap();
    order1 = diesel::update(&order1)
        .set(schema::orders::order_date.eq(date1))
        .get_result(conn)
        .unwrap();
    let mut order2 = database.create_order().for_user(&user).finish();
    let total = order2.calculate_total(conn).unwrap();
    order2
        .add_external_payment(
            Some("test".to_string()),
            ExternalPaymentType::CreditCard,
            user.id,
            total - 100,
            conn,
        )
        .unwrap();
    order2 = diesel::update(&order2)
        .set(schema::orders::order_date.eq(date2))
        .get_result(conn)
        .unwrap();

    assert_eq!(order1.status, OrderStatus::Paid);
    assert_eq!(order2.status, OrderStatus::Draft);

    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);
    let test_request = TestRequest::create_with_uri(&format!("/?"));
    let query_parameters = Query::<PagingParameters>::extract(&test_request.request).await.unwrap();
    let response: HttpResponse = orders::index((database.connection.clone(), query_parameters, auth_user)).await.into();

    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();

    let orders: Payload<DisplayOrder> = serde_json::from_str(body).unwrap();
    assert_eq!(orders.data.len(), 1);
    let order_ids: Vec<Uuid> = orders.data.iter().map(|o| o.id).collect();
    assert_eq!(order_ids, vec![order1.id]);
}

#[cfg(test)]
mod details_tests {
    use super::*;

    #[test]
    fn details_org_member() {
        base::orders::details(Roles::OrgMember, true);
    }
    #[test]
    fn details_admin() {
        base::orders::details(Roles::Admin, true);
    }
    #[test]
    fn details_user() {
        base::orders::details(Roles::User, false);
    }
    #[test]
    fn details_org_owner() {
        base::orders::details(Roles::OrgOwner, true);
    }
    #[test]
    fn details_door_person() {
        base::orders::details(Roles::DoorPerson, true);
    }
    #[test]
    fn details_promoter() {
        base::orders::details(Roles::Promoter, true);
    }
    #[test]
    fn details_promoter_read_only() {
        base::orders::details(Roles::PromoterReadOnly, true);
    }
    #[test]
    fn details_org_admin() {
        base::orders::details(Roles::OrgAdmin, true);
    }
    #[test]
    fn details_box_office() {
        base::orders::details(Roles::OrgBoxOffice, true);
    }
}

#[cfg(test)]
mod refund_tests {
    use super::*;

    #[test]
    fn refund_org_member() {
        base::orders::refund(Roles::OrgMember, false, true);
    }
    #[test]
    fn refund_admin() {
        base::orders::refund(Roles::Admin, false, true);
    }
    #[test]
    fn refund_super() {
        base::orders::refund(Roles::Super, false, true);
    }
    #[test]
    fn refund_user() {
        base::orders::refund(Roles::User, false, false);
    }
    #[test]
    fn refund_org_owner() {
        base::orders::refund(Roles::OrgOwner, false, true);
    }
    #[test]
    fn refund_door_person() {
        base::orders::refund(Roles::DoorPerson, false, false);
    }
    #[test]
    fn refund_promoter() {
        base::orders::refund(Roles::Promoter, false, false)
    }
    #[test]
    fn refund_promoter_read_only() {
        base::orders::refund(Roles::PromoterReadOnly, false, false);
    }
    #[test]
    fn refund_org_admin() {
        base::orders::refund(Roles::OrgAdmin, false, true);
    }
    #[test]
    fn refund_box_office() {
        base::orders::refund(Roles::OrgBoxOffice, false, false);
    }
    #[test]
    fn refund_override_org_member() {
        base::orders::refund(Roles::OrgMember, true, false);
    }
    #[test]
    fn refund_override_admin() {
        base::orders::refund(Roles::Admin, true, true);
    }
    #[test]
    fn refund_override_super() {
        base::orders::refund(Roles::Super, true, true);
    }
    #[test]
    fn refund_override_user() {
        base::orders::refund(Roles::User, true, false);
    }
    #[test]
    fn refund_override_org_owner() {
        base::orders::refund(Roles::OrgOwner, true, false);
    }
    #[test]
    fn refund_override_door_person() {
        base::orders::refund(Roles::DoorPerson, true, false);
    }
    #[test]
    fn refund_override_promoter() {
        base::orders::refund(Roles::Promoter, true, false)
    }
    #[test]
    fn refund_override_promoter_read_only() {
        base::orders::refund(Roles::PromoterReadOnly, true, false);
    }
    #[test]
    fn refund_override_org_admin() {
        base::orders::refund(Roles::OrgAdmin, true, false);
    }
    #[test]
    fn refund_override_box_office() {
        base::orders::refund(Roles::OrgBoxOffice, true, false);
    }
}

#[test]
pub async fn refund_for_non_refundable_tickets() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let organization = database.create_organization().with_event_fee().with_fees().finish();
    let event = database
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let user = database.create_user().finish();
    let user2 = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::OrgOwner, Some(&organization), &database);
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 2,
            redemption_code: None,
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

    let items = cart.items(&connection).unwrap();

    let order_item = items.iter().find(|i| i.ticket_type_id == Some(ticket_type.id)).unwrap();

    let event_fee_item = items.iter().find(|i| i.item_type == OrderItemTypes::EventFees).unwrap();

    let tickets = TicketInstance::find_for_order_item(order_item.id, connection).unwrap();
    let ticket = &tickets[0];

    // Transfer the first ticket away
    TicketInstance::direct_transfer(
        &user,
        &vec![ticket.id],
        "example@tari.com",
        TransferMessageType::Email,
        user2.id,
        connection,
    )
    .unwrap();

    // Refund first ticket and event fee (leaving one ticket + one fee item for that ticket)
    let refund_items = vec![
        RefundItemRequest {
            order_item_id: order_item.id,
            ticket_instance_id: Some(ticket.id),
        },
        RefundItemRequest {
            order_item_id: event_fee_item.id,
            ticket_instance_id: None,
        },
    ];
    let json = Json(RefundAttributes {
        items: refund_items,
        reason: None,
        manual_override: false,
    });

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = cart.id;
    let response: HttpResponse = orders::refund((
        database.connection.clone(),
        path,
        json,
        auth_user,
        test_request.extract_state(),
    ))
    .into();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // Reload ticket
    let ticket = TicketInstance::find(ticket.id, connection).unwrap();
    assert!(ticket.order_item_id.is_some());

    // Reload order item
    let order_item = OrderItem::find_in_order(cart.id, order_item.id, connection).unwrap();
    assert_eq!(order_item.refunded_quantity, 0);

    // Reload fee item
    let fee_item = order_item.find_fee_item(connection).unwrap().unwrap();
    assert_eq!(fee_item.refunded_quantity, 0);

    // Reload event fee
    let event_fee_item = OrderItem::find(event_fee_item.id, connection).unwrap();
    assert_eq!(event_fee_item.refunded_quantity, 0);
}

#[test]
pub async fn refund_hold_ticket() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let organization = database.create_organization().finish();
    let event = database
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let hold = database.create_hold().with_quantity(1).with_event(&event).finish();
    let user = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::OrgOwner, Some(&organization), &database);
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
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

    let items = cart.items(&connection).unwrap();
    let order_item = items.iter().find(|i| i.ticket_type_id == Some(ticket_type.id)).unwrap();
    let fee_item = order_item.find_fee_item(connection).unwrap().unwrap();
    let discount_item = order_item.find_discount_item(connection).unwrap().unwrap();
    let tickets = TicketInstance::find_for_order_item(order_item.id, connection).unwrap();
    let ticket = &tickets[0];

    // Refund first ticket and event fee (leaving one ticket + one fee item for that ticket)
    let refund_items = vec![RefundItemRequest {
        order_item_id: order_item.id,
        ticket_instance_id: Some(ticket.id),
    }];
    let json = Json(RefundAttributes {
        items: refund_items,
        reason: Some("Purchased by mistake".to_string()),
        manual_override: false,
    });

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = cart.id;
    let response: HttpResponse = orders::refund((
        database.connection.clone(),
        path,
        json,
        auth_user,
        test_request.extract_state(),
    ))
    .into();

    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    let refund_response: RefundResponse = serde_json::from_str(&body).unwrap();
    let expected_refund_amount =
        order_item.unit_price_in_cents + fee_item.unit_price_in_cents + discount_item.unit_price_in_cents;
    assert_eq!(refund_response.amount_refunded, expected_refund_amount);

    let mut expected_refund_breakdown = HashMap::new();
    expected_refund_breakdown.insert(PaymentMethods::External, expected_refund_amount);
    assert_eq!(refund_response.refund_breakdown, expected_refund_breakdown);

    // Reload ticket
    let ticket = TicketInstance::find(ticket.id, connection).unwrap();
    assert!(ticket.order_item_id.is_none());
    assert_eq!(ticket.status, TicketInstanceStatus::Available);

    // Confirm hold ticket is available for purchase again via new cart
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

    // Reload ticket
    let ticket = TicketInstance::find(ticket.id, connection).unwrap();
    assert!(ticket.order_item_id.is_some());
    assert_eq!(ticket.status, TicketInstanceStatus::Reserved);
    assert_ne!(Some(order_item.id), ticket.order_item_id);
}
