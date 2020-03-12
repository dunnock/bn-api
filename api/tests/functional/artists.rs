use crate::functional::base;
use crate::support;
use crate::support::database::TestDatabase;
use crate::support::test_request::TestRequest;
use actix_web::{
    http::StatusCode,
    web::{Path, Query},
    FromRequest, HttpResponse,
};
use bigneon_api::controllers::artists;
use bigneon_api::extractors::*;
use bigneon_api::models::{CreateArtistRequest, PathParameters, UpdateArtistRequest};
use bigneon_db::prelude::*;
use serde_json;
use std::collections::HashMap;
use uuid::Uuid;

#[actix_rt::test]
async fn index() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let artist = database.create_artist().with_name("Artist1".to_string()).finish();
    let artist2 = database.create_artist().with_name("Artist2".to_string()).finish();

    let expected_artists = vec![
        artist.for_display(connection).unwrap(),
        artist2.for_display(connection).unwrap(),
    ];
    let test_request = TestRequest::create_with_uri(&format!("/limits?"));
    let query_parameters = Query::<PagingParameters>::extract(&test_request.request).await.unwrap();
    let response: HttpResponse =
        artists::index((database.connection.clone().into(), query_parameters, OptionalUser(None)))
            .await
            .into();

    let wrapped_expected_artists = Payload {
        data: expected_artists,
        paging: Paging {
            page: 0,
            limit: 100,
            sort: "".to_string(),
            dir: SortingDir::Asc,
            total: 2,
            tags: HashMap::new(),
        },
    };
    let expected_json = serde_json::to_string(&wrapped_expected_artists).unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(body, expected_json);
}

#[actix_rt::test]
async fn index_with_org_linked_and_private_venues() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let artist = database.create_artist().with_name("Artist1".to_string()).finish();
    let artist2 = database.create_artist().with_name("Artist2".to_string()).finish();

    let org1 = database.create_organization().finish();
    let artist3 = database
        .create_artist()
        .with_name("Artist3".to_string())
        .with_organization(&org1)
        .finish();

    let artist4 = database
        .create_artist()
        .make_private()
        .with_name("Artist4".to_string())
        .with_organization(&org1)
        .finish();

    let test_request = TestRequest::create_with_uri(&format!("/limits?"));
    let query_parameters = Query::<PagingParameters>::extract(&test_request.request).await.unwrap();
    //first try with no user
    let response: HttpResponse =
        artists::index((database.connection.clone().into(), query_parameters, OptionalUser(None)))
            .await
            .into();

    let mut expected_artists = vec![
        artist.for_display(connection).unwrap(),
        artist2.for_display(connection).unwrap(),
        artist3.for_display(connection).unwrap(),
    ];

    let body = support::unwrap_body_to_string(&response).unwrap();
    let wrapped_expected_artists = Payload {
        data: expected_artists.clone(),
        paging: Paging {
            page: 0,
            limit: 100,
            sort: "".to_string(),
            dir: SortingDir::Asc,
            total: 3,
            tags: HashMap::new(),
        },
    };
    let expected_json = serde_json::to_string(&wrapped_expected_artists).unwrap();
    assert_eq!(body, expected_json);

    //now try with user that does not belong to org
    let user = support::create_auth_user(Roles::User, None, &database);
    let user_id = user.id();
    let query_parameters = Query::<PagingParameters>::extract(&test_request.request).await.unwrap();
    let response: HttpResponse = artists::index((
        database.connection.clone().into(),
        query_parameters,
        OptionalUser(Some(user.clone())),
    ))
    .await
    .into();

    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(body, expected_json);

    //now with user that DOES belong to org
    let _ = org1.add_user(
        user_id,
        vec![Roles::OrgMember],
        Vec::new(),
        database.connection.clone().get(),
    );
    expected_artists.push(artist4.for_display(connection).unwrap());
    let query_parameters = Query::<PagingParameters>::extract(&test_request.request).await.unwrap();
    let response: HttpResponse = artists::index((
        database.connection.clone().into(),
        query_parameters,
        OptionalUser(Some(user)),
    ))
    .await
    .into();
    let wrapped_expected_artists = Payload {
        data: expected_artists.clone(),
        paging: Paging {
            page: 0,
            limit: 100,
            sort: "".to_string(),
            dir: SortingDir::Asc,
            total: 4,
            tags: HashMap::new(),
        },
    };
    let expected_json = serde_json::to_string(&wrapped_expected_artists).unwrap();
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(body, expected_json);

    //now with an admin user
    let admin = support::create_auth_user(Roles::Admin, None, &database);
    let query_parameters = Query::<PagingParameters>::extract(&test_request.request).await.unwrap();
    let response: HttpResponse = artists::index((
        database.connection.clone().into(),
        query_parameters,
        OptionalUser(Some(admin)),
    ))
    .await
    .into();
    let wrapped_expected_artists = Payload {
        data: expected_artists,
        paging: Paging {
            page: 0,
            limit: 100,
            sort: "".to_string(),
            dir: SortingDir::Asc,
            total: 4,
            tags: HashMap::new(),
        },
    };
    let expected_json = serde_json::to_string(&wrapped_expected_artists).unwrap();
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(body, expected_json);
}

