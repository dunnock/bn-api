use crate::support;
use crate::support::database::TestDatabase;
use crate::support::test_request::TestRequest;
use actix_web::{FromRequest, http::StatusCode, FromRequest, HttpResponse, web::Path};
use bigneon_api::controllers::artists;
use bigneon_api::extractors::*;
use bigneon_api::models::{CreateArtistRequest, PathParameters, UpdateArtistRequest};
use bigneon_db::models::*;
use serde_json;

pub async fn create(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(&user, role, None, &database);

    let name = "Artist Example";
    let bio = "Bio";
    let website_url = "http://www.example.com";

    let json = Json(CreateArtistRequest {
        organization_id: None,
        name: Some(name.to_string()),
        bio: Some(bio.to_string()),
        website_url: Some(website_url.to_string()),
        ..Default::default()
    });

    let response: HttpResponse = artists::create((database.connection.into(), json, auth_user)).await.await.into();

    if should_test_succeed {
        let body = support::unwrap_body_to_string(&response).unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let artist: Artist = serde_json::from_str(&body).unwrap();
        assert_eq!(artist.name, name);
        assert!(!artist.is_private);
    } else {
        support::expects_unauthorized(&response);
    }
}

pub async fn create_with_organization(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);

    let name = "Artist Example";
    let bio = "Bio";
    let website_url = "http://www.example.com";

    let json = Json(CreateArtistRequest {
        organization_id: Some(organization.id),
        name: Some(name.to_string()),
        bio: Some(bio.to_string()),
        website_url: Some(website_url.to_string()),
        ..Default::default()
    });

    let response: HttpResponse = artists::create((database.connection.into(), json, auth_user.clone())).await.await.into();

    if should_test_succeed {
        let body = support::unwrap_body_to_string(&response).unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let artist: Artist = serde_json::from_str(&body).unwrap();
        assert_eq!(artist.name, name);
        assert!(artist.is_private);
    } else {
        support::expects_unauthorized(&response);
    }
}

pub async fn toggle_privacy(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let artist = database.create_artist().finish();

    let auth_user = support::create_auth_user(role, None, &database);
    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = artist.id;

    let response: HttpResponse = artists::toggle_privacy((database.connection.into(), path, auth_user)).await.await.into();
    let body = support::unwrap_body_to_string(&response).unwrap();

    if should_test_succeed {
        assert_eq!(response.status(), StatusCode::OK);
        let updated_artist: Artist = serde_json::from_str(&body).unwrap();
        assert_ne!(updated_artist.is_private, artist.is_private)
    } else {
        support::expects_unauthorized(&response);
    }
}

pub async fn update(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let organization = database.create_organization().finish();
    let artist = database.create_artist().finish();
    let event = database.create_event().with_organization(&organization).finish();
    database
        .create_event_artist()
        .with_event(&event)
        .with_artist(&artist)
        .finish();
    artist
        .set_genres(&vec!["emo".to_string(), "hard-rock".to_string()], None, connection)
        .unwrap();
    event.update_genres(None, connection).unwrap();
    assert_eq!(
        artist.genres(connection).unwrap(),
        vec!["emo".to_string(), "hard-rock".to_string()]
    );
    assert_eq!(
        event.genres(connection).unwrap(),
        vec!["emo".to_string(), "hard-rock".to_string()]
    );

    let name = "New Name";
    let bio = "New Bio";
    let website_url = "http://www.example2.com";
    let auth_user = support::create_auth_user(role, Some(&organization), &database);
    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = artist.id;

    let mut attributes: UpdateArtistRequest = Default::default();
    attributes.name = Some(name.to_string());
    attributes.bio = Some(bio.to_string());
    attributes.website_url = Some(Some(website_url.to_string()));
    attributes.youtube_video_urls = Some(Vec::new());
    attributes.genres = Some(vec!["emo".to_string()]);
    let json = Json(attributes);

    let response: HttpResponse = artists::update((database.connection.clone().into(), path, json, auth_user)).await.await.into();
    let body = support::unwrap_body_to_string(&response).unwrap();

    if should_test_succeed {
        assert_eq!(response.status(), StatusCode::OK);
        let updated_artist: Artist = serde_json::from_str(&body).unwrap();
        assert_eq!(updated_artist.name, name);
        assert_eq!(artist.genres(connection).unwrap(), vec!["emo".to_string()]);
        // Trigger sync right away as normally will happen via background worker
        event.update_genres(None, connection).unwrap();
        assert_eq!(event.genres(connection).unwrap(), vec!["emo".to_string()]);
    } else {
        support::expects_unauthorized(&response);
        assert_eq!(
            artist.genres(connection).unwrap(),
            vec!["emo".to_string(), "hard-rock".to_string()]
        );
        assert_eq!(
            event.genres(connection).unwrap(),
            vec!["emo".to_string(), "hard-rock".to_string()]
        );
    }
}

pub async fn update_with_organization(role: Roles, should_test_succeed: bool, is_private: bool) {
    let database = TestDatabase::new();

    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);

    let mut artist = database.create_artist().with_organization(&organization).finish();

    if is_private {
        artist = artist.set_privacy(true, database.connection.get()).unwrap();
    }

    let name = "New Name";
    let bio = "New Bio";
    let website_url = "http://www.example2.com";

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = artist.id;

    let mut attributes: UpdateArtistRequest = Default::default();
    attributes.name = Some(name.to_string());
    attributes.bio = Some(bio.to_string());
    attributes.website_url = Some(Some(website_url.to_string()));
    attributes.youtube_video_urls = Some(Vec::new());
    let json = Json(attributes);

    let response: HttpResponse = artists::update((database.connection.into(), path, json, auth_user.clone())).await.await.into();
    if should_test_succeed {
        let body = support::unwrap_body_to_string(&response).unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let updated_artist: Artist = serde_json::from_str(&body).unwrap();
        assert_eq!(updated_artist.name, name);
    } else {
        support::expects_unauthorized(&response);
    }
}
