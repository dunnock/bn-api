use crate::support;
use crate::support::database::TestDatabase;
use crate::support::test_request::TestRequest;
use actix_web::{http::StatusCode, HttpResponse, web::{Path, Query}};
use bigneon_api::controllers::redemption_codes::{self, *};
use bigneon_api::extractors::*;
use bigneon_api::models::UserDisplayTicketType;
use bigneon_db::prelude::*;
use serde_json;

#[test]
fn show_hold() {
    let database = TestDatabase::new();
    let connection = database.connection.get();

    let hold = database.create_hold().with_hold_type(HoldTypes::Discount).finish();
    let test_request = TestRequest::create_with_uri_custom_params("/", vec!["code"]);
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.code = hold.redemption_code.clone().unwrap();

    let test_request = TestRequest::create_with_uri("/");
    let parameters = Query::<EventParameter>::extract(&test_request.request).unwrap();

    let response: HttpResponse =
        redemption_codes::show(database.connection.clone().into(), parameters, path, OptionalUser(None)).into();
    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    let redemption_code_response: RedemptionCodeResponse = serde_json::from_str(&body).unwrap();

    match redemption_code_response {
        RedemptionCodeResponse::Hold {
            ticket_types,
            redemption_code,
            max_per_user,
            discount_in_cents,
            hold_type,
            user_purchased_ticket_count,
        } => {
            let user_display_ticket_type = UserDisplayTicketType::from_ticket_type(
                &TicketType::find(hold.ticket_type_id, connection).unwrap(),
                &FeeSchedule::find(
                    Organization::find_for_event(hold.event_id, connection)
                        .unwrap()
                        .fee_schedule_id,
                    connection,
                )
                .unwrap(),
                false,
                hold.redemption_code.clone(),
                connection,
            )
            .unwrap();
            assert_eq!(redemption_code, hold.redemption_code);
            assert_eq!(ticket_types, vec![user_display_ticket_type]);
            assert_eq!(max_per_user, hold.max_per_user);
            assert_eq!(discount_in_cents, hold.discount_in_cents);
            assert_eq!(hold_type, HoldTypes::Discount);
            assert_eq!(user_purchased_ticket_count, None);
        }
        _ => panic!("Expected RedemptionCodeResponse::Hold response"),
    }
}

#[test]
fn show_comp() {
    let database = TestDatabase::new();
    let connection = database.connection.get();

    let hold = database.create_hold().with_hold_type(HoldTypes::Comp).finish();
    let test_request = TestRequest::create_with_uri_custom_params("/", vec!["code"]);
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    let test_request = TestRequest::create_with_uri("/");
    let parameters = Query::<EventParameter>::extract(&test_request.request).unwrap();
    path.code = hold.redemption_code.clone().unwrap();
    let response: HttpResponse =
        redemption_codes::show(database.connection.clone().into(), parameters, path, OptionalUser(None)).into();
    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    let redemption_code_response: RedemptionCodeResponse = serde_json::from_str(&body).unwrap();

    match redemption_code_response {
        RedemptionCodeResponse::Hold {
            ticket_types,
            redemption_code,
            max_per_user,
            discount_in_cents,
            hold_type,
            user_purchased_ticket_count,
        } => {
            let user_display_ticket_type = UserDisplayTicketType::from_ticket_type(
                &TicketType::find(hold.ticket_type_id, connection).unwrap(),
                &FeeSchedule::find(
                    Organization::find_for_event(hold.event_id, connection)
                        .unwrap()
                        .fee_schedule_id,
                    connection,
                )
                .unwrap(),
                false,
                hold.redemption_code.clone(),
                connection,
            )
            .unwrap();
            let expected_discount_in_cents = user_display_ticket_type
                .ticket_pricing
                .clone()
                .map(|tp| tp.discount_in_cents);
            assert_eq!(redemption_code, hold.redemption_code);
            assert_eq!(ticket_types, vec![user_display_ticket_type]);
            assert_eq!(max_per_user, hold.max_per_user);
            assert_eq!(discount_in_cents, expected_discount_in_cents);
            assert_eq!(hold_type, HoldTypes::Comp);
            assert_eq!(user_purchased_ticket_count, None);
        }
        _ => panic!("Expected RedemptionCodeResponse::Hold response"),
    }
}

