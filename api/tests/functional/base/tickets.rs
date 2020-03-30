use crate::support;
use crate::support::database::TestDatabase;
use crate::support::test_request::TestRequest;
use actix_web::{http::StatusCode, web::Path, FromRequest, HttpResponse};
use api::controllers::tickets::{self, ShowTicketResponse};
use api::extractors::*;
use api::models::PathParameters;
use db::models::*;
use db::utils::dates;
use serde_json;

pub async fn show_other_user_ticket(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let request = TestRequest::create();
    let organization = database.create_organization().finish();
    let event = database
        .create_event()
        .with_organization(&organization)
        .with_ticket_pricing()
        .finish();
    let user2 = database.create_user().finish();
    let ticket_type = event.ticket_types(true, None, connection).unwrap().remove(0);
    let ticket_pricing = ticket_type.current_ticket_pricing(false, connection).unwrap();
    let cart = Order::find_or_create_cart(&user2, connection).unwrap();
    let ticket = database.create_purchased_tickets(&user2, ticket_type.id, 1).remove(0);
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);

    let mut path = Path::<PathParameters>::extract(&request.request).await.unwrap();
    path.id = ticket.id;

    let response: HttpResponse = tickets::show((database.connection.clone().into(), path, auth_user))
        .await
        .into();
    if should_test_succeed {
        assert_eq!(response.status(), StatusCode::OK);
        let body = support::unwrap_body_to_string(&response).unwrap();
        let ticket_response: ShowTicketResponse = serde_json::from_str(&body).unwrap();
        let fee_schedule = FeeSchedule::find(organization.fee_schedule_id, connection).unwrap();
        let fee_schedule_range = &fee_schedule.ranges(connection).unwrap()[0];
        let expected_ticket = DisplayTicket {
            id: ticket.id,
            order_id: cart.id,
            price_in_cents: (ticket_pricing.price_in_cents + fee_schedule_range.fee_in_cents) as u32,
            ticket_type_id: ticket_type.id,
            ticket_type_name: ticket_type.name.clone(),
            status: TicketInstanceStatus::Purchased,
            redeem_key: ticket_response.ticket.redeem_key.clone(),
            pending_transfer: false,
            first_name_override: None,
            last_name_override: None,
            transfer_id: None,
            transfer_key: None,
            transfer_address: None,
            check_in_source: None,
            promo_image_url: None,
        };

        let expected_result = ShowTicketResponse {
            ticket: expected_ticket,
            user: Some(user2.into()),
            event: event.for_display(connection).unwrap(),
        };
        assert_eq!(expected_result, ticket_response);
    } else {
        support::expects_unauthorized(&response);
    }
}

pub async fn update(role: Roles, owns_ticket: bool, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let request = TestRequest::create();
    let organization = database.create_organization().finish();
    let event = database
        .create_event()
        .with_organization(&organization)
        .with_ticket_pricing()
        .finish();
    let ticket_type = event.ticket_types(true, None, connection).unwrap().remove(0);
    let ticket_pricing = ticket_type.current_ticket_pricing(false, connection).unwrap();
    let cart = Order::find_or_create_cart(&user, connection).unwrap();
    let ticket = database.create_purchased_tickets(&user, ticket_type.id, 1).remove(0);

    let auth_user = if owns_ticket {
        support::create_auth_user_from_user(&user, role, Some(&organization), &database)
    } else {
        support::create_auth_user(role, Some(&organization), &database)
    };

    let mut path = Path::<PathParameters>::extract(&request.request).await.unwrap();
    path.id = ticket.id;
    let json = Json(UpdateTicketInstanceAttributes {
        first_name_override: Some(Some("First".to_string())),
        last_name_override: Some(Some("Last".to_string())),
    });

    let response: HttpResponse = tickets::update((database.connection.clone().into(), path, json, auth_user))
        .await
        .into();

    if should_test_succeed {
        assert_eq!(response.status(), StatusCode::OK);
        let body = support::unwrap_body_to_string(&response).unwrap();
        let ticket_response: ShowTicketResponse = serde_json::from_str(&body).unwrap();
        let fee_schedule = FeeSchedule::find(organization.fee_schedule_id, connection).unwrap();
        let fee_schedule_range = &fee_schedule.ranges(connection).unwrap()[0];
        let expected_ticket = DisplayTicket {
            id: ticket.id,
            order_id: cart.id,
            price_in_cents: (ticket_pricing.price_in_cents + fee_schedule_range.fee_in_cents) as u32,
            ticket_type_id: ticket_type.id,
            ticket_type_name: ticket_type.name.clone(),
            status: TicketInstanceStatus::Purchased,
            redeem_key: ticket_response.ticket.redeem_key.clone(),
            pending_transfer: false,
            first_name_override: Some("First".to_string()),
            last_name_override: Some("Last".to_string()),
            transfer_id: None,
            transfer_key: None,
            transfer_address: None,
            check_in_source: None,
            promo_image_url: None,
        };

        let expected_result = ShowTicketResponse {
            ticket: expected_ticket,
            user: Some(user.into()),
            event: event.for_display(connection).unwrap(),
        };
        assert_eq!(expected_result, ticket_response);
    } else {
        support::expects_unauthorized(&response);
    }
}

pub async fn show_redeemable_ticket(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let conn = database.connection.get();
    let user = database.create_user().finish();
    let request = TestRequest::create();
    let organization = database.create_organization().finish();
    let venue = database.create_venue().finish();
    let event = database
        .create_event()
        .with_organization(&organization)
        .with_event_start(dates::now().finish())
        .with_ticket_pricing()
        .with_venue(&venue)
        .finish();
    let user2 = database.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user2, conn).unwrap();
    let ticket_type = &event.ticket_types(true, None, conn).unwrap()[0];
    cart.update_quantities(
        user2.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 1,
            redemption_code: None,
        }],
        false,
        false,
        conn,
    )
    .unwrap();
    let total = cart.calculate_total(conn).unwrap();
    cart.add_external_payment(
        Some("test".to_string()),
        ExternalPaymentType::CreditCard,
        user2.id,
        total,
        conn,
    )
    .unwrap();
    let ticket = TicketInstance::find_for_user(user2.id, conn).unwrap().remove(0);
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);

    let mut path = Path::<PathParameters>::extract(&request.request).await.unwrap();
    path.id = ticket.id;

    let response: HttpResponse =
        tickets::show_redeemable_ticket((database.connection.clone().into(), path, auth_user.clone()))
            .await
            .into();

    if should_test_succeed {
        let body = support::unwrap_body_to_string(&response).unwrap();
        let ticket_response: RedeemableTicket = serde_json::from_str(&body).unwrap();
        assert!(ticket_response.redeem_key.is_some());
    } else {
        support::expects_unauthorized(&response);
    }
}
