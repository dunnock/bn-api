use crate::functional::{base::events, events::event_venue_entry};
use crate::support;
use crate::support::database::TestDatabase;
use crate::support::test_request::TestRequest;
use actix_web::{
    http::StatusCode,
    web::{Path, Query},
    FromRequest, HttpResponse,
};
use api::controllers::events::*;
use api::controllers::slugs;
use api::controllers::slugs::*;
use api::extractors::*;
use api::models::*;
use db::models::*;
use serde_json;
use std::env;

#[actix_rt::test]
async fn show_event() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);
    let conn = database.connection.get();
    let organization = database.create_organization().finish();
    let venue = database.create_venue().finish();
    let event = database
        .create_event()
        .with_name("NewEvent1".to_string())
        .with_organization(&organization)
        .with_venue(&venue)
        .finish();
    let _event_interest = EventInterest::create(event.id, user.id).commit(conn);
    let slug = "newevent1-san-francisco";
    let test_request = TestRequest::create_with_uri(&format!("/{}", slug));
    let mut path = Path::<StringPathParameters>::extract(&test_request.request)
        .await
        .unwrap();
    let event_expected_json = events::expected_show_json(
        Roles::User,
        event.clone(),
        organization.clone(),
        venue.clone(),
        false,
        None,
        None,
        conn,
        1,
        None,
    );
    path.id = slug.to_string();
    let query_parameters = Query::<EventParameters>::extract(&test_request.request).await.unwrap();

    let response: HttpResponse = slugs::show((
        test_request.extract_state().await,
        database.connection.clone().into(),
        path,
        query_parameters,
        OptionalUser(Some(auth_user.clone())),
        RequestInfo {
            user_agent: Some("test".to_string()),
        },
    ))
    .await
    .into();
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body, event_expected_json);
}

#[actix_rt::test]
async fn show_redirect_to_primary_slug() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);
    let conn = database.connection.get();
    let organization = database
        .create_organization()
        .with_name("Organization1".to_string())
        .finish();
    let venue = database.create_venue().with_name("Venue1".to_string()).finish();
    let event = database
        .create_event()
        .with_name("NewEvent1".to_string())
        .with_organization(&organization)
        .with_venue(&venue)
        .finish();
    let _event_interest = EventInterest::create(event.id, user.id).commit(conn);
    let mut slug_redirects: Vec<(Slug, &str, &str)> = Vec::new();
    let slug = database
        .create_slug()
        .for_event(&event)
        .with_slug("redirect-me")
        .finish();
    slug_redirects.push((slug, "newevent1-san-francisco", "tickets"));
    let slug = database
        .create_slug()
        .for_venue(&venue, SlugTypes::Venue)
        .with_slug("redirect-me2")
        .finish();
    slug_redirects.push((slug, "venue1", "venues"));
    let slug = database
        .create_slug()
        .for_organization(&organization)
        .with_slug("redirect-me3")
        .finish();
    slug_redirects.push((slug, "organization1", "organizations"));

    for (slug, expected_redirect_slug, expected_path) in slug_redirects {
        let test_request = TestRequest::create_with_uri(&format!("/{}", &slug.slug));
        let mut path = Path::<StringPathParameters>::extract(&test_request.request)
            .await
            .unwrap();
        path.id = slug.slug.to_string();
        let query_parameters = Query::<EventParameters>::extract(&test_request.request).await.unwrap();

        let response: HttpResponse = slugs::show((
            test_request.extract_state().await,
            database.connection.clone().into(),
            path,
            query_parameters,
            OptionalUser(Some(auth_user.clone())),
            RequestInfo {
                user_agent: Some("test".to_string()),
            },
        ))
        .await
        .into();
        let body = support::unwrap_body_to_string(&response).unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            body,
            json!({
                "redirect":
                    format!(
                        "{}/{}/{}",
                        env::var("FRONT_END_URL").unwrap(),
                        expected_path,
                        expected_redirect_slug
                    )
            })
            .to_string()
        );
    }
}