#[test]
fn show_hold_for_user() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);

    let hold = database.create_hold().with_hold_type(HoldTypes::Discount).finish();
    let test_request = TestRequest::create_with_uri_custom_params("/", vec!["code"]);
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.code = hold.redemption_code.clone().unwrap();

    let event = Event::find(hold.event_id, connection).unwrap();
    database
        .create_order()
        .for_event(&event)
        .quantity(4)
        .with_redemption_code(hold.redemption_code.clone().unwrap())
        .for_user(&user)
        .is_paid()
        .finish();

    let test_request = TestRequest::create_with_uri("/");
    let parameters = Query::<EventParameter>::extract(&test_request.request).unwrap();

    let response: HttpResponse = redemption_codes::show(
        database.connection.clone().into(),
        parameters,
        path,
        OptionalUser(Some(auth_user.clone())),
    )
    .into();
    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    let redemption_code_response: RedemptionCodeResponse = serde_json::from_str(&body).unwrap();

    match redemption_code_response {
        RedemptionCodeResponse::Hold {
            ticket_types,
            redemption_code,
            max_per_user,
            discount_in_cents,
            hold_type,
            user_purchased_ticket_count,
        } => {
            let user_display_ticket_type = UserDisplayTicketType::from_ticket_type(
                &TicketType::find(hold.ticket_type_id, connection).unwrap(),
                &FeeSchedule::find(
                    Organization::find_for_event(hold.event_id, connection)
                        .unwrap()
                        .fee_schedule_id,
                    connection,
                )
                .unwrap(),
                false,
                hold.redemption_code.clone(),
                connection,
            )
            .unwrap();
            assert_eq!(redemption_code, hold.redemption_code);
            assert_eq!(ticket_types, vec![user_display_ticket_type]);
            assert_eq!(max_per_user, hold.max_per_user);
            assert_eq!(discount_in_cents, hold.discount_in_cents);
            assert_eq!(hold_type, HoldTypes::Discount);
            assert_eq!(user_purchased_ticket_count, Some(4));
        }
        _ => panic!("Expected RedemptionCodeResponse::Hold response"),
    }
}

#[test]
fn show_code() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let organization = database.create_organization().with_fees().finish();
    let event = database
        .create_event()
        .with_organization(&organization)
        .with_ticket_pricing()
        .finish();
    let ticket_type = event.ticket_types(true, None, connection).unwrap().remove(0);

    let code = database
        .create_code()
        .with_event(&event)
        .with_code_type(CodeTypes::Discount)
        .for_ticket_type(&ticket_type)
        .with_discount_in_cents(Some(10))
        .finish();
    let test_request = TestRequest::create_with_uri_custom_params("/", vec!["code"]);
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.code = code.redemption_code.clone();
    let test_request = TestRequest::create_with_uri("/");
    let parameters = Query::<EventParameter>::extract(&test_request.request).unwrap();
    let response: HttpResponse =
        redemption_codes::show(database.connection.clone().into(), parameters, path, OptionalUser(None)).into();
    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    let redemption_code_response: RedemptionCodeResponse = serde_json::from_str(&body).unwrap();

    match redemption_code_response {
        RedemptionCodeResponse::Code {
            ticket_types,
            redemption_code,
            max_uses,
            discount_in_cents,
            discount_as_percentage,
            code_type,
            start_date,
            end_date,
            max_per_user: max_tickets_per_user,
            available,
            user_purchased_ticket_count,
        } => {
            let user_display_ticket_type = UserDisplayTicketType::from_ticket_type(
                &ticket_type,
                &FeeSchedule::find(organization.fee_schedule_id, connection).unwrap(),
                false,
                Some(code.redemption_code.clone()),
                connection,
            )
            .unwrap();
            assert_eq!(redemption_code, code.redemption_code);
            assert_eq!(ticket_types, vec![user_display_ticket_type]);
            assert_eq!(max_uses, code.max_uses);
            assert_eq!(max_tickets_per_user, code.max_tickets_per_user);
            assert_eq!(start_date, code.start_date);
            assert_eq!(end_date, code.end_date);
            assert_eq!(discount_in_cents, code.discount_in_cents);
            assert_eq!(code_type, CodeTypes::Discount);
            assert_eq!(available, 30);
            assert_eq!(discount_as_percentage, None);
            assert_eq!(user_purchased_ticket_count, None);
        }
        _ => panic!("Expected RedemptionCodeResponse::Code response"),
    }
}

#[test]
fn show_invalid() {
    let database = TestDatabase::new();
    let test_request = TestRequest::create_with_uri_custom_params("/", vec!["code"]);
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.code = "invalid".to_string();
    let test_request = TestRequest::create_with_uri("/");
    let parameters = Query::<EventParameter>::extract(&test_request.request).unwrap();
    let response: HttpResponse =
        redemption_codes::show(database.connection.clone().into(), parameters, path, OptionalUser(None)).into();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