#[actix_rt::test]
pub async fn search_no_spotify() {
    let database = TestDatabase::new();
    let artist = database.create_artist().with_name("Artist1".to_string()).finish();

    let expected_artists = vec![artist.id];
    let test_request = TestRequest::create_with_uri(&format!("/?q=Artist&spotify=1"));
    let query_parameters = Query::<PagingParameters>::extract(&test_request.request).await.unwrap();
    let response = artists::search((database.connection.into(), query_parameters, OptionalUser(None)))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let collected_ids = response
        .payload()
        .data
        .iter()
        .map(|i| i.id.unwrap_or(Uuid::new_v4()))
        .collect::<Vec<Uuid>>();

    assert_eq!(expected_artists, collected_ids);
}

#[actix_rt::test]
pub async fn search_with_spotify() {
    let database = TestDatabase::new();
    let _artist = database.create_artist().with_name("Artist1".to_string()).finish();

    let test_request = TestRequest::create_with_uri(&format!("/?q=Powerwolf&spotify=1"));
    let query_parameters = Query::<PagingParameters>::extract(&test_request.request).await.unwrap();
    let response = artists::search((database.connection.into(), query_parameters, OptionalUser(None)))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let spotify_results = response
        .payload()
        .data
        .iter()
        .filter(|i| i.spotify_id.is_some())
        .count();
    if test_request.extract_state().await.config.spotify_auth_token.is_some() {
        assert_ne!(spotify_results, 0);
    } else {
        assert_eq!(spotify_results, 0);
    }
}

#[actix_rt::test]
pub async fn show() {
    let database = TestDatabase::new();
    let artist = database.create_artist().finish();
    let artist_expected_json =
        serde_json::to_string(&artist.clone().for_display(database.connection.get()).unwrap()).unwrap();

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = artist.id;

    let response: HttpResponse = artists::show((database.connection.into(), path)).await.into();
    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(body, artist_expected_json);
}

#[actix_rt::test]
pub async fn show_from_organizations_private_artist_same_org() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let organization = database
        .create_organization()
        .with_member(&user, Roles::OrgMember)
        .finish();
    let artist = database
        .create_artist()
        .with_name("Artist 1".to_string())
        .with_organization(&organization)
        .finish();
    let artist2 = database
        .create_artist()
        .with_name("Artist 2".to_string())
        .with_organization(&organization)
        .make_private()
        .finish();

    let user2 = database.create_user().finish();

    let all_artists = vec![
        artist.for_display(connection).unwrap(),
        artist2.for_display(connection).unwrap(),
    ];
    let wrapped_expected_artists = Payload {
        data: all_artists,
        paging: Paging {
            page: 0,
            limit: 100,
            sort: "".to_string(),
            dir: SortingDir::Asc,
            total: 2,
            tags: HashMap::new(),
        },
    };
    let expected_json = serde_json::to_string(&wrapped_expected_artists).unwrap();
    let test_request = TestRequest::create();

    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = organization.id;

    let user = support::create_auth_user_from_user(&user2, Roles::OrgOwner, Some(&organization), &database);

    let test_request = TestRequest::create_with_uri(&format!("/limits?"));
    let query_parameters = Query::<PagingParameters>::extract(&test_request.request).await.unwrap();
    let response: HttpResponse = artists::show_from_organizations((
        database.connection.clone().into(),
        path,
        query_parameters,
        OptionalUser(Some(user)),
    ))
    .await
    .into();

    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(expected_json, body);
}

