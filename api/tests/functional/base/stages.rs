use crate::support;
use crate::support::database::TestDatabase;
use crate::support::test_request::TestRequest;
use actix_web::{http::StatusCode, web::Path, FromRequest, HttpResponse};
use api::controllers::stages;
use api::extractors::*;
use api::models::PathParameters;
use db::models::{Roles, Stage, StageEditableAttributes};
use serde_json;

pub async fn create(role: Roles, should_succeed: bool) {
    let database = TestDatabase::new();
    let venue = database.create_venue().finish();
    let name = "Stage Example";

    let user = support::create_auth_user(role, None, &database);
    let json = Json(stages::CreateStage {
        name: name.to_string(),
        description: None,
        capacity: None,
    });

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = venue.id;
    let response: HttpResponse = stages::create((database.connection.into(), path, json, user))
        .await
        .into();

    if !should_succeed {
        support::expects_unauthorized(&response);
        return;
    }
    assert_eq!(response.status(), StatusCode::CREATED);
    let body = support::unwrap_body_to_string(&response).unwrap();
    let stage: Stage = serde_json::from_str(&body).unwrap();
    assert_eq!(stage.name, name);
}

pub async fn update(role: Roles, should_succeed: bool) {
    let database = TestDatabase::new();
    let venue = database.create_venue().finish();
    let stage = database.create_stage().with_venue_id(venue.id).finish();
    let new_name = "Updated Stage Example";

    let user = support::create_auth_user(role, None, &database);
    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = stage.id;

    let mut attributes: StageEditableAttributes = Default::default();
    attributes.name = Some(new_name.to_string());
    let json = Json(attributes);

    let response: HttpResponse = stages::update((database.connection.into(), path, json, user))
        .await
        .into();
    if !should_succeed {
        support::expects_unauthorized(&response);
        return;
    }
    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    let updated_stage: Stage = serde_json::from_str(&body).unwrap();
    assert_eq!(updated_stage.name, new_name);
}
