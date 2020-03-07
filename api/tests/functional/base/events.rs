use actix_web::{http::StatusCode, FromRequest, HttpResponse, Path, Query};
use bigneon_api::controllers::events;
use bigneon_api::controllers::events::*;
use bigneon_api::db::CacheDatabase;
use bigneon_api::extractors::*;
use bigneon_api::models::*;
use bigneon_db::dev::times;
use bigneon_db::models::*;
use bigneon_db::utils::dates;
use chrono::prelude::*;
use chrono::Duration;
use diesel::PgConnection;
use serde_json;
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use crate::support;
use crate::support::database::TestDatabase;
use crate::support::test_request::TestRequest;
use uuid::Uuid;

pub fn clone(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let event = database
        .create_event()
        .with_organization(&organization)
        .with_ticket_pricing()
        .finish();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);

    let test_request = TestRequest::create_with_uri(&format!("/events/{}/clone", event.id,));
    let mut path_parameters = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path_parameters.id = event.id;
    let clone_fields = CloneFields {
        name: "Cloned Event".to_string(),
        event_start: dates::now().add_days(5).finish(),
        event_end: dates::now().add_days(6).finish(),
    };
    let json = Json(clone_fields.clone());
    let response: HttpResponse = events::clone((
        database.connection.into(),
        json,
        path_parameters,
        auth_user,
        test_request.extract_state(),
    ))
    .into();

    if should_test_succeed {
        assert_eq!(response.status(), StatusCode::CREATED);
        let body = support::unwrap_body_to_string(&response).unwrap();
        let cloned_event: Event = serde_json::from_str(&body).unwrap();
        assert_eq!(cloned_event.status, EventStatus::Draft);
        assert_ne!(cloned_event.id, event.id);
        assert_eq!(&cloned_event.name, &clone_fields.name);
        assert_eq!(
            cloned_event.event_start.unwrap().timestamp(),
            clone_fields.event_start.timestamp()
        );
        assert_eq!(
            cloned_event.event_end.unwrap().timestamp(),
            clone_fields.event_end.timestamp()
        );
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

    let mut path = Path::<PathParameters>::extract(&request.request).unwrap();
    path.id = event.id;
    let mut path2 = Path::<PathParameters>::extract(&request.request).unwrap();
    path2.id = event.id;

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
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
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

pub fn export_event_data(role: Roles, should_test_succeed: bool, past_or_upcoming: Option<PastOrUpcoming>) {
    let database = TestDatabase::new();

    let organization = database.create_organization().finish();
    let event = database
        .create_event()
        .with_name("NewEvent".to_string())
        .with_event_start(NaiveDateTime::parse_from_str("2014-03-04 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f").unwrap())
        .with_event_end(NaiveDateTime::parse_from_str("2014-03-05 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f").unwrap())
        .with_organization(&organization)
        .finish();
    let event2 = database
        .create_event()
        .with_name("NewEvent2".to_string())
        .with_event_start(NaiveDateTime::parse_from_str("2059-03-02 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f").unwrap())
        .with_event_end(NaiveDateTime::parse_from_str("2059-03-03 12:00:00.000", "%Y-%m-%d %H:%M:%S%.f").unwrap())
        .with_organization(&organization)
        .finish();

    let user = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);

    let expected_events = if past_or_upcoming.is_none() {
        vec![event.id, event2.id]
    } else if past_or_upcoming == Some(PastOrUpcoming::Upcoming) {
        vec![event2.id]
    } else {
        vec![event.id]
    };

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = organization.id;
    let uri = match past_or_upcoming {
        Some(past_or_upcoming) => format!("/?past_or_upcoming={}", past_or_upcoming),
        None => "/".to_string(),
    };
    let test_request = TestRequest::create_with_uri(&uri);
    let query_parameters = Query::<PagingParameters>::extract(&test_request.request).unwrap();
    let response = events::export_event_data((database.connection.into(), path, query_parameters, auth_user));

    if should_test_succeed {
        let response = response.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_equiv!(
            expected_events,
            response.payload().data.iter().map(|i| i.id).collect::<Vec<Uuid>>()
        );
    } else {
        assert_eq!(
            response.err().unwrap().to_string(),
            "User does not have the required permissions"
        );
    }
}

