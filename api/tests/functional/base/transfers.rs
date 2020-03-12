use crate::support;
use crate::support::database::TestDatabase;
use crate::support::test_request::TestRequest;
use actix_web::{FromRequest, http::StatusCode, HttpResponse, web::{Path, Query}};
use bigneon_api::controllers::transfers::{self, *};
use bigneon_api::errors::BigNeonError;
use bigneon_api::models::*;
use bigneon_db::prelude::*;
use chrono::prelude::*;
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

pub async fn index(role: Roles, owns_order: bool, should_succeed: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let user2 = database.create_user().finish();
    let organization = database.create_organization().finish();
    let event = database
        .create_event()
        .with_tickets()
        .with_ticket_pricing()
        .with_organization(&organization)
        .finish();
    let order = database
        .create_order()
        .for_event(&event)
        .for_user(&user)
        .quantity(1)
        .is_paid()
        .finish();
    let order2 = database
        .create_order()
        .for_event(&event)
        .for_user(&user)
        .quantity(1)
        .is_paid()
        .finish();

    let ticket = TicketInstance::find(
        TicketInstance::find_ids_for_order(order.id, connection)
            .unwrap()
            .pop()
            .unwrap(),
        connection,
    )
    .unwrap();
    let ticket2 = TicketInstance::find(
        TicketInstance::find_ids_for_order(order2.id, connection)
            .unwrap()
            .pop()
            .unwrap(),
        connection,
    )
    .unwrap();

    let transfer = Transfer::create(user.id, Uuid::new_v4(), None, None, false)
        .commit(connection)
        .unwrap();
    transfer.add_transfer_ticket(ticket.id, connection).unwrap();
    transfer.update_associated_orders(connection).unwrap();
    transfer
        .update(
            TransferEditableAttributes {
                destination_user_id: Some(user2.id),
                ..Default::default()
            },
            connection,
        )
        .unwrap();
    let transfer2 = Transfer::create(user.id, Uuid::new_v4(), None, None, false)
        .commit(connection)
        .unwrap();
    transfer2.add_transfer_ticket(ticket2.id, connection).unwrap();
    transfer2.update_associated_orders(connection).unwrap();

    let expected_transfers = Transfer::find_for_user_for_display(
        user.id,
        Some(order.id),
        SourceOrDestination::Source,
        None,
        None,
        None,
        None,
        connection,
    )
    .unwrap()
    .data;
    assert_eq!(expected_transfers.len(), 1);
    assert_eq!(expected_transfers[0].ticket_ids, vec![ticket.id]);

    let auth_user = if owns_order {
        support::create_auth_user_from_user(&user, role, Some(&organization), &database)
    } else {
        support::create_auth_user(role, Some(&organization), &database)
    };
    let test_request = TestRequest::create_with_uri("/transfers?source_or_destination=source");
    let paging_parameters = Query::<PagingParameters>::extract(&test_request.request).await.unwrap();
    let filter_parameters = Query::<TransferFilters>::extract(&test_request.request).await.unwrap();
    let mut path = Path::<OptionalPathParameters>::extract(&test_request.request).await.unwrap();
    path.id = Some(order.id);
    let response: Result<WebPayload<DisplayTransfer>, BigNeonError> = transfers::index((
        database.connection.clone().into(),
        paging_parameters,
        filter_parameters,
        path,
        auth_user.clone(),
    ));

    if should_succeed {
        let response = response.unwrap();
        let mut expected_tags: HashMap<String, Value> = HashMap::new();
        let no_datetime: Option<NaiveDateTime> = None;
        expected_tags.insert("source_or_destination".to_string(), json!("source"));
        expected_tags.insert("start_utc".to_string(), json!(no_datetime));
        expected_tags.insert("end_utc".to_string(), json!(no_datetime));
        let wrapped_expected_transfers = Payload {
            data: expected_transfers,
            paging: Paging {
                page: 0,
                limit: 100,
                sort: "".to_string(),
                dir: SortingDir::Asc,
                total: 1,
                tags: expected_tags,
            },
        };

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.payload(), &wrapped_expected_transfers);
    } else {
        support::expects_unauthorized(&response.unwrap_err().into_inner().to_response());
    }
}

