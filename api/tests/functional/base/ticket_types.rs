use crate::support;
use crate::support::database::TestDatabase;
use crate::support::test_request::TestRequest;
use actix_web::{FromRequest, http::StatusCode, HttpResponse, web::{Path, Query}};
use bigneon_api::controllers::ticket_types;
use bigneon_api::controllers::ticket_types::*;
use bigneon_api::extractors::*;
use bigneon_api::models::{AdminDisplayTicketType, EventTicketPathParameters, PathParameters};
use bigneon_db::models::*;
use chrono::prelude::*;
use serde_json;

pub async fn create(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let event = database.create_event().with_organization(&organization).finish();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);

    //Construct Ticket creation and pricing request
    let test_request = TestRequest::create();
    let state = test_request.extract_state().await;
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = event.id;
    let mut ticket_pricing: Vec<CreateTicketPricingRequest> = Vec::new();
    let start_date = NaiveDate::from_ymd(2018, 5, 1).and_hms(6, 20, 21);
    let middle_date = NaiveDate::from_ymd(2018, 6, 2).and_hms(7, 45, 31);
    let end_date = NaiveDate::from_ymd(2018, 7, 3).and_hms(9, 23, 23);
    ticket_pricing.push(CreateTicketPricingRequest {
        name: String::from("Early bird"),
        price_in_cents: 10000,
        start_date,
        end_date: middle_date,
        is_box_office_only: Some(false),
    });
    ticket_pricing.push(CreateTicketPricingRequest {
        name: String::from("Base"),
        price_in_cents: 20000,
        start_date: middle_date,
        end_date,
        is_box_office_only: Some(false),
    });
    let request_data = CreateTicketTypeRequest {
        name: "VIP".into(),
        description: None,
        capacity: 1000,
        start_date: Some(start_date),
        end_date: Some(end_date),
        end_date_type: Some(TicketTypeEndDateType::Manual),
        ticket_pricing,
        increment: None,
        limit_per_person: 0,
        price_in_cents: 20000,
        visibility: TicketTypeVisibility::Always,
        parent_id: None,
        additional_fee_in_cents: None,
        rank: 1,
        ..Default::default()
    };
    let response: HttpResponse =
        ticket_types::create((database.connection.into(), path, Json(request_data), auth_user, state)).await.into();

    if should_test_succeed {
        assert_eq!(response.status(), StatusCode::CREATED);
    } else {
        support::expects_unauthorized(&response);
    }
}

pub async fn create_multiple(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let event = database.create_event().with_organization(&organization).finish();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);

    //Construct Ticket creation and pricing request
    let test_request = TestRequest::create();
    let state = test_request.extract_state().await;
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = event.id;
    let mut ticket_types: Vec<CreateTicketTypeRequest> = Vec::new();
    let mut ticket_pricing: Vec<CreateTicketPricingRequest> = Vec::new();
    let start_date = NaiveDate::from_ymd(2018, 5, 1).and_hms(6, 20, 21);
    let middle_date = NaiveDate::from_ymd(2018, 6, 2).and_hms(7, 45, 31);
    let end_date = NaiveDate::from_ymd(2018, 7, 3).and_hms(9, 23, 23);
    ticket_pricing.push(CreateTicketPricingRequest {
        name: String::from("Early bird"),
        price_in_cents: 10000,
        start_date,
        end_date: middle_date,
        is_box_office_only: Some(false),
    });
    ticket_pricing.push(CreateTicketPricingRequest {
        name: String::from("Base"),
        price_in_cents: 20000,
        start_date: middle_date,
        end_date,
        is_box_office_only: Some(false),
    });
    ticket_types.push(CreateTicketTypeRequest {
        name: "VIP".into(),
        description: None,
        capacity: 1000,
        start_date: Some(start_date),
        end_date: Some(end_date),
        end_date_type: Some(TicketTypeEndDateType::Manual),
        ticket_pricing,
        increment: None,
        limit_per_person: 0,
        price_in_cents: 20000,
        visibility: TicketTypeVisibility::Always,
        parent_id: None,
        ..Default::default()
    });
    let mut ticket_pricing: Vec<CreateTicketPricingRequest> = Vec::new();
    ticket_pricing.push(CreateTicketPricingRequest {
        name: String::from("Base"),
        price_in_cents: 10000,
        start_date,
        end_date,
        is_box_office_only: Some(false),
    });
    ticket_types.push(CreateTicketTypeRequest {
        name: "GA".into(),
        description: None,
        capacity: 2000,
        start_date: Some(start_date),
        end_date: Some(end_date),
        end_date_type: Some(TicketTypeEndDateType::Manual),
        ticket_pricing,
        increment: None,
        limit_per_person: 0,
        price_in_cents: 10000,
        visibility: TicketTypeVisibility::Always,
        parent_id: None,
        additional_fee_in_cents: None,
        rank: 1,

        ..Default::default()
    });
    let response: HttpResponse = ticket_types::create_multiple((
        database.connection.clone().into(),
        path,
        Json(CreateMultipleTicketTypeRequest { ticket_types }),
        auth_user,
        state,
    ))
    .into();

    if should_test_succeed {
        assert_eq!(response.status(), StatusCode::CREATED);
        let ticket_types = &event.ticket_types(true, None, connection).unwrap();
        match ticket_types.iter().find(|tt| tt.name == "GA") {
            Some(ticket_type) => {
                assert_eq!(ticket_type.name, "GA".to_string());
                assert_eq!(ticket_type.ticket_pricing(true, connection).unwrap().len(), 2);
                assert_eq!(
                    ticket_type.start_date.map(|d| d.timestamp()),
                    Some(start_date.timestamp())
                );
                assert_eq!(
                    ticket_type.end_date(connection).unwrap().timestamp(),
                    end_date.timestamp()
                );
            }
            None => panic!("Expected GA ticket type to exist"),
        }
        match ticket_types.iter().find(|tt| tt.name == "VIP") {
            Some(ticket_type) => {
                assert_eq!(ticket_type.name, "VIP".to_string());
                assert_eq!(ticket_type.ticket_pricing(true, connection).unwrap().len(), 3);
                assert_eq!(
                    ticket_type.start_date.map(|d| d.timestamp()),
                    Some(start_date.timestamp())
                );
                assert_eq!(
                    ticket_type.end_date(connection).unwrap().timestamp(),
                    end_date.timestamp()
                );
            }
            None => panic!("Expected VIP ticket type to exist"),
        }
    } else {
        support::expects_unauthorized(&response);
    }
}