pub fn create(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let venue = database.create_venue().finish();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);

    let name = "event Example";
    let new_event = NewEvent {
        name: name.to_string(),
        organization_id: organization.id,
        venue_id: Some(venue.id),
        event_start: Some(NaiveDate::from_ymd(2016, 7, 8).and_hms(9, 10, 11)),
        door_time: Some(NaiveDate::from_ymd(2016, 7, 8).and_hms(8, 11, 12)),
        publish_date: Some(NaiveDate::from_ymd(2016, 7, 1).and_hms(9, 10, 11)),
        ..Default::default()
    };
    // Emulate serialization for default serde behavior
    let new_event: NewEvent = serde_json::from_str(&serde_json::to_string(&new_event).unwrap()).unwrap();
    let json = Json(new_event);

    let response: HttpResponse = events::create((database.connection.into(), json, auth_user.clone())).into();
    if should_test_succeed {
        assert_eq!(response.status(), StatusCode::CREATED);
        let body = support::unwrap_body_to_string(&response).unwrap();
        let event: Event = serde_json::from_str(&body).unwrap();
        assert_eq!(event.status, EventStatus::Draft);
    } else {
        support::expects_unauthorized(&response);
    }
}

pub fn show_box_office_pricing(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let conn = database.connection.get();
    let user = database.create_user().finish();
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
    let ticket_type = &event.ticket_types(true, None, conn).unwrap()[0];
    ticket_type
        .add_ticket_pricing(
            "Box office".into(),
            NaiveDate::from_ymd(2016, 7, 8).and_hms(7, 8, 10),
            NaiveDate::from_ymd(9999, 7, 8).and_hms(7, 8, 10),
            5000,
            true,
            None,
            None,
            conn,
        )
        .unwrap();

    let artist1 = database.create_artist().finish();
    let artist2 = database.create_artist().finish();
    event.add_artist(None, artist1.id, conn).unwrap();
    event.add_artist(None, artist2.id, conn).unwrap();
    let _event_interest = EventInterest::create(event.id, user.id).commit(conn);
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);

    let test_request = TestRequest::create_with_uri(&format!("/events/{}?box_office_pricing=true", event.id));
    let mut path = Path::<StringPathParameters>::extract(&test_request.request).unwrap();
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

    if should_test_succeed {
        let event_expected_json = expected_show_json(role, event, organization, venue, true, None, None, conn, 1, None);
        let body = support::unwrap_body_to_string(&response).unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(body, event_expected_json);
    } else {
        support::expects_unauthorized(&response);
    }
}

pub fn update(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let event = database.create_event().with_organization(&organization).finish();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);

    let new_name = "New Event Name";
    let test_request = TestRequest::create();

    let json = Json(EventEditableAttributes {
        name: Some(new_name.to_string()),
        ..Default::default()
    });
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = event.id;

    let response: HttpResponse = events::update((database.connection.into(), path, json, auth_user.clone())).into();
    let body = support::unwrap_body_to_string(&response).unwrap();
    if should_test_succeed {
        assert_eq!(response.status(), StatusCode::OK);
        let updated_event: Event = serde_json::from_str(&body).unwrap();
        assert_eq!(updated_event.name, new_name);
    } else {
        support::expects_unauthorized(&response);
    }
}

pub fn delete(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let event = database
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = event.id;

    let response: HttpResponse = events::delete((database.connection.clone().into(), path, auth_user)).into();
    if should_test_succeed {
        assert_eq!(response.status(), StatusCode::OK);
        assert!(Event::find(event.id, connection).is_err());
    } else {
        support::expects_unauthorized(&response);
    }
}

pub fn cancel(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let event = database.create_event().with_organization(&organization).finish();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = event.id;

    let response: HttpResponse = events::cancel((database.connection.into(), path, auth_user)).into();
    if should_test_succeed {
        let body = support::unwrap_body_to_string(&response).unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let updated_event: Event = serde_json::from_str(&body).unwrap();
        assert!(!updated_event.cancelled_at.is_none());
    } else {
        support::expects_unauthorized(&response);
    }
}

