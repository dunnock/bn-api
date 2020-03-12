use actix_web::{FromRequest, http::StatusCode, HttpResponse, web::{Path, Query}};
use chrono::prelude::*;
use serde_json;
use uuid::Uuid;

use crate::functional::base;
use crate::support;
use crate::support::database::TestDatabase;
use crate::support::test_request::TestRequest;
use bigneon_api::controllers::tickets::SendTicketsRequest;
use bigneon_api::controllers::tickets::{self, SearchParameters, ShowTicketResponse, TransferTicketRequest};
use bigneon_api::extractors::*;
use bigneon_api::models::{OptionalPathParameters, PathParameters};
use bigneon_db::prelude::*;

#[test]
pub async fn index() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let test_request = TestRequest::create();
    let organization = database.create_organization().finish();
    let fee_schedule = FeeSchedule::find(organization.fee_schedule_id, connection).unwrap();
    let fee_schedule_range = &fee_schedule.ranges(connection).unwrap()[0];
    let venue = database.create_venue().finish();
    let event = database
        .create_event()
        .with_name("Event1".into())
        .with_event_start(NaiveDate::from_ymd(2016, 7, 8).and_hms(9, 10, 11))
        .with_event_end(NaiveDate::from_ymd(2016, 7, 9).and_hms(9, 10, 11))
        .with_venue(&venue)
        .with_organization(&organization)
        .with_ticket_pricing()
        .finish();
    // Other event
    let event2 = database
        .create_event()
        .with_name("Event2".into())
        .with_event_start(NaiveDate::from_ymd(2017, 7, 8).and_hms(9, 10, 11))
        .with_event_end(NaiveDate::from_ymd(2017, 7, 9).and_hms(9, 10, 11))
        .with_organization(&organization)
        .with_ticket_pricing()
        .finish();
    let conn = database.connection.get();
    let ticket_type = &event.ticket_types(true, None, conn).unwrap()[0];
    let ticket_pricing = ticket_type.current_ticket_pricing(false, conn).unwrap();
    let ticket_type2 = &event2.ticket_types(true, None, conn).unwrap()[0];
    let ticket_pricing2 = ticket_type2.current_ticket_pricing(false, conn).unwrap();
    let mut cart = Order::find_or_create_cart(&user, conn).unwrap();
    cart.update_quantities(
        user.id,
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
        user.id,
        total,
        conn,
    )
    .unwrap();
    let mut cart2 = Order::find_or_create_cart(&user, conn).unwrap();
    cart2
        .update_quantities(
            user.id,
            &[UpdateOrderItem {
                ticket_type_id: ticket_type2.id,
                quantity: 1,
                redemption_code: None,
            }],
            false,
            false,
            conn,
        )
        .unwrap();

    let total = cart2.calculate_total(conn).unwrap();
    cart2
        .add_external_payment(
            Some("test".to_string()),
            ExternalPaymentType::CreditCard,
            user.id,
            total,
            conn,
        )
        .unwrap();
    let ticket = TicketInstance::find_for_user_for_display(user.id, Some(event.id), None, None, conn)
        .unwrap()
        .remove(0)
        .1
        .remove(0);
    let ticket_id = ticket.id;
    let ticket2 = TicketInstance::find_for_user_for_display(user.id, Some(event2.id), None, None, conn)
        .unwrap()
        .remove(0)
        .1
        .remove(0);
    let ticket2_id = ticket2.id;
    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);

    // Test with specified event
    let mut path = Path::<OptionalPathParameters>::extract(&test_request.request).await.unwrap();
    path.id = Some(event.id);
    let parameters = Query::<SearchParameters>::extract(&test_request.request).await.unwrap();
    let response = tickets::index((database.connection.clone().into(), path, parameters, auth_user.clone())).unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    let found_data: Payload<DisplayTicket> = serde_json::from_str(&body).unwrap();
    let expected_ticket = DisplayTicket {
        id: ticket_id,
        order_id: cart.id,
        price_in_cents: (ticket_pricing.price_in_cents + fee_schedule_range.fee_in_cents) as u32,
        ticket_type_id: ticket_type.id,
        ticket_type_name: ticket_type.name.clone(),
        status: TicketInstanceStatus::Purchased,
        redeem_key: ticket.redeem_key,
        pending_transfer: false,
        first_name_override: None,
        last_name_override: None,
        transfer_id: None,
        transfer_key: None,
        transfer_address: None,
        check_in_source: None,
    };
    assert_eq!(vec![expected_ticket.clone()], found_data.data);
    // Test without specified event
    let mut path = Path::<OptionalPathParameters>::extract(&test_request.request).await.unwrap();
    path.id = None;
    let parameters = Query::<SearchParameters>::extract(&test_request.request).await.unwrap();
    let response = tickets::index((database.connection.clone().into(), path, parameters, auth_user.clone())).unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    let found_data: Payload<(DisplayEvent, Vec<DisplayTicket>)> = serde_json::from_str(&body).unwrap();
    let found_tickets = found_data.data;
    let expected_ticket2 = DisplayTicket {
        id: ticket2_id,
        order_id: cart2.id,
        price_in_cents: (ticket_pricing2.price_in_cents + fee_schedule_range.fee_in_cents) as u32,
        ticket_type_id: ticket_type2.id,
        ticket_type_name: ticket_type2.name.clone(),
        status: TicketInstanceStatus::Purchased,
        redeem_key: ticket2.redeem_key,
        pending_transfer: false,
        first_name_override: None,
        last_name_override: None,
        transfer_id: None,
        transfer_key: None,
        transfer_address: None,
        check_in_source: None,
    };
    assert_eq!(
        vec![
            (event.clone().for_display(conn).unwrap(), vec![expected_ticket.clone()]),
            (
                event2.clone().for_display(conn).unwrap(),
                vec![expected_ticket2.clone()]
            )
        ],
        found_tickets
    );

    // Tickets include live event
    let mut path = Path::<OptionalPathParameters>::extract(&test_request.request).await.unwrap();
    path.id = None;
    let mut parameters = Query::<SearchParameters>::extract(&test_request.request).await.unwrap();
    parameters.start_utc = Some(NaiveDate::from_ymd(2016, 7, 8).and_hms(9, 11, 11));
    let response = tickets::index((database.connection.clone().into(), path, parameters, auth_user.clone())).unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    let found_data: Payload<(DisplayEvent, Vec<DisplayTicket>)> = serde_json::from_str(&body).unwrap();
    let found_tickets = found_data.data;
    assert_eq!(
        vec![
            (event.clone().for_display(conn).unwrap(), vec![expected_ticket.clone()]),
            (
                event2.clone().for_display(conn).unwrap(),
                vec![expected_ticket2.clone()]
            )
        ],
        found_tickets
    );

    // Test with search parameter
    let mut path = Path::<OptionalPathParameters>::extract(&test_request.request).await.unwrap();
    path.id = None;
    let mut parameters = Query::<SearchParameters>::extract(&test_request.request).await.unwrap();
    parameters.start_utc = Some(NaiveDate::from_ymd(2017, 7, 8).and_hms(9, 0, 11));
    let response = tickets::index((database.connection.clone().into(), path, parameters, auth_user)).unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    let found_data: Payload<(DisplayEvent, Vec<DisplayTicket>)> = serde_json::from_str(&body).unwrap();
    let found_tickets = found_data.data;
    assert_eq!(
        vec![(event2.for_display(conn).unwrap(), vec![expected_ticket2.clone()])],
        found_tickets
    );
}

