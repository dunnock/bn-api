use actix_web::{http::StatusCode, FromRequest, HttpResponse, Path};
use bigneon_api::controllers::events::{self, TicketRedeemRequest};
use bigneon_api::controllers::tickets::{self, ShowTicketResponse};
use bigneon_api::db::CacheDatabase;
use bigneon_api::extractors::*;
use bigneon_api::models::{PathParameters, RedeemTicketPathParameters};
use bigneon_db::models::*;
use bigneon_db::utils::dates;
use serde_json;
use support;
use support::database::TestDatabase;
use support::test_request::TestRequest;

pub fn show_other_user_ticket(role: Roles, should_test_succeed: bool) {
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

    let mut path = Path::<PathParameters>::extract(&request.request).unwrap();
    path.id = ticket.id;

    let response: HttpResponse = tickets::show((database.connection.clone().into(), path, auth_user)).into();
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

pub fn update(role: Roles, owns_ticket: bool, should_test_succeed: bool) {
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

    let mut path = Path::<PathParameters>::extract(&request.request).unwrap();
    path.id = ticket.id;
    let json = Json(UpdateTicketInstanceAttributes {
        first_name_override: Some(Some("First".to_string())),
        last_name_override: Some(Some("Last".to_string())),
    });

    let response: HttpResponse = tickets::update((database.connection.clone().into(), path, json, auth_user)).into();

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

pub fn redeem_ticket(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let conn = database.connection.get();
    let user = database.create_user().finish();
    let request = TestRequest::create_with_uri_custom_params("/", vec!["id", "ticket_instance_id"]);
    let organization = database.create_organization().finish();
    let event = database
        .create_event()
        .with_organization(&organization)
        .with_ticket_pricing()
        .finish();
    let user2 = database.create_user().finish();
    let ticket_type = event.ticket_types(true, None, conn).unwrap()[0].id;
    let ticket = database.create_purchased_tickets(&user2, ticket_type, 5).remove(0);
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);

    let mut path = Path::<RedeemTicketPathParameters>::extract(&request.request).unwrap();
    path.id = event.id;
    path.ticket_instance_id = ticket.id;
    let mut path2 = Path::<RedeemTicketPathParameters>::extract(&request.request).unwrap();
    path2.id = event.id;
    path2.ticket_instance_id = ticket.id;

    //First try when Redeem code is wrong
    let request_data = TicketRedeemRequest {
        redeem_key: "WrongKey".to_string(),
        check_in_source: Some(CheckInSource::Scanned),
    };

    let response: HttpResponse = events::redeem_ticket((
        database.connection.clone().into(),
        path,
        Json(request_data),
        auth_user.clone(),
        request.extract_state(),
        CacheDatabase { inner: None },
    ))
    .into();

    if should_test_succeed {
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        //Now try with redeem code being correct
        let request_data = TicketRedeemRequest {
            redeem_key: ticket.redeem_key.unwrap(),
            check_in_source: Some(CheckInSource::Scanned),
        };

        let response: HttpResponse = events::redeem_ticket((
            database.connection.clone().into(),
            path2,
            Json(request_data),
            auth_user,
            request.extract_state(),
            CacheDatabase { inner: None },
        ))
        .into();
        let ticket = TicketInstance::find(ticket.id, conn).unwrap();
        assert_eq!(ticket.check_in_source, Some(CheckInSource::Scanned));

        assert_eq!(response.status(), StatusCode::OK);
    } else {
        support::expects_unauthorized(&response);
    }
}

pub fn show_redeemable_ticket(role: Roles, should_test_succeed: bool) {
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

    let mut path = Path::<PathParameters>::extract(&request.request).unwrap();
    path.id = ticket.id;

    let response: HttpResponse =
        tickets::show_redeemable_ticket((database.connection.clone().into(), path, auth_user.clone())).into();

    if should_test_succeed {
        let body = support::unwrap_body_to_string(&response).unwrap();
        let ticket_response: RedeemableTicket = serde_json::from_str(&body).unwrap();
        assert!(ticket_response.redeem_key.is_some());
    } else {
        support::expects_unauthorized(&response);
    }
}