#[cfg(test)]
mod create_tests {
    use super::*;
    #[actix_rt::test]
    async fn create_org_member() {
        base::artists::create(Roles::OrgMember, false).await;
    }
    #[actix_rt::test]
    async fn create_admin() {
        base::artists::create(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn create_user() {
        base::artists::create(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn create_org_owner() {
        base::artists::create(Roles::OrgOwner, false).await;
    }
    #[actix_rt::test]
    async fn create_door_person() {
        base::artists::create(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn create_promoter() {
        base::artists::create(Roles::Promoter, false).await;
    }
    #[actix_rt::test]
    async fn create_promoter_read_only() {
        base::artists::create(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn create_org_admin() {
        base::artists::create(Roles::OrgAdmin, false).await;
    }
    #[actix_rt::test]
    async fn create_box_office() {
        base::artists::create(Roles::OrgBoxOffice, false).await;
    }
    #[actix_rt::test]
    async fn create_with_organization_org_member() {
        base::artists::create_with_organization(Roles::OrgMember, true).await;
    }
    #[actix_rt::test]
    async fn create_with_organization_admin() {
        base::artists::create_with_organization(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn create_with_organization_user() {
        base::artists::create_with_organization(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn create_with_organization_org_owner() {
        base::artists::create_with_organization(Roles::OrgOwner, true).await;
    }
    #[actix_rt::test]
    async fn create_with_organization_door_person() {
        base::artists::create_with_organization(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn create_with_organization_promoter() {
        base::artists::create_with_organization(Roles::Promoter, false).await;
    }
    #[actix_rt::test]
    async fn create_with_organization_promoter_read_only() {
        base::artists::create_with_organization(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn create_with_organization_org_admin() {
        base::artists::create_with_organization(Roles::OrgAdmin, true).await;
    }
    #[actix_rt::test]
    async fn create_with_organization_box_office() {
        base::artists::create_with_organization(Roles::OrgBoxOffice, false).await;
    }
}

#[actix_rt::test]
pub async fn create_with_validation_errors() {
    let database = TestDatabase::new();

    let name = "Artist Example";
    let bio = "Bio";
    let website_url = "invalid-format.com";
    let json = Json(CreateArtistRequest {
        name: Some(name.to_string()),
        bio: Some(bio.to_string()),
        website_url: Some(website_url.to_string()),
        youtube_video_urls: Some(vec!["invalid".to_string()]),
        ..Default::default()
    });
    let user = support::create_auth_user(Roles::Admin, None, &database);
    let response: HttpResponse = artists::create((database.connection.into(), json, user)).await.into();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(response.error().is_some());
    let validation_response = support::validation_response_from_response(&response).unwrap();
    let website_url = validation_response.fields.get("website_url").unwrap();
    assert_eq!(website_url[0].code, "url");
    assert_eq!(
        &website_url[0].message.clone().unwrap().into_owned(),
        "Website URL is invalid"
    );
    let youtube_video_urls = validation_response.fields.get("youtube_video_urls").unwrap();
    assert_eq!(youtube_video_urls[0].code, "url");
    assert_eq!(
        &youtube_video_urls[0].message.clone().unwrap().into_owned(),
        "URL is invalid"
    );
}

#[cfg(test)]
mod toggle_privacy_tests {
    use super::*;
    #[actix_rt::test]
    async fn toggle_privacy_org_member() {
        base::artists::toggle_privacy(Roles::OrgMember, false).await;
    }
    #[actix_rt::test]
    async fn toggle_privacy_admin() {
        base::artists::toggle_privacy(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn toggle_privacy_user() {
        base::artists::toggle_privacy(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn toggle_privacy_org_owner() {
        base::artists::toggle_privacy(Roles::OrgOwner, false).await;
    }
    #[actix_rt::test]
    async fn toggle_privacy_door_person() {
        base::artists::toggle_privacy(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn toggle_privacy_promoter() {
        base::artists::toggle_privacy(Roles::Promoter, false).await;
    }
    #[actix_rt::test]
    async fn toggle_privacy_promoter_read_only() {
        base::artists::toggle_privacy(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn toggle_privacy_org_admin() {
        base::artists::toggle_privacy(Roles::OrgAdmin, false).await;
    }
    #[actix_rt::test]
    async fn toggle_privacy_box_office() {
        base::artists::toggle_privacy(Roles::OrgBoxOffice, false).await;
    }
}

#[cfg(test)]
mod update_tests {
    use super::*;
    #[actix_rt::test]
    async fn update_org_member() {
        base::artists::update(Roles::OrgMember, false).await;
    }
    #[actix_rt::test]
    async fn update_admin() {
        base::artists::update(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn update_user() {
        base::artists::update(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn update_org_owner() {
        base::artists::update(Roles::OrgOwner, false).await;
    }
    #[actix_rt::test]
    async fn update_door_person() {
        base::artists::update(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn update_promoter() {
        base::artists::update(Roles::Promoter, false).await;
    }
    #[actix_rt::test]
    async fn update_promoter_read_only() {
        base::artists::update(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn update_org_admin() {
        base::artists::update(Roles::OrgAdmin, false).await;
    }
    #[actix_rt::test]
    async fn update_box_office() {
        base::artists::update(Roles::OrgBoxOffice, false).await;
    }
}

#[cfg(test)]
mod update_with_organization_tests {
    use super::*;
    #[actix_rt::test]
    async fn update_with_organization_org_member() {
        base::artists::update_with_organization(Roles::OrgMember, true, true).await;
    }
    #[actix_rt::test]
    async fn update_with_organization_admin() {
        base::artists::update_with_organization(Roles::Admin, true, true).await;
    }
    #[actix_rt::test]
    async fn update_with_organization_user() {
        base::artists::update_with_organization(Roles::User, false, true).await;
    }
    #[actix_rt::test]
    async fn update_with_organization_org_owner() {
        base::artists::update_with_organization(Roles::OrgOwner, true, true).await;
    }
    #[actix_rt::test]
    async fn update_with_organization_door_person() {
        base::artists::update_with_organization(Roles::DoorPerson, false, true).await;
    }
    #[actix_rt::test]
    async fn update_with_organization_promoter() {
        base::artists::update_with_organization(Roles::Promoter, false, true).await;
    }
    #[actix_rt::test]
    async fn update_with_organization_promoter_read_only() {
        base::artists::update_with_organization(Roles::PromoterReadOnly, false, true).await;
    }
    #[actix_rt::test]
    async fn update_with_organization_org_admin() {
        base::artists::update_with_organization(Roles::OrgAdmin, true, true).await;
    }
    #[actix_rt::test]
    async fn update_with_organization_box_office() {
        base::artists::update_with_organization(Roles::OrgBoxOffice, false, true).await;
    }
}

#[cfg(test)]
mod update_public_artist_with_organization_tests {
    use super::*;
    #[actix_rt::test]
    async fn update_public_artist_with_organization_org_member() {
        base::artists::update_with_organization(Roles::OrgMember, false, false).await;
    }
    #[actix_rt::test]
    async fn update_public_artist_with_organization_admin() {
        base::artists::update_with_organization(Roles::Admin, true, false).await;
    }
    #[actix_rt::test]
    async fn update_public_artist_with_organization_user() {
        base::artists::update_with_organization(Roles::User, false, false).await;
    }
    #[actix_rt::test]
    async fn update_public_artist_with_organization_org_owner() {
        base::artists::update_with_organization(Roles::OrgOwner, false, false).await;
    }
    #[actix_rt::test]
    async fn update_public_artist_with_organization_door_person() {
        base::artists::update_with_organization(Roles::DoorPerson, false, false).await;
    }
    #[actix_rt::test]
    async fn update_public_artist_with_organization_promoter() {
        base::artists::update_with_organization(Roles::Promoter, false, false).await;
    }
    #[actix_rt::test]
    async fn update_public_artist_with_organization_promoter_read_only() {
        base::artists::update_with_organization(Roles::PromoterReadOnly, false, false).await;
    }
    #[actix_rt::test]
    async fn update_public_artist_with_organization_org_admin() {
        base::artists::update_with_organization(Roles::OrgAdmin, false, false).await;
    }
    #[actix_rt::test]
    async fn update_public_artist_with_organization_box_office() {
        base::artists::update_with_organization(Roles::OrgBoxOffice, false, false).await;
    }
}

#[actix_rt::test]
pub async fn update_with_validation_errors() {
    let database = TestDatabase::new();
    let artist = database.create_artist().finish();
    let name = "New Name";
    let bio = "New Bio";
    let website_url = "invalid-format.com";

    let user = support::create_auth_user(Roles::Admin, None, &database);
    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = artist.id;

    let mut attributes: UpdateArtistRequest = Default::default();
    attributes.name = Some(name.to_string());
    attributes.bio = Some(bio.to_string());
    attributes.website_url = Some(Some(website_url.to_string()));
    attributes.youtube_video_urls = Some(vec!["invalid".to_string()]);
    let json = Json(attributes);

    let response: HttpResponse = artists::update((database.connection.into(), path, json, user))
        .await
        .into();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(response.error().is_some());
    let validation_response = support::validation_response_from_response(&response).unwrap();
    let website_url = validation_response.fields.get("website_url").unwrap();
    assert_eq!(website_url[0].code, "url");
    assert_eq!(
        &website_url[0].message.clone().unwrap().into_owned(),
        "Website URL is invalid"
    );
    let youtube_video_urls = validation_response.fields.get("youtube_video_urls").unwrap();
    assert_eq!(youtube_video_urls[0].code, "url");
    assert_eq!(
        &youtube_video_urls[0].message.clone().unwrap().into_owned(),
        "URL is invalid"
    );
}