#[actix_rt::test]
async fn show_venue() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);
    let connection = database.connection.get();
    let organization = database.create_organization().finish();
    let venue = database.create_venue().with_name("Venue1".to_string()).finish();
    let venue2 = database.create_venue().with_name("Venue2".to_string()).finish();
    let event = database
        .create_event()
        .with_name("NewEvent1".to_string())
        .with_organization(&organization)
        .with_venue(&venue)
        .finish();
    let event2 = database
        .create_event()
        .with_name("NewEvent2".to_string())
        .with_organization(&organization)
        .with_venue(&venue)
        .finish();
    let event3 = database
        .create_event()
        .with_name("NewEvent3".to_string())
        .with_organization(&organization)
        .with_venue(&venue2)
        .finish();

    let slug = "venue1";
    let test_request = TestRequest::create_with_uri(&format!("/{}", slug));
    let mut path = Path::<StringPathParameters>::extract(&test_request.request)
        .await
        .unwrap();
    path.id = slug.to_string();
    let query_parameters = Query::<EventParameters>::extract(&test_request.request).await.unwrap();
    let response: HttpResponse = slugs::show((
        test_request.extract_state().await,
        database.connection.clone().into(),
        path,
        query_parameters,
        OptionalUser(Some(auth_user.clone())),
        RequestInfo {
            user_agent: Some("test".to_string()),
        },
    ))
    .await
    .into();
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let expected_events = vec![
        event_venue_entry(&event, &venue, &vec![], None, &*connection),
        event_venue_entry(&event2, &venue, &vec![], None, &*connection),
    ];
    let expected_json = serde_json::to_string(&SlugResponse::Venue {
        venue: venue.for_display(connection).unwrap(),
        events: expected_events,
        meta: SlugMetaData {
            title: None,
            description: None,
        },
    })
    .unwrap();
    assert_eq!(body, &expected_json);

    let slug = "venue2";
    let test_request = TestRequest::create_with_uri(&format!("/{}", slug));
    let mut path = Path::<StringPathParameters>::extract(&test_request.request)
        .await
        .unwrap();
    path.id = slug.to_string();
    let query_parameters = Query::<EventParameters>::extract(&test_request.request).await.unwrap();
    let response: HttpResponse = slugs::show((
        test_request.extract_state().await,
        database.connection.clone().into(),
        path,
        query_parameters,
        OptionalUser(Some(auth_user.clone())),
        RequestInfo {
            user_agent: Some("test".to_string()),
        },
    ))
    .await
    .into();
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let expected_events = vec![event_venue_entry(&event3, &venue2, &vec![], None, &*connection)];
    let expected_json = serde_json::to_string(&SlugResponse::Venue {
        venue: venue2.for_display(connection).unwrap(),
        events: expected_events,
        meta: SlugMetaData {
            title: None,
            description: None,
        },
    })
    .unwrap();
    assert_eq!(body, &expected_json);
}

