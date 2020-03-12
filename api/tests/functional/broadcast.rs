use crate::support;
use crate::support::database::TestDatabase;
use crate::support::test_request::RequestBuilder;
use actix_web::{FromRequest, http::StatusCode, HttpResponse, Path};
use bigneon_api::controllers::broadcasts;
use bigneon_api::models::PathParameters;
use bigneon_db::models::enums::{BroadcastAudience, BroadcastChannel, BroadcastType};
use bigneon_db::models::*;
use bigneon_db::prelude::Broadcast;
use serde_json::Value;
use std::string::ToString;

#[actix_rt::test]
async fn broadcast_counter() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);
    let conn = database.connection.clone();
    let connection = database.connection.get();
    let event_id = database.create_event().finish().id;
    let broadcast = Broadcast::create(
        event_id,
        BroadcastType::Custom,
        BroadcastChannel::PushNotification,
        "Name".to_string(),
        Some("message".to_string()),
        None,
        None,
        None,
        BroadcastAudience::PeopleAtTheEvent,
        None,
    )
    .commit(connection)
    .unwrap();
    let b = Broadcast::set_sent_count(broadcast.id, 2, &connection).unwrap();
    assert_eq!(b.sent_quantity, 2);

    let broadcast_id = broadcast.id;
    let request = RequestBuilder::new(&format!("/broadcasts/{}/tracking_count", broadcast_id));
    let mut path: Path<PathParameters> = request.path();
    path.id = broadcast_id;

    let response: HttpResponse = broadcasts::tracking_count((conn.into(), path, auth_user)).await.into();
    assert_eq!(response.status(), StatusCode::OK);
    let result: Value = support::unwrap_body_to_object(&response).unwrap();
    assert_eq!(result["event_id"], json!(event_id));
    let b = Broadcast::find(broadcast.id, &connection).unwrap();
    assert_eq!(b.opened_quantity, 1);
}