pub fn add_artist(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let event = database.create_event().with_organization(&organization).finish();
    let artist = database.create_artist().with_organization(&organization).finish();

    artist
        .set_genres(&vec!["emo".to_string(), "hard-rock".to_string()], None, connection)
        .unwrap();
    assert!(event.genres(connection).unwrap().is_empty());

    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);

    let test_request = TestRequest::create();
    let new_event_artist = AddArtistRequest {
        artist_id: artist.id,
        rank: 5,
        set_time: Some(NaiveDate::from_ymd(2016, 7, 1).and_hms(9, 10, 11)),
        importance: 0,
        stage_id: None,
    };

    let json = Json(new_event_artist);

    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = event.id;

    let response: HttpResponse =
        events::add_artist((database.connection.clone().into(), path, json, auth_user.clone())).into();
    if should_test_succeed {
        assert_eq!(response.status(), StatusCode::CREATED);
        // Trigger sync right away as normally will happen via background worker
        event.update_genres(None, connection).unwrap();
        assert_eq!(
            event.genres(connection).unwrap(),
            vec!["emo".to_string(), "hard-rock".to_string()]
        );
    } else {
        support::expects_unauthorized(&response);
        assert!(event.genres(connection).unwrap().is_empty());
    }
}

pub fn list_interested_users(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let event = database.create_event().finish();
    let primary_user = support::create_auth_user(role, None, &database);
    let conn = database.connection.get();
    EventInterest::create(event.id, primary_user.id()).commit(conn).unwrap();
    let n_secondary_users = 5;
    let mut secondary_users: Vec<DisplayEventInterestedUser> = Vec::new();
    secondary_users.reserve(n_secondary_users);
    for _u_id in 0..n_secondary_users {
        let current_secondary_user = database.create_user().finish();
        EventInterest::create(event.id, current_secondary_user.id)
            .commit(conn)
            .unwrap();
        let current_user_entry = DisplayEventInterestedUser {
            user_id: current_secondary_user.id,
            first_name: current_secondary_user.first_name.clone().unwrap_or("".to_string()),
            last_name: current_secondary_user.last_name.clone().unwrap_or("".to_string()),
            thumb_profile_pic_url: None,
        };
        secondary_users.push(current_user_entry);
    }
    secondary_users.sort_by_key(|x| x.user_id); //Sort results for testing purposes
                                                //Construct api query
    let page: usize = 0;
    let limit: usize = 100;
    let test_request = TestRequest::create_with_uri(&format!(
        "/interest?page={}&limit={}",
        page.to_string(),
        limit.to_string()
    ));
    let query_parameters = Query::<PagingParameters>::extract(&test_request.request).unwrap();
    let mut path_parameters = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path_parameters.id = event.id;
    let response: HttpResponse = events::list_interested_users((
        database.connection.clone(),
        path_parameters,
        query_parameters,
        primary_user,
    ))
    .into();
    let response_body = support::unwrap_body_to_string(&response).unwrap();
    //Construct expected output
    let len = secondary_users.len() as u64;
    let wrapped_expected_date = Payload {
        data: secondary_users,
        paging: Paging {
            page: 0,
            limit: 100,
            sort: "".to_string(),
            dir: SortingDir::Asc,
            total: len,
            tags: HashMap::new(),
        },
    };
    let expected_json_body = serde_json::to_string(&wrapped_expected_date).unwrap();

    if should_test_succeed {
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response_body, expected_json_body);
    } else {
        support::expects_unauthorized(&response);
    }
}

pub fn add_interest(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let event = database.create_event().finish();

    let user = support::create_auth_user(role, None, &database);
    let test_request = TestRequest::create();

    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = event.id;

    let response: HttpResponse = events::add_interest((database.connection.into(), path, user)).into();

    if should_test_succeed {
        assert_eq!(response.status(), StatusCode::CREATED);
    } else {
        support::expects_unauthorized(&response);
    }
}

