use actix_web::{http::StatusCode, FromRequest, HttpResponse, Path, Query};
use bigneon_api::controllers::notes::{self, *};
use bigneon_api::extractors::*;
use bigneon_api::models::*;
use bigneon_db::models::*;
use bigneon_db::utils::dates;
use diesel;
use diesel::sql_types;
use diesel::RunQueryDsl;
use serde_json::Value;
use std::collections::HashMap;
use crate::support;
use crate::support::database::TestDatabase;
use crate::support::test_request::TestRequest;

pub fn index(role: Roles, filter_deleted_disabled: bool, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let organization = database.create_organization().with_event_fee().with_fees().finish();
    let event = database
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();

    let order = database.create_order().for_event(&event).is_paid().finish();
    let note = database.create_note().for_order(&order).finish();
    let note2 = database.create_note().for_order(&order).finish();

    // order by created_at desc so note2 with an older created_at will appear last
    diesel::sql_query(
        r#"
        UPDATE notes
        SET created_at = $1
        WHERE id = $2;
        "#,
    )
    .bind::<sql_types::Timestamp, _>(dates::now().add_hours(-2).finish())
    .bind::<sql_types::Uuid, _>(note2.id)
    .execute(connection)
    .unwrap();
    note2.destroy(user.id, connection).unwrap();
    let note2 = Note::find(note2.id, connection).unwrap();
    let order2 = database.create_order().is_paid().finish();
    database.create_note().for_order(&order2).finish();
    let mut expected_notes = vec![note.clone()];

    if filter_deleted_disabled {
        expected_notes.push(note2.clone());
    }

    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);

    let test_request = if filter_deleted_disabled {
        TestRequest::create_with_uri_custom_params("/?filter_deleted=false", vec!["main_table", "id"])
    } else {
        TestRequest::create_with_uri_custom_params("/", vec!["main_table", "id"])
    };
    let query_parameters = Query::<PagingParameters>::extract(&test_request.request).unwrap();
    let filter_parameters = Query::<NoteFilterParameters>::extract(&test_request.request).unwrap();
    let mut path = Path::<MainTablePathParameters>::extract(&test_request.request).unwrap();
    path.id = order.id;
    path.main_table = Tables::Orders.to_string();

    let response = notes::index((
        database.connection.clone().into(),
        path,
        query_parameters,
        filter_parameters,
        auth_user,
    ));

    let mut expected_tags: HashMap<String, Value> = HashMap::new();
    expected_tags.insert("filter_deleted".to_string(), json!(!filter_deleted_disabled));
    let wrapped_expected_orgs = Payload {
        data: expected_notes.clone(),
        paging: Paging {
            page: 0,
            limit: 100,
            sort: "".to_string(),
            dir: SortingDir::Asc,
            total: expected_notes.len() as u64,
            tags: expected_tags,
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

pub fn create(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let organization = database.create_organization().with_event_fee().with_fees().finish();
    let event = database
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();

    let order = database.create_order().for_event(&event).is_paid().finish();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);

    let note_text = "Note Example".to_string();

    let json = Json(NewNoteRequest {
        note: note_text.clone(),
    });

    let test_request = TestRequest::create_with_uri_custom_params("/", vec!["main_table", "id"]);
    let mut path = Path::<MainTablePathParameters>::extract(&test_request.request).unwrap();
    path.id = order.id;
    path.main_table = Tables::Orders.to_string();

    let response: HttpResponse = notes::create((database.connection.into(), path, json, auth_user)).into();
    if should_test_succeed {
        let body = support::unwrap_body_to_string(&response).unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let note: Note = serde_json::from_str(&body).unwrap();
        assert_eq!(note.note, note_text);
        assert_eq!(note.main_id, order.id);
        assert_eq!(note.main_table, Tables::Orders);
        assert_eq!(note.created_by, user.id);
    } else {
        support::expects_unauthorized(&response);
    }
}

pub fn destroy(role: Roles, should_succeed: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let organization = database.create_organization().with_event_fee().with_fees().finish();
    let event = database
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();

    let order = database.create_order().for_event(&event).is_paid().finish();
    let note = database.create_note().for_order(&order).finish();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = note.id;

    let response: HttpResponse = notes::destroy((database.connection.clone().into(), path, auth_user)).into();

    if should_succeed {
        assert_eq!(response.status(), StatusCode::OK);
        let note = Note::find(note.id, connection).unwrap();
        assert!(note.deleted_at.is_some());
        assert_eq!(note.deleted_by, Some(user.id));
    } else {
        support::expects_unauthorized(&response);
    }
}