#[actix_rt::test]
async fn show_organization() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);
    let connection = database.connection.get();
    let organization = database
        .create_organization()
        .with_name("Organization1".to_string())
        .finish();
    let organization2 = database
        .create_organization()
        .with_name("Organization2".to_string())
        .finish();
    let venue = database.create_venue().finish();
    let venue2 = database.create_venue().finish();
    let event = database
        .create_event()
        .with_name("NewEvent1".to_string())
        .with_organization(&organization)
        .with_venue(&venue)
        .finish();
    let event2 = database
        .create_event()
        .with_name("NewEvent2".to_string())
        .with_organization(&organization2)
        .with_venue(&venue)
        .finish();
    let event3 = database
        .create_event()
        .with_name("NewEvent3".to_string())
        .with_organization(&organization)
        .with_venue(&venue2)
        .finish();

    let slug = "organization1";
    let test_request = TestRequest::create_with_uri(&format!("/{}", slug));
    let mut path = Path::<StringPathParameters>::extract(&test_request.request)
        .await
        .unwrap();
    path.id = slug.to_string();
    let query_parameters = Query::<EventParameters>::extract(&test_request.request).await.unwrap();
    let response: HttpResponse = slugs::show((
        test_request.extract_state().await,
        database.connection.clone().into(),
        path,
        query_parameters,
        OptionalUser(Some(auth_user.clone())),
        RequestInfo {
            user_agent: Some("test".to_string()),
        },
    ))
    .await
    .into();
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let expected_events = vec![
        event_venue_entry(&event, &venue, &vec![], None, &*connection),
        event_venue_entry(&event3, &venue2, &vec![], None, &*connection),
    ];
    let expected_json = serde_json::to_string(&SlugResponse::Organization {
        organization: organization.for_display(connection).unwrap(),
        events: expected_events,
        meta: SlugMetaData {
            title: None,
            description: None,
        },
    })
    .unwrap();
    assert_eq!(body, &expected_json);

    let slug = "organization2";
    let test_request = TestRequest::create_with_uri(&format!("/{}", slug));
    let mut path = Path::<StringPathParameters>::extract(&test_request.request)
        .await
        .unwrap();
    path.id = slug.to_string();
    let query_parameters = Query::<EventParameters>::extract(&test_request.request).await.unwrap();
    let response: HttpResponse = slugs::show((
        test_request.extract_state().await,
        database.connection.clone().into(),
        path,
        query_parameters,
        OptionalUser(Some(auth_user.clone())),
        RequestInfo {
            user_agent: Some("test".to_string()),
        },
    ))
    .await
    .into();
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let expected_events = vec![event_venue_entry(&event2, &venue, &vec![], None, &*connection)];
    let expected_json = serde_json::to_string(&SlugResponse::Organization {
        organization: organization2.for_display(connection).unwrap(),
        events: expected_events,
        meta: SlugMetaData {
            title: None,
            description: None,
        },
    })
    .unwrap();
    assert_eq!(body, &expected_json);
}

#[actix_rt::test]
async fn show_city() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);
    let connection = database.connection.get();
    let organization = database.create_organization().finish();
    let venue = database
        .create_venue()
        .with_city("San Francisco".to_string())
        .with_state("California".to_string())
        .with_country("US".to_string())
        .finish();
    let venue2 = database
        .create_venue()
        .with_city("San Francisco".to_string())
        .with_state("California".to_string())
        .with_country("US".to_string())
        .finish();
    let venue3 = database
        .create_venue()
        .with_city("Oakland".to_string())
        .with_state("California".to_string())
        .with_country("US".to_string())
        .finish();
    let event = database
        .create_event()
        .with_name("NewEvent1".to_string())
        .with_organization(&organization)
        .with_venue(&venue)
        .finish();
    let event2 = database
        .create_event()
        .with_name("NewEvent2".to_string())
        .with_organization(&organization)
        .with_venue(&venue2)
        .finish();
    let event3 = database
        .create_event()
        .with_name("NewEvent3".to_string())
        .with_organization(&organization)
        .with_venue(&venue3)
        .finish();

    let slug = "san-francisco";
    let test_request = TestRequest::create_with_uri(&format!("/{}", slug));
    let mut path = Path::<StringPathParameters>::extract(&test_request.request)
        .await
        .unwrap();
    path.id = slug.to_string();
    let query_parameters = Query::<EventParameters>::extract(&test_request.request).await.unwrap();
    let response: HttpResponse = slugs::show((
        test_request.extract_state().await,
        database.connection.clone().into(),
        path,
        query_parameters,
        OptionalUser(Some(auth_user.clone())),
        RequestInfo {
            user_agent: Some("test".to_string()),
        },
    ))
    .await
    .into();
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let expected_events = vec![
        event_venue_entry(&event, &venue, &vec![], None, &*connection),
        event_venue_entry(&event2, &venue2, &vec![], None, &*connection),
    ];
    let expected_json = serde_json::to_string(&SlugResponse::City {
        city: CityData {
            city: "San Francisco".to_string(),
            state: "California".to_string(),
            country: "US".to_string(),
            google_place_id: venue3.google_place_id.clone(),
            latitude: venue3.latitude,
            longitude: venue3.longitude,
            timezone: venue3.timezone.clone(),
        },
        events: expected_events,
        meta: SlugMetaData {
            title: None,
            description: None,
        },
    })
    .unwrap();
    assert_eq!(body, &expected_json);

    let slug = "oakland";
    let test_request = TestRequest::create_with_uri(&format!("/{}", slug));
    let mut path = Path::<StringPathParameters>::extract(&test_request.request)
        .await
        .unwrap();
    path.id = slug.to_string();
    let query_parameters = Query::<EventParameters>::extract(&test_request.request).await.unwrap();
    let response: HttpResponse = slugs::show((
        test_request.extract_state().await,
        database.connection.clone().into(),
        path,
        query_parameters,
        OptionalUser(Some(auth_user.clone())),
        RequestInfo {
            user_agent: Some("test".to_string()),
        },
    ))
    .await
    .into();
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let expected_events = vec![event_venue_entry(&event3, &venue3, &vec![], None, &*connection)];
    let expected_json = serde_json::to_string(&SlugResponse::City {
        city: CityData {
            city: "Oakland".to_string(),
            state: "California".to_string(),
            country: "US".to_string(),
            google_place_id: venue3.google_place_id.clone(),
            latitude: venue3.latitude,
            longitude: venue3.longitude,
            timezone: venue3.timezone.clone(),
        },
        events: expected_events,
        meta: SlugMetaData {
            title: None,
            description: None,
        },
    })
    .unwrap();
    assert_eq!(body, &expected_json);
}

