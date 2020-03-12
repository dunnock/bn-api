use crate::support;
use crate::support::database::TestDatabase;
use crate::support::test_request::TestRequest;
use actix_web::{FromRequest, http::StatusCode, HttpResponse, web::Path};
use bigneon_api::controllers::event_report_subscribers::{self, NewEventReportSubscriberRequest};
use bigneon_api::extractors::*;
use bigneon_api::models::PathParameters;
use bigneon_db::models::*;
use std::collections::HashMap;

pub async fn index(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let event = database.create_event().finish();
    let organization = event.organization(connection).unwrap();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);
    let event_report_subscriber1 = database
        .create_event_report_subscriber()
        .with_event(&event)
        .with_email("email1@tari.com")
        .finish();
    let event_report_subscriber2 = database
        .create_event_report_subscriber()
        .with_event(&event)
        .with_email("email2@tari.com")
        .finish();
    let expected_event_report_subscribers = vec![event_report_subscriber1, event_report_subscriber2];

    let test_request = TestRequest::create_with_uri(&format!("/limits?"));
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = event.id;

    let response = event_report_subscribers::index((database.connection.clone().into(), path, auth_user)).await;
    let counter = expected_event_report_subscribers.len() as u32;
    let wrapped_expected_orgs = Payload {
        data: expected_event_report_subscribers,
        paging: Paging {
            page: 0,
            limit: counter,
            sort: "".to_string(),
            dir: SortingDir::Asc,
            total: counter as u64,
            tags: HashMap::new(),
        },
    };

    if should_test_succeed {
        let response = response.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(wrapped_expected_orgs, *response.payload());
    } else {
        assert_eq!(
            response.err().unwrap().to_string(),
            "User does not have the required permissions"
        );
    }
}

pub async fn create(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let event = database.create_event().finish();
    let organization = event.organization(connection).unwrap();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);
    let email = "email@address.com".to_string();

    let json = Json(NewEventReportSubscriberRequest {
        email: email.clone(),
        report_type: ReportTypes::TicketCounts,
    });

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = event.id;

    let response = event_report_subscribers::create((database.connection.clone().into(), json, path, auth_user)).await;

    if should_test_succeed {
        let response = response.unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let event_report_subscriber = response.data();
        assert_eq!(event_report_subscriber.event_id, event.id);
        assert_eq!(event_report_subscriber.email, email);
        assert_eq!(event_report_subscriber.report_type, ReportTypes::TicketCounts);
    } else {
        assert_eq!(
            response.err().unwrap().to_string(),
            "User does not have the required permissions",
        );
    }
}

pub async fn destroy(role: Roles, should_succeed: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let event_report_subscriber = database.create_event_report_subscriber().finish();
    let event = Event::find(event_report_subscriber.event_id, connection).unwrap();
    let organization = event.organization(connection).unwrap();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);

    let test_request = TestRequest::create_with_uri_custom_params("/", vec!["id"]);
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = event_report_subscriber.id;

    let response: HttpResponse =
        event_report_subscribers::destroy((database.connection.clone().into(), path, auth_user)).await.into();

    if should_succeed {
        assert_eq!(response.status(), StatusCode::OK);
        let event_report_subscriber = EventReportSubscriber::find(event_report_subscriber.id, connection);
        assert!(event_report_subscriber.is_err());
    } else {
        support::expects_unauthorized(&response);
    }
}