pub async fn update(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let request = TestRequest::create();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let event = database
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);

    //Retrieve created ticket type and pricing
    let conn = database.connection.get();
    let created_ticket_type = &event.ticket_types(true, None, conn).unwrap()[0];
    let created_ticket_capacity = created_ticket_type.valid_ticket_count(conn).unwrap();
    let created_ticket_pricing = created_ticket_type.ticket_pricing(false, conn).unwrap();

    //Construct update request
    let test_request = TestRequest::create_with_uri_custom_params("/", vec!["event_id", "ticket_type_id"]);
    let mut path = Path::<EventTicketPathParameters>::extract(&test_request.request).await.unwrap();
    path.event_id = event.id;
    path.ticket_type_id = created_ticket_type.id;

    let mut request_ticket_pricing: Vec<UpdateTicketPricingRequest> = Vec::new();
    let start_date = Some(NaiveDate::from_ymd(2018, 5, 1).and_hms(6, 20, 21));
    let middle_date = Some(NaiveDate::from_ymd(2018, 6, 2).and_hms(7, 45, 31));
    let end_date = Some(NaiveDate::from_ymd(2018, 7, 3).and_hms(9, 23, 23));
    let new_pricing_name = String::from("Online");
    //Remove 1st pricing, modify 2nd pricing and add new additional pricing
    request_ticket_pricing.push(UpdateTicketPricingRequest {
        id: Some(created_ticket_pricing[1].id),
        name: Some(String::from("Base")),
        start_date: middle_date,
        end_date,
        price_in_cents: Some(20000),
        is_box_office_only: Some(false),
    });
    request_ticket_pricing.push(UpdateTicketPricingRequest {
        id: None,
        name: Some(new_pricing_name.clone()),
        start_date,
        end_date: middle_date,
        price_in_cents: Some(15000),
        is_box_office_only: Some(false),
    });
    let request_data = UpdateTicketTypeRequest {
        name: Some("Updated VIP".into()),
        description: None,
        capacity: Some(created_ticket_capacity),
        start_date: Some(start_date),
        end_date: Some(end_date),
        end_date_type: None,
        ticket_pricing: Some(request_ticket_pricing),
        increment: None,
        limit_per_person: Some(0),
        price_in_cents: Some(15000),
        visibility: Some(TicketTypeVisibility::WhenAvailable),
        parent_id: None,
        ..Default::default()
    };
    let request_json = serde_json::to_string(&request_data).unwrap();

    //Send update request
    let response: HttpResponse = ticket_types::update((
        database.connection.clone().into(),
        path,
        Json(request_data),
        auth_user,
        request.extract_state(),
    ))
    .into();

    //Check if fields have been updated by retrieving the ticket type and pricing
    let updated_ticket_type = &event.ticket_types(true, None, conn).unwrap()[0];
    let updated_ticket_capacity = updated_ticket_type.valid_ticket_count(conn).unwrap();
    let updated_ticket_pricing = updated_ticket_type.ticket_pricing(false, conn).unwrap();
    let mut new_ticket_pricing: Vec<UpdateTicketPricingRequest> = Vec::new();
    new_ticket_pricing.reserve(updated_ticket_pricing.len());
    for current_ticket_pricing in &updated_ticket_pricing {
        //Replace the id of the new additional pricing with None so we can compare it with the request json
        let option_pricing_id = if current_ticket_pricing.name == new_pricing_name {
            None
        } else {
            Some(current_ticket_pricing.id)
        };

        new_ticket_pricing.push(UpdateTicketPricingRequest {
            id: option_pricing_id,
            name: Some(current_ticket_pricing.name.clone()),
            start_date: Some(current_ticket_pricing.start_date),
            end_date: Some(current_ticket_pricing.end_date),
            price_in_cents: Some(current_ticket_pricing.price_in_cents),
            is_box_office_only: Some(false),
        });
    }
    let updated_data = UpdateTicketTypeRequest {
        name: Some(updated_ticket_type.name.clone()),
        description: None,
        capacity: Some(updated_ticket_capacity),
        start_date: Some(updated_ticket_type.start_date),
        end_date: Some(updated_ticket_type.end_date),
        end_date_type: None,
        ticket_pricing: Some(new_ticket_pricing),
        increment: None,
        limit_per_person: Some(0),
        price_in_cents: Some(updated_ticket_type.price_in_cents),
        visibility: Some(TicketTypeVisibility::WhenAvailable),
        parent_id: None,
        ..Default::default()
    };
    let updated_json = serde_json::to_string(&updated_data).unwrap();

    if should_test_succeed {
        assert_eq!(request_json, updated_json);
        assert_eq!(response.status(), StatusCode::OK);
    } else {
        support::expects_unauthorized(&response);
    }
}