#[test]
pub async fn show() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let request = TestRequest::create();
    let organization = database.create_organization().finish();
    let fee_schedule = FeeSchedule::find(organization.fee_schedule_id, connection).unwrap();
    let fee_schedule_range = &fee_schedule.ranges(connection).unwrap()[0];
    let event = database
        .create_event()
        .with_organization(&organization)
        .with_event_start(dates::now().finish())
        .with_ticket_pricing()
        .finish();
    let conn = database.connection.get();
    let mut cart = Order::find_or_create_cart(&user, conn).unwrap();
    let ticket_type = &event.ticket_types(true, None, conn).unwrap()[0];
    let ticket_pricing = ticket_type.current_ticket_pricing(false, conn).unwrap();
    cart.update_quantities(
        user.id,
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
        user.id,
        total,
        conn,
    )
    .unwrap();
    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);
    let mut path = Path::<PathParameters>::extract(&request.request).await.unwrap();
    let ticket = TicketInstance::find_for_user(user.id, conn).unwrap().remove(0);
    path.id = ticket.id;
    let response = tickets::show((database.connection.clone().into(), path, auth_user)).unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    let ticket_response: ShowTicketResponse = serde_json::from_str(&body).unwrap();
    let expected_ticket = DisplayTicket {
        id: ticket.id,
        order_id: cart.id,
        price_in_cents: (ticket_pricing.price_in_cents + fee_schedule_range.fee_in_cents) as u32,
        ticket_type_id: ticket_type.id,
        ticket_type_name: ticket_type.name.clone(),
        status: TicketInstanceStatus::Purchased,
        redeem_key: ticket.redeem_key,
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
        user: Some(user.into()),
        event: event.for_display(conn).unwrap(),
    };
    assert_eq!(expected_result, ticket_response);
}

