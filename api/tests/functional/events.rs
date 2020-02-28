use actix_web::Query;
use actix_web::{http::StatusCode, FromRequest, HttpResponse, Path};
use bigneon_api::controllers::events;
use bigneon_api::controllers::events::*;
use bigneon_api::extractors::*;
use bigneon_api::models::*;
use bigneon_db::models::*;
use bigneon_db::utils::dates;
use chrono::prelude::*;
use chrono::Duration;
use diesel::PgConnection;
use functional::base;
use serde_json;
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use support;
use support::database::TestDatabase;
use support::test_request::{RequestBuilder, TestRequest};
use uuid::Uuid;

#[test]
pub fn index() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let organization = database.create_organization().finish();
    let venue = database.create_venue().finish();
    let event = database
        .create_event()
        .with_name("NewEvent1".to_string())
        .with_organization(&organization)
        .with_venue(&venue)
        .with_event_start(NaiveDate::from_ymd(2022, 7, 8).and_hms(9, 10, 11))
        .with_event_end(NaiveDate::from_ymd(2022, 7, 9).and_hms(9, 10, 11))
        .finish();
    let event2 = database
        .create_event()
        .with_name("NewEvent2".to_string())
        .with_organization(&organization)
        .with_venue(&venue)
        .with_event_start(NaiveDate::from_ymd(2022, 7, 8).and_hms(9, 10, 11))
        .with_event_end(NaiveDate::from_ymd(2022, 7, 9).and_hms(9, 10, 11))
        .finish();
    let _event3 = database
        .create_event()
        .with_name("NewEvent3".to_string())
        .with_organization(&organization)
        .with_venue(&venue)
        .with_event_start(NaiveDate::from_ymd(2022, 7, 8).and_hms(9, 10, 11))
        .with_event_end(NaiveDate::from_ymd(2022, 7, 9).and_hms(9, 10, 11))
        .as_private("access".to_string())
        .finish();
    let user = database.create_user().finish();
    // Deleted event
    let event4 = database
        .create_event()
        .with_name("NewEvent4".to_string())
        .with_organization(&organization)
        .with_venue(&venue)
        .with_event_start(NaiveDate::from_ymd(2022, 7, 8).and_hms(9, 10, 11))
        .with_event_end(NaiveDate::from_ymd(2022, 7, 9).and_hms(9, 10, 11))
        .finish();
    event4.delete(user.id, connection).unwrap();

    let expected_results = vec![
        event_venue_entry(&event, &venue, &vec![], None, &*connection),
        event_venue_entry(&event2, &venue, &vec![], None, &*connection),
    ];

    let test_request = TestRequest::create_with_uri("/events?query=New");
    let parameters = Query::<SearchParameters>::extract(&test_request.request).unwrap();
    let response: HttpResponse = events::index((
        test_request.extract_state(),
        database.connection.clone().into(),
        parameters,
        OptionalUser(None),
    ))
    .into();

    let body = support::unwrap_body_to_string(&response).unwrap();

    let mut expected_tags: HashMap<String, Value> = HashMap::new();
    expected_tags.insert("query".to_string(), json!("New"));
    let wrapped_expected_events = Payload {
        data: expected_results,
        paging: Paging {
            page: 0,
            limit: 50,
            sort: "".to_string(),
            dir: SortingDir::Asc,
            total: 2,
            tags: expected_tags,
        },
    };
    let expected_json = serde_json::to_string(&wrapped_expected_events).unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body, expected_json);

    // cache headers
    //    let mut headers = response.headers().clone();
    //    let cache_control_headers = headers.entry("Cache-Control");
    //    assert!(cache_control_headers.is_ok());
}

#[test]
pub fn index_for_user() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let organization = database.create_organization().finish();
    let user = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);
    let venue = database.create_venue().finish();
    let event = database
        .create_event()
        .with_name("NewEvent1".to_string())
        .with_organization(&organization)
        .with_venue(&venue)
        .with_event_start(NaiveDate::from_ymd(2022, 7, 8).and_hms(9, 10, 11))
        .with_event_end(NaiveDate::from_ymd(2022, 7, 9).and_hms(9, 10, 11))
        .finish();
    let _event_interest = EventInterest::create(event.id, user.id).commit(connection);
    let event2 = database
        .create_event()
        .with_name("NewEvent2".to_string())
        .with_organization(&organization)
        .with_venue(&venue)
        .with_event_start(NaiveDate::from_ymd(2022, 7, 8).and_hms(9, 10, 11))
        .with_event_end(NaiveDate::from_ymd(2022, 7, 9).and_hms(9, 10, 11))
        .finish();
    // Deleted event
    let event3 = database
        .create_event()
        .with_name("NewEvent3".to_string())
        .with_organization(&organization)
        .with_venue(&venue)
        .with_event_start(NaiveDate::from_ymd(2022, 7, 8).and_hms(9, 10, 11))
        .with_event_end(NaiveDate::from_ymd(2022, 7, 9).and_hms(9, 10, 11))
        .finish();
    event3.delete(user.id, connection).unwrap();
    //Event that has ended
    let _ended_event = database
        .create_event()
        .with_name("EndedEvent".to_string())
        .with_organization(&organization)
        .with_venue(&venue)
        .with_event_start(NaiveDate::from_ymd(2018, 7, 8).and_hms(9, 10, 11))
        .with_event_end(NaiveDate::from_ymd(2018, 7, 9).and_hms(9, 10, 11))
        .finish();

    let expected_results = vec![
        event_venue_entry(&event, &venue, &vec![], Some(user.clone()), &*connection),
        event_venue_entry(&event2, &venue, &vec![], Some(user), &*connection),
    ];

    let test_request = TestRequest::create_with_uri("/events?query=New");
    let parameters = Query::<SearchParameters>::extract(&test_request.request).unwrap();
    let response: HttpResponse = events::index((
        test_request.extract_state(),
        database.connection.clone().into(),
        parameters,
        OptionalUser(Some(auth_user)),
    ))
    .into();

    let body = support::unwrap_body_to_string(&response).unwrap();

    let mut expected_tags: HashMap<String, Value> = HashMap::new();
    expected_tags.insert("query".to_string(), json!("New"));
    let wrapped_expected_events = Payload {
        data: expected_results,
        paging: Paging {
            page: 0,
            limit: 50,
            sort: "".to_string(),
            dir: SortingDir::Asc,
            total: 2,
            tags: expected_tags,
        },
    };
    let expected_json = serde_json::to_string(&wrapped_expected_events).unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body, expected_json);
}

#[test]
pub fn index_with_draft_for_organization_user() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::OrgMember, Some(&organization), &database);
    let venue = database.create_venue().finish();
    let event = database
        .create_event()
        .with_name("NewEvent1".to_string())
        .with_organization(&organization)
        .with_venue(&venue)
        .with_event_start(NaiveDate::from_ymd(2022, 7, 8).and_hms(9, 10, 11))
        .with_event_end(NaiveDate::from_ymd(2022, 7, 9).and_hms(9, 10, 11))
        .finish();
    let event2 = database
        .create_event()
        .with_name("NewEvent2".to_string())
        .with_status(EventStatus::Draft)
        .with_organization(&organization)
        .with_venue(&venue)
        .with_event_start(NaiveDate::from_ymd(2022, 7, 8).and_hms(9, 10, 11))
        .with_event_end(NaiveDate::from_ymd(2022, 7, 9).and_hms(9, 10, 11))
        .finish();

    let expected_results = vec![
        event_venue_entry(&event, &venue, &vec![], None, &*connection),
        event_venue_entry(&event2, &venue, &vec![], None, &*connection),
    ];

    let test_request = TestRequest::create_with_uri("/events?query=New");
    let parameters = Query::<SearchParameters>::extract(&test_request.request).unwrap();
    let response: HttpResponse = events::index((
        test_request.extract_state(),
        database.connection.clone().into(),
        parameters,
        OptionalUser(Some(auth_user)),
    ))
    .into();

    let body = support::unwrap_body_to_string(&response).unwrap();

    let mut expected_tags: HashMap<String, Value> = HashMap::new();
    expected_tags.insert("query".to_string(), json!("New"));
    let wrapped_expected_events = Payload {
        data: expected_results,
        paging: Paging {
            page: 0,
            limit: 50,
            sort: "".to_string(),
            dir: SortingDir::Asc,
            total: 2,
            tags: expected_tags,
        },
    };
    let expected_json = serde_json::to_string(&wrapped_expected_events).unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body, expected_json);
}