pub fn remove_interest(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let event = database.create_event().finish();
    EventInterest::create(event.id, user.id)
        .commit(database.connection.get())
        .unwrap();
    let user = support::create_auth_user_from_user(&user, role, None, &database);

    let test_request = TestRequest::create();

    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = event.id;

    let response: HttpResponse = events::remove_interest((database.connection.into(), path, user)).into();
    let body = support::unwrap_body_to_string(&response).unwrap();

    if should_test_succeed {
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(body, "1");
    } else {
        support::expects_unauthorized(&response);
    }
}

pub fn update_artists(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let event = database.create_event().with_organization(&organization).finish();
    let artist1 = database.create_artist().finish();
    let artist2 = database.create_artist().finish();
    artist1
        .set_genres(&vec!["emo".to_string(), "hard-rock".to_string()], None, connection)
        .unwrap();
    artist2
        .set_genres(&vec!["happy".to_string()], None, connection)
        .unwrap();
    assert!(event.genres(connection).unwrap().is_empty());

    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = event.id;

    let mut payload: UpdateArtistsRequestList = Default::default();
    payload.artists.push(UpdateArtistsRequest {
        artist_id: artist1.id,
        set_time: Some(NaiveDate::from_ymd(2016, 7, 8).and_hms(9, 10, 11)),
        importance: 0,
        stage_id: None,
    });
    payload.artists.push(UpdateArtistsRequest {
        artist_id: artist2.id,
        set_time: None,
        importance: 1,
        stage_id: None,
    });
    let response: HttpResponse = events::update_artists((
        database.connection.clone().into(),
        path,
        Json(payload),
        auth_user.clone(),
    ))
    .into();
    let body = support::unwrap_body_to_string(&response).unwrap();

    if should_test_succeed {
        assert_eq!(response.status(), StatusCode::OK);
        // Trigger sync right away as normally will happen via background worker
        event.update_genres(None, connection).unwrap();
        assert_eq!(
            event.genres(connection).unwrap(),
            vec!["emo".to_string(), "happy".to_string(), "hard-rock".to_string()]
        );
        let returned_event_artists: Vec<EventArtist> = serde_json::from_str(&body).unwrap();
        assert_eq!(returned_event_artists[0].artist_id, artist1.id);
        assert_eq!(returned_event_artists[1].set_time, None);
        assert_eq!(returned_event_artists[1].importance, 1);
    } else {
        assert!(event.genres(connection).unwrap().is_empty());
        support::expects_unauthorized(&response);
    }
}

pub fn dashboard(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let organization = database.create_organization().with_fees().finish();
    let event = database
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);

    // user purchases 10 tickets
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    cart.update_quantities(
        user.id,
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
        user.id,
        1700,
        connection,
    )
    .unwrap();
    assert_eq!(cart.status, OrderStatus::Paid);

    let start_utc = Utc::now().naive_utc().date() - Duration::days(1);
    let end_utc = Utc::now().naive_utc().date();

    let test_request = TestRequest::create_with_uri(&format!(
        "/events/{}/dashboard?start_utc={:?}&end_utc={:?}",
        event.id, start_utc, end_utc
    ));
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
    if should_test_succeed {
        assert_eq!(response.status(), StatusCode::OK);
        let body = support::unwrap_body_to_string(&response).unwrap();
        let dashboard_result: DashboardResult = serde_json::from_str(&body).unwrap();
        assert_eq!(dashboard_result.day_stats.len(), 2);
        assert_eq!(
            dashboard_result.day_stats,
            vec![
                DayStats {
                    date: start_utc,
                    revenue_in_cents: 0,
                    ticket_sales: 0,
                },
                DayStats {
                    date: end_utc,
                    revenue_in_cents: 1500,
                    ticket_sales: 10,
                }
            ]
        );
    } else {
        support::expects_unauthorized(&response);
    }
}

pub fn guest_list(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let event = database
        .create_event()
        .with_organization(&organization)
        .with_ticket_pricing()
        .finish();
    database.create_order().for_event(&event).is_paid().finish();
    database.create_order().for_event(&event).is_paid().finish();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);

    let test_request = TestRequest::create_with_uri(&format!("/events/{}/guest?query=", event.id,));
    let query_parameters = Query::<GuestListQueryParameters>::extract(&test_request.request).unwrap();
    let mut path_parameters = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path_parameters.id = event.id;
    let response: HttpResponse =
        events::guest_list((database.connection.into(), query_parameters, path_parameters, auth_user)).into();

    if should_test_succeed {
        assert_eq!(response.status(), StatusCode::OK);
    } else {
        support::expects_unauthorized(&response);
    }
}