pub async fn cancel_completed_transfer(role: Roles, should_succeed: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let user2 = database.create_user().finish();
    let organization = database.create_organization().finish();
    let event = database
        .create_event()
        .with_tickets()
        .with_ticket_pricing()
        .with_organization(&organization)
        .finish();
    let order = database
        .create_order()
        .for_event(&event)
        .for_user(&user)
        .quantity(1)
        .is_paid()
        .finish();

    let ticket = TicketInstance::find(
        TicketInstance::find_ids_for_order(order.id, connection)
            .unwrap()
            .pop()
            .unwrap(),
        connection,
    )
    .unwrap();

    let transfer = TicketInstance::direct_transfer(
        &user,
        &vec![ticket.id],
        "nowhere",
        TransferMessageType::Email,
        user2.id,
        connection,
    )
    .unwrap();
    let wallet = Wallet::find_default_for_user(transfer.source_user_id, connection).unwrap();
    for ticket in transfer.tickets(connection).unwrap() {
        assert_ne!(ticket.wallet_id, wallet.id);
    }

    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);
    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = transfer.id;
    let response: HttpResponse = transfers::cancel((
        database.connection.clone().into(),
        path,
        auth_user.clone(),
        test_request.extract_state(),
    ))
    .into();

    if should_succeed {
        assert_eq!(response.status(), StatusCode::OK);
        let body = support::unwrap_body_to_string(&response).unwrap();
        let found_transfer: DisplayTransfer = serde_json::from_str(&body).unwrap();
        assert_eq!(found_transfer.id, transfer.id);
        assert_eq!(found_transfer.status, TransferStatus::Cancelled);

        for ticket in transfer.tickets(connection).unwrap() {
            assert_eq!(ticket.wallet_id, wallet.id);
        }
    } else {
        support::expects_forbidden(
            &response,
            Some("You do not have access to cancel this transfer as it is completed"),
        );

        for ticket in transfer.tickets(connection).unwrap() {
            assert_ne!(ticket.wallet_id, wallet.id);
        }
    }
}

pub async fn cancel(role: Roles, owns_order: bool, should_succeed: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let event = database
        .create_event()
        .with_tickets()
        .with_ticket_pricing()
        .with_organization(&organization)
        .finish();
    let order = database
        .create_order()
        .for_event(&event)
        .for_user(&user)
        .quantity(1)
        .is_paid()
        .finish();

    let ticket = TicketInstance::find(
        TicketInstance::find_ids_for_order(order.id, connection)
            .unwrap()
            .pop()
            .unwrap(),
        connection,
    )
    .unwrap();

    let transfer = Transfer::create(user.id, Uuid::new_v4(), None, None, false)
        .commit(connection)
        .unwrap();
    transfer.add_transfer_ticket(ticket.id, connection).unwrap();
    transfer.update_associated_orders(connection).unwrap();

    let auth_user = if owns_order {
        support::create_auth_user_from_user(&user, role, Some(&organization), &database)
    } else {
        support::create_auth_user(role, Some(&organization), &database)
    };
    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = transfer.id;
    let response: HttpResponse = transfers::cancel((
        database.connection.clone().into(),
        path,
        auth_user.clone(),
        test_request.extract_state(),
    ))
    .into();

    if should_succeed {
        assert_eq!(response.status(), StatusCode::OK);
        let body = support::unwrap_body_to_string(&response).unwrap();
        let found_transfer: DisplayTransfer = serde_json::from_str(&body).unwrap();
        assert_eq!(found_transfer.id, transfer.id);
        assert_eq!(found_transfer.status, TransferStatus::Cancelled);
    } else {
        support::expects_forbidden(&response, Some("You do not have access to this transfer"));
    }
}