pub async fn cancel(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let event = database
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);

    let conn = database.connection.get();
    let created_ticket_type = &event.ticket_types(true, None, conn).unwrap()[0];

    let valid_unsold_ticket_count = created_ticket_type.valid_unsold_ticket_count(conn).unwrap();
    assert_eq!(100, valid_unsold_ticket_count);

    //Construct update request
    let test_request = TestRequest::create_with_uri_custom_params("/", vec!["event_id", "ticket_type_id"]);
    let state = test_request.extract_state().await;
    let mut path = Path::<EventTicketPathParameters>::extract(&test_request.request).await.unwrap();
    path.event_id = event.id;
    path.ticket_type_id = created_ticket_type.id;

    //Send update request
    let response: HttpResponse =
        ticket_types::cancel((database.connection.clone().into(), path, auth_user, state)).await.into();

    let updated_ticket_types = event.ticket_types(true, None, conn).unwrap();

    if should_test_succeed {
        assert_eq!(updated_ticket_types.len(), 0);
        assert_eq!(response.status(), StatusCode::OK);
    } else {
        support::expects_unauthorized(&response);
    }
}

pub async fn index(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let request = TestRequest::create();
    let organization = database.create_organization().finish();
    let conn = database.connection.get();
    let fee_schedule = FeeSchedule::find(organization.fee_schedule_id, conn).unwrap();
    let event = database
        .create_event()
        .with_organization(&organization)
        .with_ticket_pricing()
        .finish();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);

    let mut path = Path::<PathParameters>::extract(&request.request).await.unwrap();
    path.id = event.id;
    let test_request = TestRequest::create_with_uri(&format!("/limits?"));
    let query_parameters = Query::<PagingParameters>::extract(&test_request.request).await.unwrap();
    let response: HttpResponse =
        ticket_types::index((database.connection.clone().into(), path, query_parameters, auth_user)).await.into();
    if should_test_succeed {
        let body = support::unwrap_body_to_string(&response).unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let ticket_type = &event.ticket_types(true, None, conn).unwrap()[0];
        let expected_ticket_types =
            vec![AdminDisplayTicketType::from_ticket_type(ticket_type, &fee_schedule, conn).unwrap()];
        let ticket_types_response: Payload<AdminDisplayTicketType> = serde_json::from_str(&body).unwrap();
        assert_eq!(ticket_types_response.data, expected_ticket_types);
    } else {
        support::expects_unauthorized(&response);
    }
}