#[test]
pub fn index_with_draft_for_user_ignores_drafts() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);
    let organization = database.create_organization().finish();
    let venue = database.create_venue().finish();
    let event = database
        .create_event()
        .with_name("NewEvent1".to_string())
        .with_organization(&organization)
        .with_venue(&venue)
        .with_event_start(NaiveDate::from_ymd(2022, 7, 8).and_hms(9, 10, 11))
        .with_event_end(NaiveDate::from_ymd(2022, 7, 9).and_hms(9, 10, 11))
        .finish();
    let _event2 = database
        .create_event()
        .with_name("NewEvent2".to_string())
        .with_status(EventStatus::Draft)
        .with_organization(&organization)
        .with_venue(&venue)
        .with_event_start(NaiveDate::from_ymd(2022, 7, 8).and_hms(9, 10, 11))
        .with_event_end(NaiveDate::from_ymd(2022, 7, 9).and_hms(9, 10, 11))
        .finish();

    let expected_results = vec![event_venue_entry(&event, &venue, &vec![], None, &*connection)];

    let test_request = TestRequest::create_with_uri("/events?query=New");
    let parameters = Query::<SearchParameters>::extract(&test_request.request).unwrap();
    let response: HttpResponse = events::index((
        test_request.extract_state(),
        database.connection.clone().into(),
        parameters,
        OptionalUser(Some(auth_user)),
    ))
    .into();

    let body = support::unwrap_body_to_string(&response).unwrap();
    let mut expected_tags: HashMap<String, Value> = HashMap::new();
    expected_tags.insert("query".to_string(), json!("New"));
    let wrapped_expected_events = Payload {
        data: expected_results,
        paging: Paging {
            page: 0,
            limit: 50,
            sort: "".to_string(),
            dir: SortingDir::Asc,
            total: 1,
            tags: expected_tags,
        },
    };
    let expected_json = serde_json::to_string(&wrapped_expected_events).unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body, expected_json);
}

#[test]
pub fn index_search_with_filter() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let organization = database.create_organization().finish();
    let event = database
        .create_event()
        .with_name("NewEvent1".to_string())
        .with_organization(&organization)
        .with_event_start(NaiveDate::from_ymd(2022, 7, 8).and_hms(9, 10, 11))
        .with_event_end(NaiveDate::from_ymd(2022, 7, 9).and_hms(9, 10, 11))
        .finish();
    database
        .create_event()
        .with_name("NewEvent2".to_string())
        .with_organization(&organization)
        .with_event_start(NaiveDate::from_ymd(2022, 7, 8).and_hms(9, 10, 11))
        .with_event_end(NaiveDate::from_ymd(2022, 7, 9).and_hms(9, 10, 11))
        .finish();

    let localized_times = event.get_all_localized_time_strings(None);
    let slug = event.slug(connection).unwrap();
    let expected_events = vec![EventVenueEntry {
        id: event.id,
        name: event.name,
        organization_id: event.organization_id,
        venue_id: event.venue_id,
        created_at: event.created_at,
        updated_at: event.updated_at,
        event_start: event.event_start,
        door_time: event.door_time,
        status: event.status,
        publish_date: event.publish_date,
        promo_image_url: event.promo_image_url.clone(),
        original_promo_image_url: event.promo_image_url,
        additional_info: event.additional_info,
        top_line_info: event.top_line_info,
        age_limit: event.age_limit,
        cancelled_at: event.cancelled_at,
        venue: None,
        artists: Some(vec![]),
        min_ticket_price: None,
        max_ticket_price: None,
        is_external: false,
        external_url: None,
        user_is_interested: false,
        localized_times,
        tracking_keys: TrackingKeys { ..Default::default() },
        event_type: EventTypes::Music,
        url: format!("{}/tickets/{}", env::var("FRONT_END_URL").unwrap(), &slug),
        slug,
        event_end: event.event_end,
    }];

    let test_request = TestRequest::create_with_uri("/events?query=NewEvent1");
    let parameters = Query::<SearchParameters>::extract(&test_request.request).unwrap();
    let response: HttpResponse = events::index((
        test_request.extract_state(),
        database.connection.clone().into(),
        parameters,
        OptionalUser(None),
    ))
    .into();

    let body = support::unwrap_body_to_string(&response).unwrap();
    let mut expected_tags: HashMap<String, Value> = HashMap::new();
    expected_tags.insert("query".to_string(), json!("NewEvent1"));
    let wrapped_expected_events = Payload {
        data: expected_events,
        paging: Paging {
            page: 0,
            limit: 50,
            sort: "".to_string(),
            dir: SortingDir::Asc,
            total: 1,
            tags: expected_tags,
        },
    };
    let expected_json = serde_json::to_string(&wrapped_expected_events).unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body, expected_json);
}

#[test]
fn show() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);

    let organization = database.create_organization().finish();
    let venue = database.create_venue().finish();
    let event = database
        .create_event()
        .with_name("NewEvent".to_string())
        .with_organization(&organization)
        .with_venue(&venue)
        .with_ticket_pricing()
        .finish();
    let event_id = event.id;

    let artist1 = database.create_artist().finish();
    let artist2 = database.create_artist().finish();
    let conn = database.connection.get();

    event.add_artist(None, artist1.id, conn).unwrap();
    event.add_artist(None, artist2.id, conn).unwrap();

    let _event_interest = EventInterest::create(event.id, user.id).commit(conn);
    let test_request = TestRequest::create_with_uri(&format!("/events/{}", event.id));
    let mut path = Path::<StringPathParameters>::extract(&test_request.request).unwrap();
    let event_expected_json = base::events::expected_show_json(
        Roles::User,
        event,
        organization,
        venue,
        false,
        None,
        None,
        conn,
        1,
        None,
    );
    path.id = event_id.to_string();
    let query_parameters = Query::<EventParameters>::extract(&test_request.request).unwrap();

    let response: HttpResponse = events::show((
        test_request.extract_state(),
        database.connection.clone().into(),
        path,
        query_parameters,
        OptionalUser(Some(auth_user)),
        RequestInfo {
            user_agent: Some("test".to_string()),
        },
    ))
    .into();
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body, event_expected_json);
}

#[test]
fn show_ended_event() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);

    let organization = database.create_organization().finish();
    let venue = database.create_venue().finish();
    let event = database
        .create_event()
        .with_name("NewEvent".to_string())
        .with_organization(&organization)
        .with_venue(&venue)
        .with_ticket_pricing()
        .with_event_start(NaiveDate::from_ymd(2018, 7, 8).and_hms(9, 10, 11))
        .with_event_end(NaiveDate::from_ymd(2018, 7, 9).and_hms(9, 10, 11))
        .finish();
    let event_id = event.id;

    let artist1 = database.create_artist().finish();
    let artist2 = database.create_artist().finish();
    let conn = database.connection.get();

    event.add_artist(None, artist1.id, conn).unwrap();
    event.add_artist(None, artist2.id, conn).unwrap();

    let _event_interest = EventInterest::create(event.id, user.id).commit(conn);
    let test_request = TestRequest::create_with_uri(&format!("/tickets/{}", event.id));
    let mut path = Path::<StringPathParameters>::extract(&test_request.request).unwrap();
    let event_expected_json = base::events::expected_show_json(
        Roles::User,
        event,
        organization,
        venue,
        false,
        None,
        None,
        conn,
        1,
        Some(EventStatus::Closed),
    );
    path.id = event_id.to_string();
    let query_parameters = Query::<EventParameters>::extract(&test_request.request).unwrap();

    let response: HttpResponse = events::show((
        test_request.extract_state(),
        database.connection.clone().into(),
        path,
        query_parameters,
        OptionalUser(Some(auth_user)),
        RequestInfo {
            user_agent: Some("test".to_string()),
        },
    ))
    .into();
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body, event_expected_json);
}

