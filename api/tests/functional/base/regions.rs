use crate::support;
use crate::support::database::TestDatabase;
use crate::support::test_request::TestRequest;
use actix_web::{http::StatusCode, web::Path, FromRequest, HttpResponse};
use bigneon_api::controllers::regions;
use bigneon_api::extractors::*;
use bigneon_api::models::PathParameters;
use bigneon_db::models::{NewRegion, Region, RegionEditableAttributes, Roles};
use serde_json;

pub async fn create(role: Roles, should_succeed: bool) {
    let database = TestDatabase::new();
    let name = "Region Example";

    let user = support::create_auth_user(role, None, &database);
    let json = Json(NewRegion { name: name.to_string() });

    let response: HttpResponse = regions::create((database.connection.into(), json, user)).await.into();

    if !should_succeed {
        support::expects_unauthorized(&response);
        return;
    }
    assert_eq!(response.status(), StatusCode::CREATED);
    let body = support::unwrap_body_to_string(&response).unwrap();
    let region: Region = serde_json::from_str(&body).unwrap();
    assert_eq!(region.name, name);
}

pub async fn update(role: Roles, should_succeed: bool) {
    let database = TestDatabase::new();
    let region = database.create_region().finish();
    let new_name = "New Name";

    let user = support::create_auth_user(role, None, &database);
    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = region.id;

    let mut attributes: RegionEditableAttributes = Default::default();
    attributes.name = Some(new_name.to_string());
    let json = Json(attributes);

    let response: HttpResponse = regions::update((database.connection.into(), path, json, user))
        .await
        .into();
    if !should_succeed {
        support::expects_unauthorized(&response);
        return;
    }
    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    let updated_region: Region = serde_json::from_str(&body).unwrap();
    assert_eq!(updated_region.name, new_name);
}