#[cfg(test)]
mod index_tests {
    use super::*;
    #[test]
    fn update_org_member() {
        base::tickets::update(Roles::OrgMember, false, false);
    }
    #[test]
    fn update_admin() {
        base::tickets::update(Roles::Admin, false, true);
    }
    #[test]
    fn update_user() {
        base::tickets::update(Roles::User, false, false);
    }
    #[test]
    fn update_org_owner() {
        base::tickets::update(Roles::OrgOwner, false, true);
    }
    #[test]
    fn update_door_person() {
        base::tickets::update(Roles::DoorPerson, false, false);
    }
    #[test]
    fn update_promoter() {
        base::tickets::update(Roles::Promoter, false, false);
    }
    #[test]
    fn update_promoter_read_only() {
        base::tickets::update(Roles::PromoterReadOnly, false, false);
    }
    #[test]
    fn update_org_admin() {
        base::tickets::update(Roles::OrgAdmin, false, true);
    }
    #[test]
    fn update_box_office() {
        base::tickets::update(Roles::OrgBoxOffice, false, false);
    }
    #[test]
    fn update_owns_order_org_member() {
        base::tickets::update(Roles::OrgMember, true, true);
    }
    #[test]
    fn update_owns_order_admin() {
        base::tickets::update(Roles::Admin, true, true);
    }
    #[test]
    fn update_owns_order_user() {
        base::tickets::update(Roles::User, true, true);
    }
    #[test]
    fn update_owns_order_org_owner() {
        base::tickets::update(Roles::OrgOwner, true, true);
    }
    #[test]
    fn update_owns_order_door_person() {
        base::tickets::update(Roles::DoorPerson, true, true);
    }
    #[test]
    fn update_owns_order_promoter() {
        base::tickets::update(Roles::Promoter, true, true);
    }
    #[test]
    fn update_owns_order_promoter_read_only() {
        base::tickets::update(Roles::PromoterReadOnly, true, true);
    }
    #[test]
    fn update_owns_order_org_admin() {
        base::tickets::update(Roles::OrgAdmin, true, true);
    }
    #[test]
    fn update_owns_order_box_office() {
        base::tickets::update(Roles::OrgBoxOffice, true, true);
    }
}

#[cfg(test)]
mod show_other_user_ticket_tests {
    use super::*;

    #[test]
    fn show_other_user_ticket_org_member() {
        base::tickets::show_other_user_ticket(Roles::OrgMember, true);
    }
    #[test]
    fn show_other_user_ticket_admin() {
        base::tickets::show_other_user_ticket(Roles::Admin, true);
    }
    #[test]
    fn show_other_user_ticket_user() {
        base::tickets::show_other_user_ticket(Roles::User, false);
    }
    #[test]
    fn show_other_user_ticket_org_owner() {
        base::tickets::show_other_user_ticket(Roles::OrgOwner, true);
    }
    #[test]
    fn show_other_user_ticket_door_person() {
        base::tickets::show_other_user_ticket(Roles::DoorPerson, true);
    }
    #[test]
    fn show_other_user_ticket_promoter() {
        base::tickets::show_other_user_ticket(Roles::Promoter, true);
    }
    #[test]
    fn show_other_user_ticket_promoter_read_only() {
        base::tickets::show_other_user_ticket(Roles::PromoterReadOnly, true);
    }
    #[test]
    fn show_other_user_ticket_org_admin() {
        base::tickets::show_other_user_ticket(Roles::OrgAdmin, true);
    }
    #[test]
    fn show_other_user_ticket_box_office() {
        base::tickets::show_other_user_ticket(Roles::OrgBoxOffice, true);
    }
}

#[cfg(test)]
mod show_redeem_key {
    use super::*;