#[test]
fn show_future_published_no_preview() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);
    let organization = database.create_organization().finish();
    let venue = database.create_venue().finish();
    let event = database
        .create_event()
        .with_name("PublicEvent".to_string())
        .with_organization(&organization)
        .with_venue(&venue)
        .with_ticket_pricing()
        .with_publish_date(dates::now().add_hours(10).finish())
        .finish();
    let event_id = event.id;

    let test_request = TestRequest::create_with_uri(&format!("/events/{}", event_id));
    let mut path = Path::<StringPathParameters>::extract(&test_request.request).unwrap();
    path.id = event_id.to_string();
    let query_parameters = Query::<EventParameters>::extract(&test_request.request).unwrap();
    let response: HttpResponse = events::show((
        test_request.extract_state(),
        database.connection.clone().into(),
        path,
        query_parameters,
        OptionalUser(Some(auth_user.clone())),
        RequestInfo {
            user_agent: Some("test".to_string()),
        },
    ))
    .into();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[test]
fn show_from_slug() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);
    let conn = database.connection.get();
    let organization = database.create_organization().finish();
    let venue = database.create_venue().finish();
    let event1 = database
        .create_event()
        .with_name("NewEvent1".to_string())
        .with_organization(&organization)
        .with_venue(&venue)
        .finish();
    let _event_interest = EventInterest::create(event1.id, user.id).commit(conn);
    let slug1 = "newevent1-san-francisco";
    let test_request = TestRequest::create_with_uri(&format!("/events/{}", slug1));
    let mut path = Path::<StringPathParameters>::extract(&test_request.request).unwrap();
    let event_expected_json = base::events::expected_show_json(
        Roles::User,
        event1.clone(),
        organization.clone(),
        venue.clone(),
        false,
        None,
        None,
        conn,
        1,
        None,
    );
    path.id = slug1.to_string();
    let query_parameters = Query::<EventParameters>::extract(&test_request.request).unwrap();

    let response: HttpResponse = events::show((
        test_request.extract_state(),
        database.connection.clone().into(),
        path,
        query_parameters,
        OptionalUser(Some(auth_user.clone())),
        RequestInfo {
            user_agent: Some("test".to_string()),
        },
    ))
    .into();
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body, event_expected_json);

    //Now delete the event
    event1.delete(user.id, conn).unwrap();
    let slug1 = "newevent1-at-name-san-francisco";
    let test_request = TestRequest::create_with_uri(&format!("/events/{}", slug1));
    let mut path = Path::<StringPathParameters>::extract(&test_request.request).unwrap();
    path.id = slug1.to_string();
    let query_parameters = Query::<EventParameters>::extract(&test_request.request).unwrap();

    let response: HttpResponse = events::show((
        test_request.extract_state(),
        database.connection.clone().into(),
        path,
        query_parameters,
        OptionalUser(Some(auth_user.clone())),
        RequestInfo {
            user_agent: Some("test".to_string()),
        },
    ))
    .into();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    //Now try recreate the event with the same name, the slug should change
    let event2 = database
        .create_event()
        .with_name("NewEvent1".to_string())
        .with_organization(&organization)
        .with_venue(&venue)
        .finish();
    let _event_interest = EventInterest::create(event2.id, user.id).commit(conn);
    let slug2 = event2.clone().slug(conn).unwrap();
    let test_request = TestRequest::create_with_uri(&format!("/events/{}", slug2));
    let mut path = Path::<StringPathParameters>::extract(&test_request.request).unwrap();
    let event_expected_json = base::events::expected_show_json(
        Roles::User,
        event2.clone(),
        organization.clone(),
        venue.clone(),
        false,
        None,
        None,
        conn,
        1,
        None,
    );
    path.id = slug2.to_string();
    let query_parameters = Query::<EventParameters>::extract(&test_request.request).unwrap();

    let response: HttpResponse = events::show((
        test_request.extract_state(),
        database.connection.clone().into(),
        path,
        query_parameters,
        OptionalUser(Some(auth_user.clone())),
        RequestInfo {
            user_agent: Some("test".to_string()),
        },
    ))
    .into();
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body, event_expected_json);
}

#[test]
fn show_future_published_with_preview() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let organization = database.create_organization().finish();
    let user = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::OrgMember, Some(&organization), &database);
    let venue = database.create_venue().finish();
    let event = database
        .create_event()
        .with_name("PublicEvent".to_string())
        .with_organization(&organization)
        .with_venue(&venue)
        .with_ticket_pricing()
        .with_publish_date(dates::now().add_hours(10).finish())
        .finish();
    let event_id = event.id;
    let event_expected_json = base::events::expected_show_json(
        Roles::OrgMember,
        event,
        organization.clone(),
        venue.clone(),
        false,
        None,
        None,
        connection,
        1,
        None,
    );

    EventInterest::create(event_id, user.id).commit(connection).unwrap();
    let test_request = TestRequest::create_with_uri(&format!("/events/{}", event_id));
    let mut path = Path::<StringPathParameters>::extract(&test_request.request).unwrap();
    path.id = event_id.to_string();
    let query_parameters = Query::<EventParameters>::extract(&test_request.request).unwrap();
    let response: HttpResponse = events::show((
        test_request.extract_state(),
        database.connection.clone().into(),
        path,
        query_parameters,
        OptionalUser(Some(auth_user.clone())),
        RequestInfo {
            user_agent: Some("test".to_string()),
        },
    ))
    .into();
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body, event_expected_json);
}

#[test]
fn show_deleted_event() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);
    let organization = database.create_organization().finish();
    let venue = database.create_venue().finish();
    let event = database
        .create_event()
        .with_name("PublicEvent".to_string())
        .with_organization(&organization)
        .with_venue(&venue)
        .with_ticket_pricing()
        .finish();
    let event_id = event.id;
    event.delete(user.id, connection).unwrap();

    let test_request = TestRequest::create_with_uri(&format!("/events/{}", event_id));
    let mut path = Path::<StringPathParameters>::extract(&test_request.request).unwrap();
    path.id = event_id.to_string();
    let query_parameters = Query::<EventParameters>::extract(&test_request.request).unwrap();
    let response: HttpResponse = events::show((
        test_request.extract_state(),
        database.connection.clone().into(),
        path,
        query_parameters,
        OptionalUser(Some(auth_user.clone())),
        RequestInfo {
            user_agent: Some("test".to_string()),
        },
    ))
    .into();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[test]
