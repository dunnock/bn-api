use crate::support;
use crate::support::database::TestDatabase;
use crate::support::test_request::TestRequest;
use actix_web::{
    http::StatusCode,
    web::{Path, Query},
    FromRequest, HttpResponse,
};
use api::controllers::announcements;
use api::extractors::*;
use api::models::*;
use db::models::*;
use db::utils::dates;
use diesel;
use diesel::query_dsl::RunQueryDsl;
use diesel::sql_types;
use serde_json;
use std::collections::HashMap;

pub async fn create(role: Roles, should_succeed: bool) {
    let database = TestDatabase::new();
    let message = "Announcement Example";

    let user = support::create_auth_user(role, None, &database);
    let json = Json(NewAnnouncement {
        message: message.to_string(),
        organization_id: None,
    });

    let response: HttpResponse = announcements::create((database.connection.into(), json, user))
        .await
        .into();

    if !should_succeed {
        support::expects_unauthorized(&response);
        return;
    }
    assert_eq!(response.status(), StatusCode::CREATED);
    let body = support::unwrap_body_to_string(&response).unwrap();
    let announcement: Announcement = serde_json::from_str(&body).unwrap();
    assert_eq!(announcement.message, message);
    assert!(announcement.organization_id.is_none());
}

pub async fn update(role: Roles, should_succeed: bool) {
    let database = TestDatabase::new();
    let announcement = database.create_announcement().finish();
    let new_message = "New Message";

    let user = support::create_auth_user(role, None, &database);
    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = announcement.id;

    let mut attributes: AnnouncementEditableAttributes = Default::default();
    attributes.message = Some(new_message.to_string());
    let json = Json(attributes);

    let response: HttpResponse = announcements::update((database.connection.into(), path, json, user))
        .await
        .into();
    if !should_succeed {
        support::expects_unauthorized(&response);
        return;
    }
    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    let updated_announcement: Announcement = serde_json::from_str(&body).unwrap();
    assert_eq!(updated_announcement.message, new_message);
}

pub async fn destroy(role: Roles, should_succeed: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let organization = database.create_organization().with_event_fee().with_fees().finish();
    let announcement = database.create_announcement().finish();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = announcement.id;

    let response: HttpResponse = announcements::destroy((database.connection.clone().into(), path, auth_user))
        .await
        .into();

    if should_succeed {
        assert_eq!(response.status(), StatusCode::OK);
        let announcement = Announcement::find(announcement.id, true, connection).unwrap();
        assert!(announcement.deleted_at.is_some());
    } else {
        support::expects_unauthorized(&response);
    }
}

pub async fn index(role: Roles, should_succeed: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let announcement = database.create_announcement().finish();
    diesel::sql_query(
        r#"
        UPDATE announcements
        SET created_at = $1
        WHERE id = $2;
        "#,
    )
    .bind::<sql_types::Timestamp, _>(dates::now().add_hours(-1).finish())
    .bind::<sql_types::Uuid, _>(announcement.id)
    .execute(connection)
    .unwrap();
    let announcement = Announcement::find(announcement.id, false, connection).unwrap();
    let announcement2 = database.create_announcement().finish();

    let expected_announcements = vec![announcement, announcement2];
    let wrapped_expected_announcements = Payload {
        data: expected_announcements,
        paging: Paging {
            page: 0,
            limit: 100,
            sort: "".to_string(),
            dir: SortingDir::Asc,
            total: 2,
            tags: HashMap::new(),
        },
    };

    let test_request = TestRequest::create();
    let query_parameters = Query::<PagingParameters>::extract(&test_request.request).await.unwrap();

    let user = support::create_auth_user(role, None, &database);
    let response = announcements::index((database.connection.into(), query_parameters, user)).await;

    if !should_succeed {
        assert_eq!(
            response.err().unwrap().to_string(),
            "User does not have the required permissions"
        );
        return;
    }

    let response = response.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(wrapped_expected_announcements, *response.payload());
}

pub async fn show(role: Roles, should_succeed: bool) {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);
    let announcement = database.create_announcement().finish();

    let expected_json = serde_json::to_string(&announcement).unwrap();

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = announcement.id;
    let response: HttpResponse = announcements::show((database.connection.into(), path, auth_user.clone()))
        .await
        .into();

    if !should_succeed {
        support::expects_unauthorized(&response);
        return;
    }

    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(body, expected_json);
}