    #[test]
    fn show_redeemable_ticket_org_member() {
        base::tickets::show_redeemable_ticket(Roles::OrgMember, true);
    }
    #[test]
    fn show_redeemable_ticket_admin() {
        base::tickets::show_redeemable_ticket(Roles::Admin, true);
    }
    #[test]
    fn show_redeemable_ticket_user() {
        base::tickets::show_redeemable_ticket(Roles::User, false);
    }
    #[test]
    fn show_redeemable_ticket_org_owner() {
        base::tickets::show_redeemable_ticket(Roles::OrgOwner, true);
    }
    #[test]
    fn show_redeemable_ticket_door_person() {
        base::tickets::show_redeemable_ticket(Roles::DoorPerson, true);
    }
    #[test]
    fn show_redeemable_ticket_promoter() {
        base::tickets::show_redeemable_ticket(Roles::Promoter, true);
    }
    #[test]
    fn show_redeemable_ticket_promoter_read_only() {
        base::tickets::show_redeemable_ticket(Roles::PromoterReadOnly, true);
    }
    #[test]
    fn show_redeemable_ticket_org_admin() {
        base::tickets::show_redeemable_ticket(Roles::OrgAdmin, true);
    }
    #[test]
    fn show_redeemable_ticket_box_office() {
        base::tickets::show_redeemable_ticket(Roles::OrgBoxOffice, true);
    }
}

#[actix_rt::test]
async fn ticket_transfer_authorization() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);
    let organization = database.create_organization().finish();
    let venue = database.create_venue().finish();
    let event = database
        .create_event()
        .with_name("Event1".into())
        .with_event_start(NaiveDate::from_ymd(2016, 7, 8).and_hms(9, 10, 11))
        .with_event_end(NaiveDate::from_ymd(2599, 7, 9).and_hms(9, 10, 11))
        .with_venue(&venue)
        .with_organization(&organization)
        .with_ticket_pricing()
        .finish();
    let conn = database.connection.get();

    let mut cart = Order::find_or_create_cart(&user, conn).unwrap();
    let ticket_type = &event.ticket_types(true, None, conn).unwrap()[0];

    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 5,
            redemption_code: None,
        }],
        false,
        false,
        conn,
    )
    .unwrap();

    let tickets = cart.tickets(None, conn).unwrap();
    //Try transfer before paying for the tickets
    let mut ticket_transfer_request = TransferTicketRequest {
        ticket_ids: vec![tickets[0].id, tickets[1].id],
    };

    let response = tickets::transfer_authorization((
        database.connection.clone().into(),
        Json(ticket_transfer_request.clone()),
        auth_user.clone(),
    ));

    assert!(response.is_err());

    //Try after paying for the tickets
    let total = cart.calculate_total(conn).unwrap();
    cart.add_external_payment(
        Some("test".to_string()),
        ExternalPaymentType::CreditCard,
        user.id,
        total,
        conn,
    )
    .unwrap();

    let response = tickets::transfer_authorization((
        database.connection.clone().into(),
        Json(ticket_transfer_request.clone()),
        auth_user.clone(),
    ))
    .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = support::unwrap_body_to_string(&response).unwrap();
    let transfer_response: TransferAuthorization = serde_json::from_str(&body).unwrap();
    assert_eq!(transfer_response.sender_user_id, user.id);

    //Now lets try add a ticket that the user doesn't own.
    ticket_transfer_request.ticket_ids.push(Uuid::new_v4());
    let response = tickets::transfer_authorization((
        database.connection.clone().into(),
        Json(ticket_transfer_request),
        auth_user.clone(),
    ));

    assert!(response.is_err());
}

#[actix_rt::test]
async fn send_to_existing_user() {
    let database = TestDatabase::new();
    let conn = database.connection.get();
    let sender = database.create_user().finish();
    let auth_sender = support::create_auth_user_from_user(&sender, Roles::User, None, &database);
    database.create_order().for_user(&sender).is_paid().finish();
    let tickets = TicketInstance::find_for_user(sender.id, conn).unwrap();
    let tickets: Vec<Uuid> = tickets.into_iter().map(|t| t.id).collect();
    let expected_tickets = tickets.clone();

    let receiver = database.create_user().finish();
    let ticket_transfer_request = SendTicketsRequest {
        ticket_ids: tickets,
        email_or_phone: receiver.email.unwrap(),
    };

    let request = TestRequest::create();
    let response = tickets::send_via_email_or_phone((
        database.connection.clone().into(),
        Json(ticket_transfer_request.clone()),
        auth_sender.clone(),
        request.extract_state(),
    ))
    .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let sender_tickets = TicketInstance::find_for_user(sender.id, conn).unwrap();
    let receiver_tickets = TicketInstance::find_for_user(receiver.id, conn).unwrap();
    assert_eq!(sender_tickets, vec![]);
    assert_equiv!(
        receiver_tickets.into_iter().map(|t| t.id).collect::<Vec<Uuid>>(),
        expected_tickets
    );
}