fn show_private() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);

    let organization = database.create_organization().finish();

    let org_user = database.create_user().finish();
    let auth_org_user =
        support::create_auth_user_from_user(&org_user, Roles::OrgMember, Some(&organization), &database);
    let venue = database.create_venue().finish();
    let event = database
        .create_event()
        .with_name("PublicEvent".to_string())
        .with_organization(&organization)
        .with_venue(&venue)
        .with_ticket_pricing()
        .finish();
    let event_id = event.id;

    let private_event = database
        .create_event()
        .with_name("PrivateEvent".to_string())
        .with_organization(&organization)
        .with_venue(&venue)
        .with_ticket_pricing()
        .as_private("access".to_string())
        .finish();
    let private_event_id = private_event.id;

    let artist1 = database.create_artist().finish();
    let artist2 = database.create_artist().finish();
    let conn = database.connection.get();

    event.add_artist(None, artist1.id, conn).unwrap();
    event.add_artist(None, artist2.id, conn).unwrap();

    let _event_interest = EventInterest::create(event.id, user.id).commit(conn);
    let test_request = TestRequest::create_with_uri(&format!("/events/{}", event.id));
    let mut path = Path::<StringPathParameters>::extract(&test_request.request).unwrap();
    let event_expected_json = base::events::expected_show_json(
        Roles::User,
        event,
        organization.clone(),
        venue.clone(),
        false,
        None,
        None,
        conn,
        1,
        None,
    );
    path.id = event_id.to_string();
    let query_parameters = Query::<EventParameters>::extract(&test_request.request).unwrap();

    let response: HttpResponse = events::show((
        test_request.extract_state(),
        database.connection.clone().into(),
        path,
        query_parameters,
        OptionalUser(Some(auth_user.clone())),
        RequestInfo {
            user_agent: Some("test".to_string()),
        },
    ))
    .into();
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body, event_expected_json);

    let _event_interest = EventInterest::create(private_event_id, org_user.id).commit(conn);
    let _event_interest = EventInterest::create(private_event_id, user.id).commit(conn);
    let test_request = TestRequest::create_with_uri(&format!("/events/{}", private_event_id));
    let mut path = Path::<StringPathParameters>::extract(&test_request.request).unwrap();
    let event_expected_json = base::events::expected_show_json(
        Roles::OrgMember,
        private_event.clone(),
        organization.clone(),
        venue.clone(),
        false,
        None,
        None,
        conn,
        2,
        None,
    );
    path.id = private_event_id.to_string();
    let query_parameters = Query::<EventParameters>::extract(&test_request.request).unwrap();

    let response: HttpResponse = events::show((
        test_request.extract_state(),
        database.connection.clone().into(),
        path,
        query_parameters,
        OptionalUser(Some(auth_org_user)),
        RequestInfo {
            user_agent: Some("test".to_string()),
        },
    ))
    .into();
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body, event_expected_json);

    let test_request = TestRequest::create_with_uri(&format!("/events/{}", private_event_id));
    let mut path = Path::<StringPathParameters>::extract(&test_request.request).unwrap();
    path.id = private_event_id.to_string();
    let query_parameters = Query::<EventParameters>::extract(&test_request.request).unwrap();
    let response: HttpResponse = events::show((
        test_request.extract_state(),
        database.connection.clone().into(),
        path,
        query_parameters,
        OptionalUser(Some(auth_user.clone())),
        RequestInfo {
            user_agent: Some("test".to_string()),
        },
    ))
    .into();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let test_request = TestRequest::create_with_uri(&format!(
        "/events/{}?private_access_code={}",
        private_event_id,
        "access".to_string()
    ));
    let mut path = Path::<StringPathParameters>::extract(&test_request.request).unwrap();
    path.id = private_event_id.to_string();
    let query_parameters = Query::<EventParameters>::extract(&test_request.request).unwrap();
    let response: HttpResponse = events::show((
        test_request.extract_state(),
        database.connection.clone().into(),
        path,
        query_parameters,
        OptionalUser(Some(auth_user)),
        RequestInfo {
            user_agent: Some("test".to_string()),
        },
    ))
    .into();

    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let event_expected_json = base::events::expected_show_json(
        Roles::User,
        private_event,
        organization,
        venue,
        false,
        None,
        None,
        conn,
        2,
        None,
    );
    assert_eq!(body, event_expected_json);
}

#[test]
fn show_with_cancelled_ticket_type() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);

    let organization = database.create_organization().finish();
    let venue = database.create_venue().finish();
    let event = database
        .create_event()
        .with_name("NewEvent".to_string())
        .with_organization(&organization)
        .with_venue(&venue)
        .with_ticket_pricing()
        .with_ticket_type_count(2)
        .finish();
    let event_id = event.id;

    let artist1 = database.create_artist().finish();
    let artist2 = database.create_artist().finish();
    let conn = database.connection.get();

    event.add_artist(None, artist1.id, conn).unwrap();
    event.add_artist(None, artist2.id, conn).unwrap();

    let ticket_type = event.ticket_types(true, None, conn).unwrap().remove(0);
    ticket_type.cancel(conn).unwrap();

    EventInterest::create(event.id, user.id).commit(conn).unwrap();
    let test_request = TestRequest::create_with_uri(&format!("/events/{}", event.id));
    let mut path = Path::<StringPathParameters>::extract(&test_request.request).unwrap();
    let event_expected_json = base::events::expected_show_json(
        Roles::User,
        event,
        organization,
        venue,
        false,
        None,
        None,
        conn,
        1,
        None,
    );
    path.id = event_id.to_string();
    let query_parameters = Query::<EventParameters>::extract(&test_request.request).unwrap();

    let response: HttpResponse = events::show((
        test_request.extract_state(),
        database.connection.clone().into(),
        path,
        query_parameters,
        OptionalUser(Some(auth_user)),
        RequestInfo {
            user_agent: Some("test".to_string()),
        },
    ))
    .into();
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body, event_expected_json);
}

#[test]
fn show_with_access_restricted_ticket_type_and_no_code() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);

    let organization = database.create_organization().finish();
    let venue = database.create_venue().finish();
    let event = database
        .create_event()
        .with_name("NewEvent".to_string())
        .with_organization(&organization)
        .with_venue(&venue)
        .with_ticket_pricing()
        .with_ticket_type_count(2)
        .finish();
    let event_id = event.id;

    let artist1 = database.create_artist().finish();
    let artist2 = database.create_artist().finish();
    let conn = database.connection.get();

    event.add_artist(None, artist1.id, conn).unwrap();
    event.add_artist(None, artist2.id, conn).unwrap();

    let ticket_types = &event.ticket_types(true, None, conn).unwrap();
    let ticket_type = &ticket_types[0];
    let ticket_type2 = &ticket_types[1];
    let _code = database
        .create_code()
        .with_code_type(CodeTypes::Access)
        .with_event(&event)
        .for_ticket_type(&ticket_type2)
        .finish();

    let _event_interest = EventInterest::create(event.id, user.id).commit(conn);
    let test_request = TestRequest::create_with_uri(&format!("/events/{}", event.id));
    let mut path = Path::<StringPathParameters>::extract(&test_request.request).unwrap();
    let event_expected_json = base::events::expected_show_json(
        Roles::User,
        event,
        organization,
        venue,
        false,
        None,
        Some(vec![ticket_type.id]),
        conn,
        1,
        None,
    );
    path.id = event_id.to_string();
    let query_parameters = Query::<EventParameters>::extract(&test_request.request).unwrap();
    assert_eq!(query_parameters.redemption_code, None);

    let response: HttpResponse = events::show((
        test_request.extract_state(),
        database.connection.clone().into(),
        path,
        query_parameters,
        OptionalUser(Some(auth_user)),
        RequestInfo {
            user_agent: Some("test".to_string()),
        },
    ))
    .into();
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body, event_expected_json);
}