pub fn codes(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let event = database
        .create_event()
        .with_organization(&organization)
        .with_ticket_pricing()
        .finish();
    let code = database
        .create_code()
        .with_name("Discount 1".into())
        .with_event(&event)
        .with_code_type(CodeTypes::Discount)
        .finish()
        .for_display(connection)
        .unwrap();
    let code2 = database
        .create_code()
        .with_name("Discount 2".into())
        .with_event(&event)
        .with_code_type(CodeTypes::Discount)
        .finish()
        .for_display(connection)
        .unwrap();
    let _code3 = database
        .create_code()
        .with_name("Access".into())
        .with_event(&event)
        .with_code_type(CodeTypes::Access)
        .finish()
        .for_display(connection)
        .unwrap();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);
    let all_discounts = vec![code, code2];

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = event.id;
    let test_request = TestRequest::create_with_uri(&format!("/codes?type=Discount"));
    let query_parameters = Query::<PagingParameters>::extract(&test_request.request).unwrap();

    let response: HttpResponse =
        events::codes((database.connection.clone().into(), query_parameters, path, auth_user)).into();

    let mut expected_tags: HashMap<String, Value> = HashMap::new();
    expected_tags.insert("type".to_string(), json!("Discount"));

    let expected_discounts = Payload {
        data: all_discounts,
        paging: Paging {
            page: 0,
            limit: 100,
            sort: "".to_string(),
            dir: SortingDir::Asc,
            total: 2,
            tags: expected_tags,
        },
    };
    let expected_json = serde_json::to_string(&expected_discounts).unwrap();
    let body = support::unwrap_body_to_string(&response).unwrap();
    if should_test_succeed {
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(body, expected_json);
    } else {
        support::expects_unauthorized(&response);
    }
}