#[actix_rt::test]
async fn send_to_new_user() {
    let database = TestDatabase::new();
    let conn = database.connection.get();
    let sender = database.create_user().finish();
    let auth_sender = support::create_auth_user_from_user(&sender, Roles::User, None, &database);
    database.create_order().for_user(&sender).is_paid().finish();
    let tickets = TicketInstance::find_for_user(sender.id, conn).unwrap();
    let tickets: Vec<Uuid> = tickets.into_iter().map(|t| t.id).collect();
    let expected_tickets = tickets.clone();

    let ticket_transfer_request = SendTicketsRequest {
        ticket_ids: tickets,
        email_or_phone: "test@tari.com".to_string(),
    };

    let request = TestRequest::create();
    let response = tickets::send_via_email_or_phone((
        database.connection.clone().into(),
        Json(ticket_transfer_request.clone()),
        auth_sender.clone(),
        request.extract_state(),
    ))
    .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let sender_tickets = TicketInstance::find_for_user(sender.id, conn).unwrap();
    assert_equiv!(
        sender_tickets.into_iter().map(|t| t.id).collect::<Vec<Uuid>>(),
        expected_tickets
    );
}

#[actix_rt::test]
async fn receive_ticket_transfer() {
    let database = TestDatabase::new();
    let request = TestRequest::create();
    let user = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);
    let organization = database.create_organization().finish();
    let venue = database.create_venue().finish();
    let event = database
        .create_event()
        .with_name("Event1".into())
        .with_event_start(NaiveDate::from_ymd(2016, 7, 8).and_hms(9, 10, 11))
        .with_event_end(NaiveDate::from_ymd(2599, 7, 9).and_hms(9, 10, 11))
        .with_venue(&venue)
        .with_organization(&organization)
        .with_ticket_pricing()
        .finish();
    let conn = database.connection.get();

    let mut cart = Order::find_or_create_cart(&user, conn).unwrap();
    let ticket_type = &event.ticket_types(true, None, conn).unwrap()[0];

    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 5,
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
        user.id,
        total,
        conn,
    )
    .unwrap();
    let tickets = TicketInstance::find_for_user(user.id, conn).unwrap();

    let transfer_auth: TransferAuthorization = TicketInstance::create_transfer(
        &auth_user.user,
        &vec![tickets[0].id, tickets[1].id],
        None,
        None,
        false,
        conn,
    )
    .unwrap()
    .into_authorization(conn)
    .unwrap();

    //Try receive transfer
    let user2 = database.create_user().finish();
    let auth_user2 = support::create_auth_user_from_user(&user2, Roles::User, None, &database);

    let response = tickets::receive_transfer((
        database.connection.clone().into(),
        Json(transfer_auth.clone()),
        auth_user2.clone(),
        request.extract_state(),
    ))
    .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[actix_rt::test]
async fn receive_ticket_transfer_fails_cancelled_transfer() {
    let database = TestDatabase::new();
    let request = TestRequest::create();
    let user = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);
    let organization = database.create_organization().finish();
    let venue = database.create_venue().finish();
    let event = database
        .create_event()
        .with_name("Event1".into())
        .with_event_start(NaiveDate::from_ymd(2016, 7, 8).and_hms(9, 10, 11))
        .with_event_end(NaiveDate::from_ymd(2599, 7, 9).and_hms(9, 10, 11))
        .with_venue(&venue)
        .with_organization(&organization)
        .with_ticket_pricing()
        .finish();
    let conn = database.connection.get();

    let mut cart = Order::find_or_create_cart(&user, conn).unwrap();
    let ticket_type = &event.ticket_types(true, None, conn).unwrap()[0];

    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 5,
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
        user.id,
        total,
        conn,
    )
    .unwrap();
    let tickets = TicketInstance::find_for_user(user.id, conn).unwrap();
    let ticket_ids = vec![tickets[0].id, tickets[1].id];
    let transfer = TicketInstance::create_transfer(&auth_user.user, &ticket_ids, None, None, false, conn).unwrap();
    transfer.cancel(&user, None, conn).unwrap();

    //Try receive transfer
    let user2 = database.create_user().finish();
    let auth_user2 = support::create_auth_user_from_user(&user2, Roles::User, None, &database);

    let response: HttpResponse = tickets::receive_transfer((
        database.connection.clone().into(),
        Json(transfer.into_authorization(conn).unwrap()),
        auth_user2.clone(),
        request.extract_state(),
    ))
    .into();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(body, json!({"error": "The transfer has been cancelled."}).to_string());
}