#[test]
fn show_with_access_restricted_ticket_type_and_access_code() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);

    let organization = database.create_organization().finish();
    let venue = database.create_venue().finish();
    let event = database
        .create_event()
        .with_name("NewEvent".to_string())
        .with_organization(&organization)
        .with_venue(&venue)
        .with_ticket_pricing()
        .with_ticket_type_count(2)
        .finish();
    let event_id = event.id;

    let artist1 = database.create_artist().finish();
    let artist2 = database.create_artist().finish();
    let conn = database.connection.get();

    event.add_artist(None, artist1.id, conn).unwrap();
    event.add_artist(None, artist2.id, conn).unwrap();

    let ticket_types = &event.ticket_types(true, None, conn).unwrap();
    let ticket_type = &ticket_types[0];
    let ticket_type2 = &ticket_types[1];
    let code = database
        .create_code()
        .with_code_type(CodeTypes::Access)
        .with_event(&event)
        .for_ticket_type(&ticket_type2)
        .finish();

    let _event_interest = EventInterest::create(event.id, user.id).commit(conn);
    let test_request = TestRequest::create_with_uri(&format!(
        "/events/{}?redemption_code={}",
        event.id, code.redemption_code
    ));
    let mut path = Path::<StringPathParameters>::extract(&test_request.request).unwrap();
    let event_expected_json = base::events::expected_show_json(
        Roles::User,
        event,
        organization,
        venue,
        false,
        Some(code.redemption_code.clone()),
        Some(vec![ticket_type.id, ticket_type2.id]),
        conn,
        1,
        None,
    );
    path.id = event_id.to_string();
    let query_parameters = Query::<EventParameters>::extract(&test_request.request).unwrap();
    assert_eq!(query_parameters.redemption_code, Some(code.redemption_code));

    let response: HttpResponse = events::show((
        test_request.extract_state(),
        database.connection.clone().into(),
        path,
        query_parameters,
        OptionalUser(Some(auth_user)),
        RequestInfo {
            user_agent: Some("test".to_string()),
        },
    ))
    .into();
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body, event_expected_json);
}

#[test]
fn show_with_visibility_always_before_sale() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);

    let event = database
        .create_event()
        .with_name("NewEvent".to_string())
        .with_ticket_type()
        .visibility(TicketTypeVisibility::Always)
        .starting(dates::now().add_hours(10).finish())
        .finish();
    let conn = database.connection;
    let _time = dates::now().add_days(-10).finish();

    let request = RequestBuilder::new(&format!("/events/{}", event.id));

    let mut path: Path<StringPathParameters> = request.path();
    path.id = event.id.to_string();

    let response: HttpResponse = events::show((
        request.state(),
        conn.clone().into(),
        path,
        request.query(),
        auth_user.into_optional(),
        RequestInfo {
            user_agent: Some("test".to_string()),
        },
    ))
    .into();

    assert_eq!(response.status(), StatusCode::OK);
    let result: EventShowResult = support::unwrap_body_to_object(&response).unwrap();
    println!("{:?}", result);
    assert_eq!(result.ticket_types[0].status, TicketTypeStatus::OnSaleSoon);
}

#[test]
fn show_with_visibility_always_before_sale_pricing() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);

    let event = database
        .create_event()
        .with_name("NewEvent".to_string())
        .with_ticket_type()
        .visibility(TicketTypeVisibility::Always)
        .starting(dates::now().add_hours(-1).finish())
        .with_pricing()
        .starting(dates::now().add_hours(1).finish())
        .finish();
    let conn = database.connection;
    let _time = dates::now().add_days(-10).finish();

    let request = RequestBuilder::new(&format!("/events/{}", event.id));

    let mut path: Path<StringPathParameters> = request.path();
    path.id = event.id.to_string();

    let response: HttpResponse = events::show((
        request.state(),
        conn.clone().into(),
        path,
        request.query(),
        auth_user.into_optional(),
        RequestInfo {
            user_agent: Some("test".to_string()),
        },
    ))
    .into();

    assert_eq!(response.status(), StatusCode::OK);
    let result: EventShowResult = support::unwrap_body_to_object(&response).unwrap();
    println!("{:?}", result);
    assert_eq!(result.ticket_types[0].status, TicketTypeStatus::Published);
}

#[test]
fn show_with_visibility_always_after_sale() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);

    let _start = dates::now().add_hours(-1).finish();
    let event = database
        .create_event()
        .with_name("NewEvent".to_string())
        .with_ticket_type()
        .visibility(TicketTypeVisibility::Always)
        .ending(dates::now().add_hours(-1).finish())
        .finish();
    let conn = database.connection;
    let _time = dates::now().add_days(-10).finish();

    println!("{:?}", event.id);

    let request = RequestBuilder::new(&format!("/events/{}", event.id));

    let mut path: Path<StringPathParameters> = request.path();
    path.id = event.id.to_string();

    let response: HttpResponse = events::show((
        request.state(),
        conn.clone().into(),
        path,
        request.query(),
        auth_user.into_optional(),
        RequestInfo {
            user_agent: Some("test".to_string()),
        },
    ))
    .into();

    assert_eq!(response.status(), StatusCode::OK);
    let result: EventShowResult = support::unwrap_body_to_object(&response).unwrap();
    assert_eq!(result.ticket_types[0].status, TicketTypeStatus::SaleEnded);
}

#[test]
fn show_with_hidden_ticket_type() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);

    let event = database
        .create_event()
        .with_name("NewEvent".to_string())
        .with_ticket_type()
        .visibility(TicketTypeVisibility::Hidden)
        .finish();
    let conn = database.connection;
    let _time = dates::now().add_days(-10).finish();

    println!("{:?}", event.id);

    let request = RequestBuilder::new(&format!("/events/{}", event.id));

    let mut path: Path<StringPathParameters> = request.path();
    path.id = event.id.to_string();

    let response: HttpResponse = events::show((
        request.state(),
        conn.clone().into(),
        path,
        request.query(),
        auth_user.into_optional(),
        RequestInfo {
            user_agent: Some("test".to_string()),
        },
    ))
    .into();

    assert_eq!(response.status(), StatusCode::OK);
    let result: EventShowResult = support::unwrap_body_to_object(&response).unwrap();
    assert_eq!(result.ticket_types.len(), 0);
}

#[cfg(test)]
mod clone_tests {
    use super::*;

    #[test]
    fn clone_org_member() {
        base::events::clone(Roles::OrgMember, true);
    }

    #[test]
    fn clone_admin() {
        base::events::clone(Roles::Admin, true);
    }

    #[test]
    fn clone_super() {
        base::events::clone(Roles::Super, true);
    }

    #[test]
    fn clone_user() {
        base::events::clone(Roles::User, false);
    }

    #[test]
    fn clone_org_owner() {
        base::events::clone(Roles::OrgOwner, true);
    }

    #[test]
    fn clone_door_person() {
        base::events::clone(Roles::DoorPerson, false);
    }

    #[test]
    fn clone_promoter() {
        base::events::clone(Roles::Promoter, false);
    }

    #[test]
    fn clone_promoter_read_only() {
        base::events::clone(Roles::PromoterReadOnly, false);
    }

    #[test]
    fn clone_org_admin() {
        base::events::clone(Roles::OrgAdmin, true);
    }