pub fn holds(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let fee_schedule = FeeSchedule::find(organization.fee_schedule_id, connection).unwrap();
    let event = database
        .create_event()
        .with_organization(&organization)
        .with_ticket_pricing()
        .finish();
    let hold = database
        .create_hold()
        .with_name("Hold 1".to_string())
        .with_event(&event)
        .finish();
    let hold2 = database
        .create_hold()
        .with_name("Hold 2".to_string())
        .with_event(&event)
        .finish();
    let comp = database.create_comp().with_quantity(2).with_hold(&hold).finish();
    let _comp2 = database.create_comp().with_quantity(1).with_hold(&comp).finish();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);

    #[derive(Serialize)]
    struct R {
        pub id: Uuid,
        pub name: String,
        pub event_id: Uuid,
        pub redemption_code: Option<String>,
        pub discount_in_cents: Option<i64>,
        pub end_at: Option<NaiveDateTime>,
        pub max_per_user: Option<i64>,
        pub hold_type: HoldTypes,
        pub ticket_type_id: Uuid,
        pub ticket_type_name: String,
        pub price_in_cents: Option<u32>,
        pub available: u32,
        pub quantity: u32,
        pub children_available: u32,
        pub children_quantity: u32,
        pub parent_hold_id: Option<Uuid>,
    }

    let ticket_type = UserDisplayTicketType::from_ticket_type(
        &TicketType::find(hold.ticket_type_id, connection).unwrap(),
        &fee_schedule,
        false,
        hold.redemption_code.clone(),
        connection,
    )
    .unwrap();
    let ticket_type2 = UserDisplayTicketType::from_ticket_type(
        &TicketType::find(hold2.ticket_type_id, connection).unwrap(),
        &fee_schedule,
        false,
        hold2.redemption_code.clone(),
        connection,
    )
    .unwrap();

    let all_holds = vec![
        R {
            id: hold.id,
            name: hold.name,
            event_id: hold.event_id,
            redemption_code: hold.redemption_code,
            discount_in_cents: hold.discount_in_cents,
            end_at: hold.end_at,
            max_per_user: hold.max_per_user,
            hold_type: hold.hold_type,
            ticket_type_id: hold.ticket_type_id,
            ticket_type_name: ticket_type.name,
            price_in_cents: ticket_type.ticket_pricing.map(|tp| tp.price_in_cents as u32),
            available: 8,
            quantity: 8,
            children_available: 2,
            children_quantity: 2,
            parent_hold_id: None,
        },
        R {
            id: hold2.id,
            name: hold2.name,
            event_id: hold2.event_id,
            redemption_code: hold2.redemption_code,
            discount_in_cents: hold2.discount_in_cents,
            end_at: hold2.end_at,
            max_per_user: hold2.max_per_user,
            hold_type: hold2.hold_type,
            ticket_type_id: hold2.ticket_type_id,
            ticket_type_name: ticket_type2.name,
            price_in_cents: ticket_type2.ticket_pricing.map(|tp| tp.price_in_cents as u32),
            available: 10,
            quantity: 10,
            children_available: 0,
            children_quantity: 0,
            parent_hold_id: None,
        },
    ];

    let test_request = TestRequest::create();

    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = event.id;
    let test_request = TestRequest::create_with_uri(&format!("/holds"));
    let query_parameters = Query::<PagingParameters>::extract(&test_request.request).unwrap();

    let response: HttpResponse =
        events::holds((database.connection.clone().into(), query_parameters, path, auth_user)).into();
    let expected_holds = Payload {
        data: all_holds,
        paging: Paging {
            page: 0,
            limit: 100,
            sort: "".to_string(),
            dir: SortingDir::Asc,
            total: 2,
            tags: HashMap::new(),
        },
    };
    let expected_json = serde_json::to_string(&expected_holds).unwrap();
    let body = support::unwrap_body_to_string(&response).unwrap();
    if should_test_succeed {
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(body, expected_json);
    } else {
        support::expects_unauthorized(&response);
    }
}