#[actix_rt::test]
async fn show_genre() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);
    let connection = database.connection.get();
    let organization = database
        .create_organization()
        .with_name("Organization1".to_string())
        .finish();
    let organization2 = database
        .create_organization()
        .with_name("Organization2".to_string())
        .finish();
    let venue = database.create_venue().finish();
    let venue2 = database.create_venue().finish();
    let event = database
        .create_event()
        .with_name("NewEvent1".to_string())
        .with_organization(&organization)
        .with_venue(&venue)
        .finish();
    let _event2 = database
        .create_event()
        .with_name("NewEvent2".to_string())
        .with_organization(&organization2)
        .with_venue(&venue)
        .finish();
    let _event3 = database
        .create_event()
        .with_name("NewEvent3".to_string())
        .with_organization(&organization)
        .with_venue(&venue2)
        .finish();

    let custom_genre = "custom-genre-1".to_string();
    let artist = database
        .create_artist()
        .with_name("Test Artist".to_string())
        .with_genres(vec![custom_genre.clone()])
        .finish();

    let event_artist = database
        .create_event_artist()
        .with_event(&event)
        .with_artist(&artist)
        .finish();

    event.update_genres(None, connection).unwrap();

    let slug = Slug::create_slug(custom_genre.clone().as_str());
    let test_request = TestRequest::create_with_uri(&format!("/{}", slug));
    let mut path = Path::<StringPathParameters>::extract(&test_request.request)
        .await
        .unwrap();
    path.id = slug.to_string();
    let query_parameters = Query::<EventParameters>::extract(&test_request.request).await.unwrap();
    let response: HttpResponse = slugs::show((
        test_request.extract_state().await,
        database.connection.clone().into(),
        path,
        query_parameters,
        OptionalUser(Some(auth_user.clone())),
        RequestInfo {
            user_agent: Some("test".to_string()),
        },
    ))
    .await
    .into();
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let display_event_artist = DisplayEventArtist {
        artist: artist.clone(),
        event_id: event_artist.event_id.clone(),
        rank: event_artist.rank.clone(),
        set_time: event_artist.set_time.clone(),
        importance: event_artist.importance.clone(),
        stage_id: event_artist.stage_id.clone(),
    };
    let expected_events = vec![event_venue_entry(
        &event,
        &venue,
        &vec![display_event_artist],
        None,
        &*connection,
    )];
    let expected_json = serde_json::to_string(&SlugResponse::Genre {
        genre: slug,
        events: expected_events,
        meta: SlugMetaData {
            title: None,
            description: None,
        },
    })
    .unwrap();
    assert_eq!(body, &expected_json);
}