    #[test]
    fn clone_box_office() {
        base::events::clone(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod show_box_office_pricing_tests {
    use super::*;

    #[test]
    fn show_box_office_pricing_org_member() {
        base::events::show_box_office_pricing(Roles::OrgMember, true);
    }

    #[test]
    fn show_box_office_pricing_admin() {
        base::events::show_box_office_pricing(Roles::Admin, true);
    }

    #[test]
    fn show_box_office_pricing_user() {
        base::events::show_box_office_pricing(Roles::User, false);
    }

    #[test]
    fn show_box_office_pricing_org_owner() {
        base::events::show_box_office_pricing(Roles::OrgOwner, true);
    }

    #[test]
    fn show_box_office_pricing_door_person() {
        base::events::show_box_office_pricing(Roles::DoorPerson, false);
    }

    #[test]
    fn show_box_office_pricing_promoter() {
        base::events::show_box_office_pricing(Roles::Promoter, false);
    }

    #[test]
    fn show_box_office_pricing_promoter_read_only() {
        base::events::show_box_office_pricing(Roles::PromoterReadOnly, false);
    }

    #[test]
    fn show_box_office_pricing_org_admin() {
        base::events::show_box_office_pricing(Roles::OrgAdmin, true);
    }

    #[test]
    fn show_box_office_pricing_box_office() {
        base::events::show_box_office_pricing(Roles::OrgBoxOffice, true);
    }
}

#[cfg(test)]
mod dashboard_tests {
    use super::*;

    #[test]
    fn dashboard_org_member() {
        base::events::dashboard(Roles::OrgMember, true);
    }

    #[test]
    fn dashboard_admin() {
        base::events::dashboard(Roles::Admin, true);
    }

    #[test]
    fn dashboard_user() {
        base::events::dashboard(Roles::User, false);
    }

    #[test]
    fn dashboard_org_owner() {
        base::events::dashboard(Roles::OrgOwner, true);
    }

    #[test]
    fn dashboard_door_person() {
        base::events::dashboard(Roles::DoorPerson, true);
    }

    #[test]
    fn dashboard_promoter() {
        base::events::dashboard(Roles::Promoter, true);
    }

    #[test]
    fn dashboard_promoter_read_only() {
        base::events::dashboard(Roles::PromoterReadOnly, true);
    }

    #[test]
    fn dashboard_org_admin() {
        base::events::dashboard(Roles::OrgAdmin, true);
    }

    #[test]
    fn dashboard_box_office() {
        base::events::dashboard(Roles::OrgBoxOffice, true);
    }
}

#[cfg(test)]
mod create_tests {
    use super::*;

    #[test]
    fn create_org_member() {
        base::events::create(Roles::OrgMember, true);
    }

    #[test]
    fn create_admin() {
        base::events::create(Roles::Admin, true);
    }

    #[test]
    fn create_user() {
        base::events::create(Roles::User, false);
    }

    #[test]
    fn create_org_owner() {
        base::events::create(Roles::OrgOwner, true);
    }

    #[test]
    fn create_door_person() {
        base::events::create(Roles::DoorPerson, false);
    }

    #[test]
    fn create_promoter() {
        base::events::create(Roles::Promoter, true);
    }

    #[test]
    fn create_promoter_read_only() {
        base::events::create(Roles::PromoterReadOnly, false);
    }

    #[test]
    fn create_org_admin() {
        base::events::create(Roles::OrgAdmin, true);
    }

    #[test]
    fn create_box_office() {
        base::events::create(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod update_tests {
    use super::*;

    #[test]
    fn update_org_member() {
        base::events::update(Roles::OrgMember, true);
    }

    #[test]
    fn update_admin() {
        base::events::update(Roles::Admin, true);
    }

    #[test]
    fn update_user() {
        base::events::update(Roles::User, false);
    }

    #[test]
    fn update_org_owner() {
        base::events::update(Roles::OrgOwner, true);
    }

    #[test]
    fn update_door_person() {
        base::events::update(Roles::DoorPerson, false);
    }

    #[test]
    fn update_promoter() {
        base::events::update(Roles::Promoter, true);
    }

    #[test]
    fn update_promoter_read_only() {
        base::events::update(Roles::PromoterReadOnly, false);
    }

    #[test]
    fn update_org_admin() {
        base::events::update(Roles::OrgAdmin, true);
    }

    #[test]
    fn update_box_office() {
        base::events::update(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod cancel_tests {
    use super::*;

    #[test]
    fn cancel_org_member() {
        base::events::cancel(Roles::OrgMember, true);
    }

    #[test]
    fn cancel_admin() {
        base::events::cancel(Roles::Admin, true);
    }

    #[test]
    fn cancel_user() {
        base::events::cancel(Roles::User, false);
    }

    #[test]
    fn cancel_org_owner() {
        base::events::cancel(Roles::OrgOwner, true);
    }

    #[test]
    fn cancel_door_person() {
        base::events::cancel(Roles::DoorPerson, false);
    }

    #[test]
    fn cancel_promoter() {
        base::events::cancel(Roles::Promoter, false);
    }

    #[test]
    fn cancel_promoter_read_only() {
        base::events::cancel(Roles::PromoterReadOnly, false);
    }

    #[test]
    fn cancel_org_admin() {
        base::events::cancel(Roles::OrgAdmin, true);
    }

    #[test]
    fn cancel_box_office() {
        base::events::cancel(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod delete_tests {
    use super::*;

    #[test]
    fn delete_org_member() {
        base::events::delete(Roles::OrgMember, true);
    }

    #[test]
    fn delete_admin() {
        base::events::delete(Roles::Admin, true);
    }

    #[test]
    fn delete_user() {
        base::events::delete(Roles::User, false);
    }

    #[test]
    fn delete_org_owner() {
        base::events::delete(Roles::OrgOwner, true);
    }

    #[test]
    fn delete_door_person() {
        base::events::delete(Roles::DoorPerson, false);
    }

    #[test]
    fn delete_promoter() {
        base::events::delete(Roles::Promoter, true);
    }

    #[test]
    fn delete_promoter_read_only() {
        base::events::delete(Roles::PromoterReadOnly, false);
    }

    #[test]
    fn delete_org_admin() {
        base::events::delete(Roles::OrgAdmin, true);
    }

    #[test]
    fn delete_box_office() {
        base::events::delete(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod add_artist_tests {
    use super::*;

    #[test]
    fn add_artist_org_member() {
        base::events::add_artist(Roles::OrgMember, true);
    }

    #[test]
    fn add_artist_admin() {
        base::events::add_artist(Roles::Admin, true);
    }

    #[test]
    fn add_artist_user() {
        base::events::add_artist(Roles::User, false);
    }

    #[test]
    fn add_artist_org_owner() {
        base::events::add_artist(Roles::OrgOwner, true);
    }

    #[test]
    fn add_artist_door_person() {
        base::events::add_artist(Roles::DoorPerson, false);
    }

    #[test]
    fn add_artist_promoter() {
        base::events::add_artist(Roles::Promoter, true)
    }

    #[test]
    fn add_artist_promoter_read_only() {
        base::events::add_artist(Roles::PromoterReadOnly, false);
    }

    #[test]
    fn add_artist_org_admin() {
        base::events::add_artist(Roles::OrgAdmin, true);
    }

    #[test]
    fn add_artist_box_office() {
        base::events::add_artist(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod list_interested_users_tests {
    use super::*;

    #[test]
    fn list_interested_users_org_member() {
        base::events::list_interested_users(Roles::OrgMember, true);
    }

    #[test]
    fn list_interested_users_admin() {
        base::events::list_interested_users(Roles::Admin, true);
    }

    #[test]
    fn list_interested_users_user() {
        base::events::list_interested_users(Roles::User, true);
    }

    #[test]
    fn list_interested_users_org_owner() {
        base::events::list_interested_users(Roles::OrgOwner, true);
    }

    #[test]
    fn list_interested_users_door_person() {
        base::events::list_interested_users(Roles::DoorPerson, true);
    }

    #[test]
    fn list_interested_users_promoter() {
        base::events::list_interested_users(Roles::Promoter, true);
    }

    #[test]
    fn list_interested_users_promoter_read_only() {
        base::events::list_interested_users(Roles::PromoterReadOnly, true);
    }

    #[test]
    fn list_interested_users_org_admin() {
        base::events::list_interested_users(Roles::OrgAdmin, true);
    }

    #[test]
    fn list_interested_users_box_office() {
        base::events::list_interested_users(Roles::OrgBoxOffice, true);
    }
}

#[cfg(test)]
mod add_interest_tests {
    use super::*;

    #[test]
    fn add_interest_org_member() {
        base::events::add_interest(Roles::OrgMember, true);
    }

    #[test]
    fn add_interest_admin() {
        base::events::add_interest(Roles::Admin, true);
    }

    #[test]
    fn add_interest_user() {
        base::events::add_interest(Roles::User, true);
    }

    #[test]
    fn add_interest_org_owner() {
        base::events::add_interest(Roles::OrgOwner, true);
    }

    #[test]
    fn add_interest_door_person() {
        base::events::add_interest(Roles::DoorPerson, true);
    }

    #[test]
    fn add_interest_promoter() {
        base::events::add_interest(Roles::Promoter, true);
    }

    #[test]
    fn add_interest_promoter_read_only() {
        base::events::add_interest(Roles::PromoterReadOnly, true);
    }

    #[test]
    fn add_interest_org_admin() {
        base::events::add_interest(Roles::OrgAdmin, true);
    }

    #[test]
    fn add_interest_box_office() {
        base::events::add_interest(Roles::OrgBoxOffice, true);
    }
}

#[cfg(test)]
mod remove_interest_tests {
    use super::*;

    #[test]
    fn remove_interest_org_member() {
        base::events::remove_interest(Roles::OrgMember, true);
    }

    #[test]
    fn remove_interest_admin() {
        base::events::remove_interest(Roles::Admin, true);
    }

    #[test]
    fn remove_interest_user() {
        base::events::remove_interest(Roles::User, true);
    }

    #[test]
    fn remove_interest_org_owner() {
        base::events::remove_interest(Roles::OrgOwner, true);
    }

    #[test]
    fn remove_interest_door_person() {
        base::events::remove_interest(Roles::DoorPerson, true);
    }

    #[test]
    fn remove_interest_promoter() {
        base::events::remove_interest(Roles::Promoter, true);
    }

    #[test]
    fn remove_interest_promoter_read_only() {
        base::events::remove_interest(Roles::PromoterReadOnly, true);
    }

    #[test]
    fn remove_interest_org_admin() {
        base::events::remove_interest(Roles::OrgAdmin, true);
    }

    #[test]
    fn remove_interest_box_office() {
        base::events::remove_interest(Roles::OrgBoxOffice, true);
    }
}

#[cfg(test)]
mod update_artists_tests {
    use super::*;

    #[test]
    fn update_artists_org_member() {
        base::events::update_artists(Roles::OrgMember, true);
    }

    #[test]
    fn update_artists_admin() {
        base::events::update_artists(Roles::Admin, true);
    }

    #[test]
    fn update_artists_user() {
        base::events::update_artists(Roles::User, false);
    }

    #[test]
    fn update_artists_org_owner() {
        base::events::update_artists(Roles::OrgOwner, true);
    }

    #[test]
    fn update_artists_door_person() {
        base::events::update_artists(Roles::DoorPerson, false);
    }

    #[test]
    fn update_artists_promoter() {
        base::events::update_artists(Roles::Promoter, true);
    }

    #[test]
    fn update_artists_promoter_read_only() {
        base::events::update_artists(Roles::PromoterReadOnly, false);
    }

    #[test]
    fn update_artists_org_admin() {
        base::events::update_artists(Roles::OrgAdmin, true);
    }

    #[test]
    fn update_artists_box_office() {
        base::events::update_artists(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod guest_list_tests {
    use super::*;

    #[test]
    fn guest_list_org_member() {
        base::events::guest_list(Roles::OrgMember, true);
    }

    #[test]
    fn guest_list_admin() {
        base::events::guest_list(Roles::Admin, true);
    }

    #[test]
    fn guest_list_user() {
        base::events::guest_list(Roles::User, false);
    }

    #[test]
    fn guest_list_org_owner() {
        base::events::guest_list(Roles::OrgOwner, true);
    }

    #[test]
    fn guest_list_door_person() {
        base::events::guest_list(Roles::DoorPerson, true);
    }

    #[test]
    fn guest_list_promoter() {
        base::events::guest_list(Roles::Promoter, true);
    }

    #[test]
    fn guest_list_promoter_read_only() {
        base::events::guest_list(Roles::PromoterReadOnly, true);
    }

    #[test]
    fn guest_list_org_admin() {
        base::events::guest_list(Roles::OrgAdmin, true);
    }

    #[test]
    fn guest_list_box_office() {
        base::events::guest_list(Roles::OrgBoxOffice, true);
    }
}

#[cfg(test)]
mod codes_tests {
    use super::*;

    #[test]
    fn codes_org_member() {
        base::events::codes(Roles::OrgMember, true);
    }

    #[test]
    fn codes_admin() {
        base::events::codes(Roles::Admin, true);
    }

    #[test]
    fn codes_user() {
        base::events::codes(Roles::User, false);
    }

    #[test]
    fn codes_org_owner() {
        base::events::codes(Roles::OrgOwner, true);
    }

    #[test]
    fn codes_door_person() {
        base::events::codes(Roles::DoorPerson, true);
    }

    #[test]
    fn codes_promoter() {
        base::events::codes(Roles::Promoter, true);
    }

    #[test]
    fn codes_promoter_read_only() {
        base::events::codes(Roles::PromoterReadOnly, true);
    }

    #[test]
    fn codes_org_admin() {
        base::events::codes(Roles::OrgAdmin, true);
    }

    #[test]
    fn codes_box_office() {
        base::events::codes(Roles::OrgBoxOffice, true);
    }
}

#[cfg(test)]
mod holds_tests {
    use super::*;

    #[test]
    fn holds_org_member() {
        base::events::holds(Roles::OrgMember, true);
    }

    #[test]
    fn holds_admin() {
        base::events::holds(Roles::Admin, true);
    }

    #[test]
    fn holds_user() {
        base::events::holds(Roles::User, false);
    }

    #[test]
    fn holds_org_owner() {
        base::events::holds(Roles::OrgOwner, true);
    }

    #[test]
    fn holds_door_person() {
        base::events::holds(Roles::DoorPerson, true);
    }

    #[test]
    fn holds_promoter() {
        base::events::holds(Roles::Promoter, true);
    }

    #[test]
    fn holds_promoter_read_only() {
        base::events::holds(Roles::PromoterReadOnly, true);
    }

    #[test]
    fn holds_org_admin() {
        base::events::holds(Roles::OrgAdmin, true);
    }

    #[test]
    fn holds_box_office() {
        base::events::holds(Roles::OrgBoxOffice, true);
    }
}

#[cfg(test)]
mod export_event_data_tests {
    use super::*;

    #[test]
    fn export_event_data_org_member() {
        base::events::export_event_data(Roles::OrgMember, false, None);
    }

    #[test]
    fn export_event_data_admin() {
        base::events::export_event_data(Roles::Admin, true, None);
    }

    #[test]
    fn export_event_data_user() {
        base::events::export_event_data(Roles::User, false, None);
    }

    #[test]
    fn export_event_data_org_owner() {
        base::events::export_event_data(Roles::OrgOwner, true, None);
    }

    #[test]
    fn export_event_data_door_person() {
        base::events::export_event_data(Roles::DoorPerson, false, None);
    }

    #[test]
    fn export_event_data_promoter() {
        base::events::export_event_data(Roles::Promoter, false, None);
    }

    #[test]
    fn export_event_data_promoter_read_only() {
        base::events::export_event_data(Roles::PromoterReadOnly, false, None);
    }

    #[test]
    fn export_event_data_org_admin() {
        base::events::export_event_data(Roles::OrgAdmin, true, None);
    }

    #[test]
    fn export_event_data_box_office() {
        base::events::export_event_data(Roles::OrgBoxOffice, false, None);
    }

    #[test]
    fn export_event_data_event_data_exporter() {
        base::events::export_event_data(Roles::PrismIntegration, true, None);
    }

    #[test]
    fn export_event_data_event_data_exporter_past() {
        base::events::export_event_data(Roles::PrismIntegration, true, Some(PastOrUpcoming::Past));
    }

    #[test]
    fn export_event_data_event_data_exporter_upcoming() {
        base::events::export_event_data(Roles::PrismIntegration, true, Some(PastOrUpcoming::Upcoming));
    }
}

#[cfg(test)]
mod redeem_ticket {
    use super::*;

    #[test]
    fn redeem_ticket_org_member() {
        base::events::redeem_ticket(Roles::OrgMember, true);
    }
    #[test]
    fn redeem_ticket_admin() {
        base::events::redeem_ticket(Roles::Admin, true);
    }
    #[test]
    fn redeem_ticket_user() {
        base::events::redeem_ticket(Roles::User, false);
    }
    #[test]
    fn redeem_ticket_org_owner() {
        base::events::redeem_ticket(Roles::OrgOwner, true);
    }
    #[test]
    fn redeem_ticket_door_person() {
        base::events::redeem_ticket(Roles::DoorPerson, true);
    }
    #[test]
    fn redeem_ticket_promoter() {
        base::events::redeem_ticket(Roles::Promoter, false);
    }
    #[test]
    fn redeem_ticket_promoter_read_only() {
        base::events::redeem_ticket(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn redeem_ticket_org_admin() {
        base::events::redeem_ticket(Roles::OrgAdmin, true);
    }
    #[test]
    fn redeem_ticket_box_office() {
        base::events::redeem_ticket(Roles::OrgBoxOffice, true);
    }
}

#[test]
pub fn delete_fails_has_ticket_in_cart() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let user2 = database.create_user().finish();
    let organization = database.create_organization().finish();
    let event = database
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::Admin, Some(&organization), &database);

    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    let mut cart = Order::find_or_create_cart(&user2, connection).unwrap();
    cart.update_quantities(
        user2.id,
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

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = event.id;

    let response: HttpResponse = events::delete((database.connection.clone().into(), path, auth_user)).into();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(response.error().is_some());
}

#[test]
fn update_promoter_fails_lacks_event_id() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let event = database.create_event().with_organization(&organization).finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::Promoter, Some(&organization), &database);

    // Remove event access
    EventUser::find_by_event_id_user_id(event.id, user.id, connection)
        .unwrap()
        .destroy(connection)
        .unwrap();

    let new_name = "New Event Name";
    let test_request = TestRequest::create();

    let json = Json(EventEditableAttributes {
        name: Some(new_name.to_string()),
        ..Default::default()
    });
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = event.id;

    let response: HttpResponse =
        events::update((database.connection.clone().into(), path, json, auth_user.clone())).into();
    support::expects_unauthorized(&response);
}

#[test]
fn dashboard_with_default_range() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let org_admin = database.create_user().finish();
    let organization = database.create_organization().with_fees().finish();
    let auth_user = support::create_auth_user_from_user(&org_admin, Roles::OrgOwner, Some(&organization), &database);
    let event_start = Utc::now().naive_utc() - Duration::days(1);
    let event_end = Utc::now().naive_utc() + Duration::hours(1);
    let event = database
        .create_event()
        .with_organization(&organization)
        .with_event_start(event_start)
        .with_event_end(event_end)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];

    // org_admin purchases 10 tickets
    let mut cart = Order::find_or_create_cart(&org_admin, connection).unwrap();
    cart.update_quantities(
        org_admin.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 10,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    assert_eq!(cart.calculate_total(connection).unwrap(), 1700);
    cart.add_external_payment(
        Some("test".to_string()),
        ExternalPaymentType::CreditCard,
        org_admin.id,
        1700,
        connection,
    )
    .unwrap();
    assert_eq!(cart.status, OrderStatus::Paid);

    let test_request = TestRequest::create_with_uri(&format!("/events/{}/dashboard?", event.id));
    let query_parameters = Query::<DashboardParameters>::extract(&test_request.request).unwrap();
    let mut path_parameters = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path_parameters.id = event.id;

    let response: HttpResponse = events::dashboard((
        test_request.extract_state(),
        database.connection.clone().into(),
        path_parameters,
        query_parameters,
        auth_user.clone(),
    ))
    .into();

    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    let dashboard_result: DashboardResult = serde_json::from_str(&body).unwrap();
    assert_eq!(dashboard_result.day_stats.len(), 30);
    assert_eq!(
        dashboard_result.day_stats[29],
        DayStats {
            date: event.event_end.unwrap().date(),
            revenue_in_cents: 1500,
            ticket_sales: 10,
        }
    );
}

#[test]
pub fn show_from_organizations_past() {
    let database = TestDatabase::new();

    let organization = database.create_organization().finish();
    let event = database
        .create_event()
        .with_name("NewEvent".to_string())
        .with_event_start(NaiveDateTime::parse_from_str("2014-03-04 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f").unwrap())
        .with_event_end(NaiveDateTime::parse_from_str("2014-03-05 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f").unwrap())
        .with_organization(&organization)
        .finish();
    let _event2 = database
        .create_event()
        .with_name("NewEvent2".to_string())
        .with_event_start(NaiveDateTime::parse_from_str("2059-03-02 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f").unwrap())
        .with_event_end(NaiveDateTime::parse_from_str("2059-03-03 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f").unwrap())
        .with_organization(&organization)
        .finish();

    let user = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::OrgMember, Some(&organization), &database);

    let expected_events = vec![event.id];

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = organization.id;
    let test_request = TestRequest::create_with_uri(&format!("/events?past_or_upcoming=Past"));
    let query_parameters = Query::<PagingParameters>::extract(&test_request.request).unwrap();
    let response =
        events::show_from_organizations((database.connection.into(), path, query_parameters, auth_user)).unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        expected_events,
        response.payload().data.iter().map(|i| i.id).collect::<Vec<Uuid>>()
    );
}

#[test]
pub fn show_from_organizations_upcoming() {
    let database = TestDatabase::new();

    let organization = database.create_organization().finish();
    let _event = database
        .create_event()
        .with_name("NewEvent".to_string())
        .with_event_start(NaiveDateTime::parse_from_str("2014-03-04 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f").unwrap())
        .with_event_end(NaiveDateTime::parse_from_str("2014-03-05 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f").unwrap())
        .with_organization(&organization)
        .finish();
    let event2 = database
        .create_event()
        .with_event_start(NaiveDateTime::parse_from_str("2059-03-02 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f").unwrap())
        .with_event_end(NaiveDateTime::parse_from_str("2059-03-03 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f").unwrap())
        .with_name("NewEvent2".to_string())
        .with_organization(&organization)
        .finish();

    let user = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::OrgMember, Some(&organization), &database);

    let expected_events = vec![event2.id];
    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = organization.id;
    let test_request = TestRequest::create_with_uri(&format!("/events?past_or_upcoming=Upcoming"));
    let query_parameters = Query::<PagingParameters>::extract(&test_request.request).unwrap();
    let response =
        events::show_from_organizations((database.connection.into(), path, query_parameters, auth_user)).unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        expected_events,
        response.payload().data.iter().map(|i| i.id).collect::<Vec<Uuid>>()
    );
}

pub fn event_venue_entry(
    event: &Event,
    venue: &Venue,
    artists: &Vec<DisplayEventArtist>,
    user: Option<User>,
    connection: &PgConnection,
) -> EventVenueEntry {
    let localized_times = event.get_all_localized_time_strings(Some(venue));
    let (min_ticket_price, max_ticket_price) = event.current_ticket_pricing_range(false, connection).unwrap();
    let slug = event.slug(connection).unwrap();
    EventVenueEntry {
        id: event.id,
        name: event.name.clone(),
        organization_id: event.organization_id,
        venue_id: event.venue_id,
        created_at: event.created_at,
        updated_at: event.updated_at,
        event_start: event.event_start,
        door_time: event.door_time,
        status: event.status.clone(),
        publish_date: event.publish_date,
        promo_image_url: event.promo_image_url.clone(),
        original_promo_image_url: event.promo_image_url.clone(),
        additional_info: event.additional_info.clone(),
        top_line_info: event.top_line_info.clone(),
        age_limit: event.age_limit.clone(),
        cancelled_at: event.cancelled_at,
        venue: Some(venue.clone()),
        artists: Some(artists.clone()),
        min_ticket_price,
        max_ticket_price,
        is_external: event.is_external.clone(),
        external_url: event.external_url.clone(),
        user_is_interested: user
            .map(|u| EventInterest::user_interest(event.id, u.id, connection).unwrap())
            .unwrap_or(false),
        localized_times,
        tracking_keys: TrackingKeys { ..Default::default() },
        event_type: event.event_type,
        url: format!("{}/tickets/{}", env::var("FRONT_END_URL").unwrap(), &slug),
        slug,
        event_end: event.event_end,
    }
}