pub fn expected_show_json(
    role: Roles,
    event: Event,
    organization: Organization,
    venue: Venue,
    box_office_pricing: bool,
    redemption_code: Option<String>,
    filter_ticket_type_ids: Option<Vec<Uuid>>,
    connection: &PgConnection,
    interested_users: u32,
    status: Option<EventStatus>,
) -> String {
    #[derive(Serialize)]
    struct ShortOrganization {
        id: Uuid,
        name: String,
        slug: Option<String>,
    }

    #[derive(Serialize)]
    pub struct TicketsRemaining {
        pub ticket_type_id: Uuid,
        pub tickets_remaining: i32,
    }

    let no_tickets_remaining: Vec<TicketsRemaining> = Vec::new();

    #[derive(Serialize)]
    struct R {
        id: Uuid,
        #[serde(rename = "type")]
        response_type: String,
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        private_access_code: Option<Option<String>>,
        organization_id: Uuid,
        venue_id: Option<Uuid>,
        created_at: NaiveDateTime,
        event_start: Option<NaiveDateTime>,
        door_time: Option<NaiveDateTime>,
        event_end: Option<NaiveDateTime>,
        cancelled_at: Option<NaiveDateTime>,
        fee_in_cents: Option<i64>,
        status: EventStatus,
        publish_date: Option<NaiveDateTime>,
        promo_image_url: Option<String>,
        original_promo_image_url: Option<String>,
        cover_image_url: Option<String>,
        additional_info: Option<String>,
        top_line_info: Option<String>,
        age_limit: Option<String>,
        video_url: Option<String>,
        organization: ShortOrganization,
        venue: DisplayVenue,
        artists: Vec<DisplayEventArtist>,
        ticket_types: Vec<UserDisplayTicketType>,
        total_interest: u32,
        user_is_interested: bool,
        min_ticket_price: Option<i64>,
        max_ticket_price: Option<i64>,
        is_external: bool,
        external_url: Option<String>,
        override_status: Option<EventOverrideStatus>,
        limited_tickets_remaining: Vec<TicketsRemaining>,
        localized_times: EventLocalizedTimeStrings,
        tracking_keys: TrackingKeys,
        event_type: EventTypes,
        pub sales_start_date: Option<NaiveDateTime>,
        url: String,
        slug: String,
        facebook_pixel_key: Option<String>,
        extra_admin_data: Option<Value>,
        facebook_event_id: Option<String>,
        updated_at: NaiveDateTime,
    }

    let fee_schedule = FeeSchedule::find(organization.fee_schedule_id, connection).unwrap();
    let event_artists = EventArtist::find_all_from_event(event.id, connection).unwrap();

    let mut ticket_types = event.ticket_types(false, None, connection).unwrap();
    if let Some(ticket_type_ids) = filter_ticket_type_ids {
        ticket_types = ticket_types
            .into_iter()
            .filter(|tt| ticket_type_ids.contains(&tt.id))
            .collect::<Vec<TicketType>>();
        ticket_types.sort_by_key(|tt| tt.name.to_owned());
    }

    let mut display_ticket_types: Vec<UserDisplayTicketType> = Vec::new();
    let mut sales_start_date = Some(times::infinity());
    for tt in ticket_types {
        if tt.status != TicketTypeStatus::Cancelled {
            if sales_start_date.unwrap() > tt.start_date.clone().unwrap_or(times::infinity()) {
                sales_start_date = tt.start_date.clone();
            }
            display_ticket_types.push(
                UserDisplayTicketType::from_ticket_type(
                    &tt,
                    &fee_schedule,
                    box_office_pricing,
                    redemption_code.clone(),
                    connection,
                )
                .unwrap(),
            );
        }
    }
    let localized_times: EventLocalizedTimeStrings = event.get_all_localized_time_strings(Some(&venue));
    let (min_ticket_price, max_ticket_price) = event
        .current_ticket_pricing_range(box_office_pricing, connection)
        .unwrap();

    let fee_in_cents = event
        .client_fee_in_cents
        .unwrap_or(organization.client_event_fee_in_cents)
        + event
            .company_fee_in_cents
            .unwrap_or(organization.company_event_fee_in_cents);
    let slug = event.slug(connection).unwrap();
    serde_json::to_string(&R {
        id: event.id,
        response_type: "Event".to_string(),
        private_access_code: if vec![
            Roles::Promoter,
            Roles::OrgMember,
            Roles::OrgAdmin,
            Roles::OrgOwner,
            Roles::Admin,
        ]
        .contains(&role)
        {
            Some(event.private_access_code)
        } else {
            None
        },
        name: event.name,
        organization_id: event.organization_id,
        venue_id: event.venue_id,
        created_at: event.created_at,
        event_start: event.event_start,
        door_time: event.door_time,
        event_end: event.event_end,
        cancelled_at: event.cancelled_at,
        fee_in_cents: Some(fee_in_cents),
        status: status.unwrap_or(event.status),
        publish_date: event.publish_date,
        promo_image_url: event.promo_image_url.clone(),
        original_promo_image_url: event.promo_image_url,
        cover_image_url: event.cover_image_url,
        additional_info: event.additional_info,
        top_line_info: event.top_line_info,
        age_limit: event.age_limit,
        video_url: event.video_url,
        organization: ShortOrganization {
            id: organization.id,
            slug: Some(organization.slug(connection).unwrap().slug),
            name: organization.name,
        },
        venue: venue.for_display(connection).unwrap(),
        artists: event_artists,
        ticket_types: display_ticket_types,
        total_interest: interested_users,
        user_is_interested: true,
        min_ticket_price,
        max_ticket_price,
        is_external: event.is_external,
        external_url: event.external_url,
        override_status: event.override_status,
        limited_tickets_remaining: no_tickets_remaining,
        localized_times,
        tracking_keys: TrackingKeys { ..Default::default() },
        event_type: event.event_type,
        sales_start_date,
        url: format!("{}/tickets/{}", env::var("FRONT_END_URL").unwrap(), &slug),
        slug,
        facebook_pixel_key: None,
        extra_admin_data: None,
        facebook_event_id: None,
        updated_at: event.updated_at,
    })
    .unwrap()
}
