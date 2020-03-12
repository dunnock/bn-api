use crate::functional::base;
use crate::support;
use crate::support::database::TestDatabase;
use crate::support::test_request::TestRequest;
use actix_web::{http::StatusCode, HttpResponse, web::Path};
use bigneon_api::controllers::ticket_types;
use bigneon_api::controllers::ticket_types::*;
use bigneon_api::extractors::*;
use bigneon_api::models::{EventTicketPathParameters, PathParameters};
use bigneon_db::models::*;
use chrono::prelude::*;
use serde_json;
use uuid::Uuid;

#[cfg(test)]
mod create_tests {
    use super::*;
    #[test]
    fn create_org_member() {
        base::ticket_types::create(Roles::OrgMember, true);
    }
    #[test]
    fn create_admin() {
        base::ticket_types::create(Roles::Admin, true);
    }
    #[test]
    fn create_user() {
        base::ticket_types::create(Roles::User, false);
    }
    #[test]
    fn create_org_owner() {
        base::ticket_types::create(Roles::OrgOwner, true);
    }
    #[test]
    fn create_door_person() {
        base::ticket_types::create(Roles::DoorPerson, false);
    }
    #[test]
    fn create_promoter() {
        base::ticket_types::create(Roles::Promoter, true);
    }
    #[test]
    fn create_promoter_read_only() {
        base::ticket_types::create(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn create_org_admin() {
        base::ticket_types::create(Roles::OrgAdmin, true);
    }
    #[test]
    fn create_box_office() {
        base::ticket_types::create(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod create_multiple_tests {
    use super::*;
    #[test]
    fn create_multiple_org_member() {
        base::ticket_types::create_multiple(Roles::OrgMember, true);
    }
    #[test]
    fn create_multiple_admin() {
        base::ticket_types::create_multiple(Roles::Admin, true);
    }
    #[test]
    fn create_multiple_user() {
        base::ticket_types::create_multiple(Roles::User, false);
    }
    #[test]
    fn create_multiple_org_owner() {
        base::ticket_types::create_multiple(Roles::OrgOwner, true);
    }
    #[test]
    fn create_multiple_door_person() {
        base::ticket_types::create_multiple(Roles::DoorPerson, false);
    }
    #[test]
    fn create_multiple_promoter() {
        base::ticket_types::create_multiple(Roles::Promoter, true);
    }
    #[test]
    fn create_multiple_promoter_read_only() {
        base::ticket_types::create_multiple(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn create_multiple_org_admin() {
        base::ticket_types::create_multiple(Roles::OrgAdmin, true);
    }
    #[test]
    fn create_multiple_box_office() {
        base::ticket_types::create_multiple(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod update_tests {
    use super::*;
    #[test]
    fn update_org_member() {
        base::ticket_types::update(Roles::OrgMember, true);
    }
    #[test]
    fn update_admin() {
        base::ticket_types::update(Roles::Admin, true);
    }
    #[test]
    fn update_user() {
        base::ticket_types::update(Roles::User, false);
    }
    #[test]
    fn update_org_owner() {
        base::ticket_types::update(Roles::OrgOwner, true);
    }
    #[test]
    fn update_door_person() {
        base::ticket_types::update(Roles::DoorPerson, false);
    }
    #[test]
    fn update_promoter() {
        base::ticket_types::update(Roles::Promoter, true);
    }
    #[test]
    fn update_promoter_read_only() {
        base::ticket_types::update(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn update_org_admin() {
        base::ticket_types::update(Roles::OrgAdmin, true);
    }
    #[test]
    fn update_box_office() {
        base::ticket_types::update(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod cancel_tests {
    use super::*;
    #[test]
    fn cancel_org_member() {
        base::ticket_types::cancel(Roles::OrgMember, true);
    }
    #[test]
    fn cancel_admin() {
        base::ticket_types::cancel(Roles::Admin, true);
    }
    #[test]
    fn cancel_user() {
        base::ticket_types::cancel(Roles::User, false);
    }
    #[test]
    fn cancel_org_owner() {
        base::ticket_types::cancel(Roles::OrgOwner, true);
    }
    #[test]
    fn cancel_door_person() {
        base::ticket_types::cancel(Roles::DoorPerson, false);
    }
    #[test]
    fn cancel_promoter() {
        base::ticket_types::cancel(Roles::Promoter, true);
    }
    #[test]
    fn cancel_promoter_read_only() {
        base::ticket_types::cancel(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn cancel_org_admin() {
        base::ticket_types::cancel(Roles::OrgAdmin, true);
    }
    #[test]
    fn cancel_box_office() {
        base::ticket_types::cancel(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod index_tests {
    use super::*;
    #[test]
    fn index_org_member() {
        base::ticket_types::index(Roles::OrgMember, true);
    }
    #[test]
    fn index_admin() {
        base::ticket_types::index(Roles::Admin, true);
    }
    #[test]
    fn index_user() {
        base::ticket_types::index(Roles::User, false);
    }
    #[test]
    fn index_org_owner() {
        base::ticket_types::index(Roles::OrgOwner, true);
    }
    #[test]
    fn index_door_person() {
        base::ticket_types::index(Roles::DoorPerson, false);
    }
    #[test]
    fn index_promoter() {
        base::ticket_types::index(Roles::Promoter, true);
    }
    #[test]
    fn index_promoter_read_only() {
        base::ticket_types::index(Roles::PromoterReadOnly, true);
    }
    #[test]
    fn index_org_admin() {
        base::ticket_types::index(Roles::OrgAdmin, true);
    }
    #[test]
    fn index_box_office() {
        base::ticket_types::index(Roles::OrgBoxOffice, false);
    }
}

#[test]
pub fn create_with_validation_errors() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::Admin, None, &database);
    let event = database.create_event().with_organization(&organization).finish();
    //Construct Ticket creation and pricing request
    let test_request = TestRequest::create();
    let state = test_request.extract_state();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = event.id;
    let mut ticket_pricing: Vec<CreateTicketPricingRequest> = Vec::new();
    let start_date = NaiveDate::from_ymd(2018, 8, 1).and_hms(6, 20, 21);
    let end_date = NaiveDate::from_ymd(2018, 7, 3).and_hms(9, 23, 23);
    let start_date2 = NaiveDate::from_ymd(2018, 7, 1).and_hms(6, 20, 21);
    let end_date2 = NaiveDate::from_ymd(2018, 7, 3).and_hms(9, 23, 23);
    ticket_pricing.push(CreateTicketPricingRequest {
        name: String::from("Early bird"),
        price_in_cents: 10000,
        start_date: start_date2,
        end_date: end_date2,
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
        price_in_cents: 10000,
        visibility: TicketTypeVisibility::Always,
        parent_id: None,
        ..Default::default()
    };
    let response: HttpResponse =
        ticket_types::create((database.connection.into(), path, Json(request_data), auth_user, state)).into();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(response.error().is_some());
    let validation_response = support::validation_response_from_response(&response).unwrap();
    let start_date_errors = validation_response.fields.get("start_date").unwrap();
    assert_eq!(start_date_errors.len(), 1);
    assert_eq!(start_date_errors[0].code, "start_date_must_be_before_end_date");
    assert_eq!(
        &start_date_errors[0].message.clone().unwrap().into_owned(),
        "Start date must be before end date"
    );
}

#[test]
pub fn create_with_validation_errors_on_ticket_pricing() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::Admin, None, &database);
    let event = database.create_event().with_organization(&organization).finish();
    //Construct Ticket creation and pricing request
    let test_request = TestRequest::create();
    let state = test_request.extract_state();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = event.id;
    let mut ticket_pricing: Vec<CreateTicketPricingRequest> = Vec::new();
    let start_date = NaiveDate::from_ymd(2018, 7, 1).and_hms(6, 20, 21);
    let end_date = NaiveDate::from_ymd(2018, 7, 3).and_hms(9, 23, 23);
    let start_date2 = NaiveDate::from_ymd(2018, 8, 1).and_hms(6, 20, 21);
    let end_date2 = NaiveDate::from_ymd(2018, 7, 3).and_hms(9, 23, 23);
    ticket_pricing.push(CreateTicketPricingRequest {
        name: String::from("Early bird"),
        price_in_cents: 10000,
        start_date: start_date2,
        end_date: end_date2,
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
        price_in_cents: 10000,
        visibility: TicketTypeVisibility::Always,
        parent_id: None,
        ..Default::default()
    };
    let response: HttpResponse =
        ticket_types::create((database.connection.into(), path, Json(request_data), auth_user, state)).into();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(response.error().is_some());
    let validation_response = support::validation_response_from_response(&response).unwrap();
    let start_date_errors = validation_response.fields.get("ticket_pricing.start_date").unwrap();
    assert_eq!(start_date_errors.len(), 1);
    assert_eq!(start_date_errors[0].code, "start_date_must_be_before_end_date");
    assert_eq!(
        &start_date_errors[0].message.clone().unwrap().into_owned(),
        "Start date must be before end date"
    );
}

#[test]
pub fn create_with_overlapping_periods() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::Admin, None, &database);
    let event = database.create_event().with_organization(&organization).finish();
    //Construct Ticket creation and pricing request
    let test_request = TestRequest::create();
    let state = test_request.extract_state();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
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
        start_date,
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
        ..Default::default()
    };
    let response: HttpResponse =
        ticket_types::create((database.connection.into(), path, Json(request_data), auth_user, state)).into();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(response.error().is_some());
    let validation_response = support::validation_response_from_response(&response).unwrap();
    let ticket_pricing_errors = validation_response.fields.get("ticket_pricing").unwrap();
    assert_eq!(ticket_pricing_errors.len(), 2);
    assert_eq!(ticket_pricing_errors[0].code, "ticket_pricing_overlapping_periods");
    assert_eq!(
        &ticket_pricing_errors[0].message.clone().unwrap().into_owned(),
        "Ticket pricing dates overlap another ticket pricing period"
    );
    assert_eq!(ticket_pricing_errors[1].code, "ticket_pricing_overlapping_periods");
    assert_eq!(
        &ticket_pricing_errors[1].message.clone().unwrap().into_owned(),
        "Ticket pricing dates overlap another ticket pricing period"
    );
}

#[test]
pub fn create_with_out_of_bounds_ticket_capacity() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let event = database.create_event().with_organization(&organization).finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::Admin, None, &database);

    //Construct Ticket creation and pricing request
    let test_request = TestRequest::create();
    let state = test_request.extract_state();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
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
        capacity: 11000,
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
    };
    let response: HttpResponse =
        ticket_types::create((database.connection.into(), path, Json(request_data), auth_user, state)).into();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert!(response.error().is_some());
}

#[test]
pub fn update_with_invalid_id() {
    let database = TestDatabase::new();
    let request = TestRequest::create();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::Admin, None, &database);
    let event = database
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();

    //Retrieve created ticket type and pricing
    let conn = database.connection.get();
    let created_ticket_type = &event.ticket_types(true, None, conn).unwrap()[0];
    let created_ticket_capacity = created_ticket_type.valid_ticket_count(conn).unwrap();
    created_ticket_type.ticket_pricing(false, conn).unwrap();

    //Construct update request
    let test_request = TestRequest::create_with_uri_custom_params("/", vec!["event_id", "ticket_type_id"]);
    let mut path = Path::<EventTicketPathParameters>::extract(&test_request.request).unwrap();
    path.event_id = event.id;
    path.ticket_type_id = created_ticket_type.id;

    let mut request_ticket_pricing: Vec<UpdateTicketPricingRequest> = Vec::new();
    let start_date = Some(NaiveDate::from_ymd(2018, 5, 1).and_hms(6, 20, 21));
    let end_date = Some(NaiveDate::from_ymd(2018, 7, 3).and_hms(9, 23, 23));
    request_ticket_pricing.push(UpdateTicketPricingRequest {
        id: Some(Uuid::new_v4()),
        name: Some(String::from("Base")),
        start_date,
        end_date,
        price_in_cents: Some(20000),
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
        price_in_cents: Some(20000),
        parent_id: None,
        ..Default::default()
    };

    //Send update request
    let response: HttpResponse = ticket_types::update((
        database.connection.clone().into(),
        path,
        Json(request_data),
        auth_user,
        request.extract_state(),
    ))
    .into();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert!(response.error().is_some());
}

#[test]
pub fn update_with_validation_errors() {
    let database = TestDatabase::new();
    let request = TestRequest::create();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::Admin, None, &database);
    let event = database
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();

    //Retrieve created ticket type and pricing
    let conn = database.connection.get();
    let created_ticket_type = &event.ticket_types(true, None, conn).unwrap()[0];
    let created_ticket_capacity = created_ticket_type.valid_ticket_count(conn).unwrap();
    let created_ticket_pricing = created_ticket_type.ticket_pricing(false, conn).unwrap();

    //Construct update request
    let test_request = TestRequest::create_with_uri_custom_params("/", vec!["event_id", "ticket_type_id"]);
    let mut path = Path::<EventTicketPathParameters>::extract(&test_request.request).unwrap();
    path.event_id = event.id;
    path.ticket_type_id = created_ticket_type.id;

    let mut request_ticket_pricing: Vec<UpdateTicketPricingRequest> = Vec::new();
    let start_date = Some(NaiveDate::from_ymd(2018, 7, 1).and_hms(6, 20, 21));
    let end_date = Some(NaiveDate::from_ymd(2018, 6, 3).and_hms(9, 23, 23));
    let start_date2 = Some(NaiveDate::from_ymd(2018, 5, 1).and_hms(6, 20, 21));
    let end_date2 = Some(NaiveDate::from_ymd(2018, 7, 3).and_hms(9, 23, 23));
    request_ticket_pricing.push(UpdateTicketPricingRequest {
        id: Some(created_ticket_pricing[1].id),
        name: Some(String::from("Base")),
        start_date: start_date2,
        end_date: end_date2,
        price_in_cents: Some(20000),
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
        price_in_cents: Some(20000),
        parent_id: None,
        ..Default::default()
    };

    //Send update request
    let response: HttpResponse = ticket_types::update((
        database.connection.clone().into(),
        path,
        Json(request_data),
        auth_user,
        request.extract_state(),
    ))
    .into();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(response.error().is_some());
    let validation_response = support::validation_response_from_response(&response).unwrap();
    let start_date_errors = validation_response.fields.get("start_date").unwrap();
    assert_eq!(start_date_errors.len(), 1);
    assert_eq!(start_date_errors[0].code, "start_date_must_be_before_end_date");
    assert_eq!(
        &start_date_errors[0].message.clone().unwrap().into_owned(),
        "Start date must be before end date"
    );
}

#[test]
pub fn update_with_validation_errors_on_ticket_pricing() {
    let database = TestDatabase::new();
    let request = TestRequest::create();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::Admin, None, &database);
    let event = database
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();

    //Retrieve created ticket type and pricing
    let conn = database.connection.get();
    let created_ticket_type = &event.ticket_types(true, None, conn).unwrap()[0];
    let created_ticket_capacity = created_ticket_type.valid_ticket_count(conn).unwrap();
    let created_ticket_pricing = created_ticket_type.ticket_pricing(false, conn).unwrap();

    //Construct update request
    let test_request = TestRequest::create_with_uri_custom_params("/", vec!["event_id", "ticket_type_id"]);
    let mut path = Path::<EventTicketPathParameters>::extract(&test_request.request).unwrap();
    path.event_id = event.id;
    path.ticket_type_id = created_ticket_type.id;

    let mut request_ticket_pricing: Vec<UpdateTicketPricingRequest> = Vec::new();
    let start_date = Some(NaiveDate::from_ymd(2018, 5, 1).and_hms(6, 20, 21));
    let end_date = Some(NaiveDate::from_ymd(2018, 7, 3).and_hms(9, 23, 23));
    let start_date2 = Some(NaiveDate::from_ymd(2018, 7, 1).and_hms(6, 20, 21));
    let end_date2 = Some(NaiveDate::from_ymd(2018, 6, 3).and_hms(9, 23, 23));
    request_ticket_pricing.push(UpdateTicketPricingRequest {
        id: Some(created_ticket_pricing[1].id),
        name: Some(String::from("Base")),
        start_date: start_date2,
        end_date: end_date2,
        price_in_cents: Some(20000),
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
        price_in_cents: Some(20000),
        parent_id: None,
        ..Default::default()
    };

    //Send update request
    let response: HttpResponse = ticket_types::update((
        database.connection.clone().into(),
        path,
        Json(request_data),
        auth_user,
        request.extract_state(),
    ))
    .into();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(response.error().is_some());
    let validation_response = support::validation_response_from_response(&response).unwrap();
    let start_date_errors = validation_response.fields.get("ticket_pricing.start_date").unwrap();
    assert_eq!(start_date_errors.len(), 1);
    assert_eq!(start_date_errors[0].code, "start_date_must_be_before_end_date");
    assert_eq!(
        &start_date_errors[0].message.clone().unwrap().into_owned(),
        "Start date must be before end date"
    );
}

#[test]
pub fn update_with_overlapping_periods() {
    let database = TestDatabase::new();
    let request = TestRequest::create();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::Admin, None, &database);
    let event = database
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();

    //Retrieve created ticket type and pricing
    let conn = database.connection.get();
    let created_ticket_type = &event.ticket_types(true, None, conn).unwrap()[0];
    let created_ticket_capacity = created_ticket_type.valid_ticket_count(conn).unwrap();
    let created_ticket_pricing = created_ticket_type.ticket_pricing(false, conn).unwrap();

    //Construct update request
    let test_request = TestRequest::create_with_uri_custom_params("/", vec!["event_id", "ticket_type_id"]);
    let mut path = Path::<EventTicketPathParameters>::extract(&test_request.request).unwrap();
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
        start_date,
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
        price_in_cents: Some(20000),
        parent_id: None,
        ..Default::default()
    };

    //Send update request
    let response: HttpResponse = ticket_types::update((
        database.connection.clone().into(),
        path,
        Json(request_data),
        auth_user,
        request.extract_state(),
    ))
    .into();

    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(response.error().is_some());

    #[derive(Deserialize)]
    struct Response {
        error: String,
    }
    let deserialized_response: Response = serde_json::from_str(&body).unwrap();
    assert_eq!(deserialized_response.error, "Validation error");
}

#[test]
pub fn cancel_with_sold_tickets_and_hold() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::Admin, Some(&organization), &database);
    let event = database
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();

    let conn = database.connection.get();
    let created_ticket_type = &event.ticket_types(true, None, conn).unwrap()[0];

    let valid_unsold_ticket_count = created_ticket_type.valid_unsold_ticket_count(conn).unwrap();
    // 100 before taking tickets out of available inventory
    assert_eq!(100, valid_unsold_ticket_count);

    // Hold of 10 tickets
    let hold = database
        .create_hold()
        .with_ticket_type_id(created_ticket_type.id)
        .with_event(&event)
        .finish();

    let user2 = database.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user2, conn).unwrap();
    cart.update_quantities(
        user2.id,
        &vec![
            UpdateOrderItem {
                ticket_type_id: created_ticket_type.id,
                quantity: 10,
                redemption_code: None,
            },
            UpdateOrderItem {
                ticket_type_id: created_ticket_type.id,
                quantity: 5,
                redemption_code: hold.redemption_code,
            },
        ],
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

    let valid_unsold_ticket_count = created_ticket_type.valid_unsold_ticket_count(conn).unwrap();
    // 85 left from 100 - 5 (hold) - 10 (regular)
    assert_eq!(85, valid_unsold_ticket_count);

    //Construct update request
    let test_request = TestRequest::create_with_uri_custom_params("/", vec!["event_id", "ticket_type_id"]);
    let state = test_request.extract_state();
    let mut path = Path::<EventTicketPathParameters>::extract(&test_request.request).unwrap();
    path.event_id = event.id;
    path.ticket_type_id = created_ticket_type.id;

    //Send update request
    let response: HttpResponse =
        ticket_types::cancel((database.connection.clone().into(), path, auth_user, state)).into();

    let updated_ticket_type = &event.ticket_types(true, None, conn).unwrap()[0];

    assert_eq!(updated_ticket_type.status, TicketTypeStatus::Cancelled);
    assert_eq!(response.status(), StatusCode::OK);

    let valid_unsold_ticket_count = created_ticket_type.valid_unsold_ticket_count(conn).unwrap();
    assert_eq!(0, valid_unsold_ticket_count);

    let valid_ticket_count = created_ticket_type.valid_ticket_count(conn).unwrap();
    assert_eq!(15, valid_ticket_count);
}

#[test]
pub fn cancel_with_no_sold_tickets_or_hold() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::Admin, Some(&organization), &database);
    let event = database
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();

    let conn = database.connection.get();
    let created_ticket_type = &event.ticket_types(true, None, conn).unwrap()[0];

    let valid_unsold_ticket_count = created_ticket_type.valid_unsold_ticket_count(conn).unwrap();
    // 100 before taking tickets out of available inventory
    assert_eq!(100, valid_unsold_ticket_count);

    //Construct update request
    let test_request = TestRequest::create_with_uri_custom_params("/", vec!["event_id", "ticket_type_id"]);
    let state = test_request.extract_state();
    let mut path = Path::<EventTicketPathParameters>::extract(&test_request.request).unwrap();
    path.event_id = event.id;
    path.ticket_type_id = created_ticket_type.id;

    //Send update request
    let response: HttpResponse =
        ticket_types::cancel((database.connection.clone().into(), path, auth_user, state)).into();

    let updated_ticket_types = &event.ticket_types(true, None, conn).unwrap();

    assert_eq!(updated_ticket_types.len(), 0);
    assert_eq!(response.status(), StatusCode::OK);
}
