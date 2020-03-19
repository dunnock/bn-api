use crate::functional::base;
use crate::support;
use crate::support::database::TestDatabase;
use crate::support::test_request::TestRequest;
use actix_web::{
    http::StatusCode,
    web::{Path, Query},
    FromRequest, HttpResponse,
};
use api::controllers::transfers::{self, *};
use api::errors::ApiError;
use api::models::*;
use chrono::prelude::*;
use db::prelude::*;
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

#[cfg(test)]
mod index_tests {
    use super::*;
    #[actix_rt::test]
    async fn index_org_member() {
        base::transfers::index(Roles::OrgMember, false, true).await;
    }
    #[actix_rt::test]
    async fn index_admin() {
        base::transfers::index(Roles::Admin, false, true).await;
    }
    #[actix_rt::test]
    async fn index_user() {
        base::transfers::index(Roles::User, false, false).await;
    }
    #[actix_rt::test]
    async fn index_org_owner() {
        base::transfers::index(Roles::OrgOwner, false, true).await;
    }
    #[actix_rt::test]
    async fn index_door_person() {
        base::transfers::index(Roles::DoorPerson, false, false).await;
    }
    #[actix_rt::test]
    async fn index_promoter() {
        base::transfers::index(Roles::Promoter, false, true).await;
    }
    #[actix_rt::test]
    async fn index_promoter_read_only() {
        base::transfers::index(Roles::PromoterReadOnly, false, true).await;
    }
    #[actix_rt::test]
    async fn index_org_admin() {
        base::transfers::index(Roles::OrgAdmin, false, true).await;
    }
    #[actix_rt::test]
    async fn index_box_office() {
        base::transfers::index(Roles::OrgBoxOffice, true, true).await;
    }
    #[actix_rt::test]
    async fn index_owns_order_org_member() {
        base::transfers::index(Roles::OrgMember, true, true).await;
    }
    #[actix_rt::test]
    async fn index_owns_order_admin() {
        base::transfers::index(Roles::Admin, true, true).await;
    }
    #[actix_rt::test]
    async fn index_owns_order_user() {
        base::transfers::index(Roles::User, true, true).await;
    }
    #[actix_rt::test]
    async fn index_owns_order_org_owner() {
        base::transfers::index(Roles::OrgOwner, true, true).await;
    }
    #[actix_rt::test]
    async fn index_owns_order_door_person() {
        base::transfers::index(Roles::DoorPerson, true, true).await;
    }
    #[actix_rt::test]
    async fn index_owns_order_promoter() {
        base::transfers::index(Roles::Promoter, true, true).await;
    }
    #[actix_rt::test]
    async fn index_owns_order_promoter_read_only() {
        base::transfers::index(Roles::PromoterReadOnly, true, true).await;
    }
    #[actix_rt::test]
    async fn index_owns_order_org_admin() {
        base::transfers::index(Roles::OrgAdmin, true, true).await;
    }
    #[actix_rt::test]
    async fn index_owns_order_box_office() {
        base::transfers::index(Roles::OrgBoxOffice, true, true).await;
    }
}

#[cfg(test)]
mod cancel_completed_transfer_tests {
    use super::*;
    #[actix_rt::test]
    async fn cancel_completed_transfer_org_member() {
        base::transfers::cancel_completed_transfer(Roles::OrgMember, false).await;
    }
    #[actix_rt::test]
    async fn cancel_completed_transfer_admin() {
        base::transfers::cancel_completed_transfer(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn cancel_completed_transfer_super() {
        base::transfers::cancel_completed_transfer(Roles::Super, true).await;
    }
    #[actix_rt::test]
    async fn cancel_completed_transfer_user() {
        base::transfers::cancel_completed_transfer(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn cancel_completed_transfer_org_owner() {
        base::transfers::cancel_completed_transfer(Roles::OrgOwner, false).await;
    }
    #[actix_rt::test]
    async fn cancel_completed_transfer_door_person() {
        base::transfers::cancel_completed_transfer(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn cancel_completed_transfer_promoter() {
        base::transfers::cancel_completed_transfer(Roles::Promoter, false).await;
    }
    #[actix_rt::test]
    async fn cancel_completed_transfer_promoter_read_only() {
        base::transfers::cancel_completed_transfer(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn cancel_completed_transfer_org_admin() {
        base::transfers::cancel_completed_transfer(Roles::OrgAdmin, false).await;
    }
    #[actix_rt::test]
    async fn cancel_completed_transfer_box_office() {
        base::transfers::cancel_completed_transfer(Roles::OrgBoxOffice, false).await;
    }
}

#[cfg(test)]
mod cancel_tests {
    use super::*;
    #[actix_rt::test]
    async fn cancel_org_member() {
        base::transfers::cancel(Roles::OrgMember, false, true).await;
    }
    #[actix_rt::test]
    async fn cancel_admin() {
        base::transfers::cancel(Roles::Admin, false, true).await;
    }
    #[actix_rt::test]
    async fn cancel_user() {
        base::transfers::cancel(Roles::User, false, false).await;
    }
    #[actix_rt::test]
    async fn cancel_org_owner() {
        base::transfers::cancel(Roles::OrgOwner, false, true).await;
    }
    #[actix_rt::test]
    async fn cancel_door_person() {
        base::transfers::cancel(Roles::DoorPerson, false, false).await;
    }
    #[actix_rt::test]
    async fn cancel_promoter() {
        base::transfers::cancel(Roles::Promoter, false, true).await;
    }
    #[actix_rt::test]
    async fn cancel_promoter_read_only() {
        base::transfers::cancel(Roles::PromoterReadOnly, false, false).await;
    }
    #[actix_rt::test]
    async fn cancel_org_admin() {
        base::transfers::cancel(Roles::OrgAdmin, false, true).await;
    }
    #[actix_rt::test]
    async fn cancel_box_office() {
        base::transfers::cancel(Roles::OrgBoxOffice, false, false).await;
    }
    #[actix_rt::test]
    async fn cancel_owns_order_org_member() {
        base::transfers::cancel(Roles::OrgMember, true, true).await;
    }
    #[actix_rt::test]
    async fn cancel_owns_order_admin() {
        base::transfers::cancel(Roles::Admin, true, true).await;
    }
    #[actix_rt::test]
    async fn cancel_owns_order_user() {
        base::transfers::cancel(Roles::User, true, true).await;
    }
    #[actix_rt::test]
    async fn cancel_owns_order_org_owner() {
        base::transfers::cancel(Roles::OrgOwner, true, true).await;
    }
    #[actix_rt::test]
    async fn cancel_owns_order_door_person() {
        base::transfers::cancel(Roles::DoorPerson, true, true).await;
    }
    #[actix_rt::test]
    async fn cancel_owns_order_promoter() {
        base::transfers::cancel(Roles::Promoter, true, true).await;
    }
    #[actix_rt::test]
    async fn cancel_owns_order_promoter_read_only() {
        base::transfers::cancel(Roles::PromoterReadOnly, true, true).await;
    }
    #[actix_rt::test]
    async fn cancel_owns_order_org_admin() {
        base::transfers::cancel(Roles::OrgAdmin, true, true).await;
    }
    #[actix_rt::test]
    async fn cancel_owns_order_box_office() {
        base::transfers::cancel(Roles::OrgBoxOffice, true, true).await;
    }
}

#[actix_rt::test]
pub async fn activity() {
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

    let transfer = TicketInstance::create_transfer(&user, &[ticket.id], None, None, false, connection).unwrap();

    let auth_user = support::create_auth_user_from_user(&user, Roles::User, Some(&organization), &database);
    let test_request = TestRequest::create_with_uri("/transfers/activity?past_or_upcoming=Upcoming");
    let paging_parameters = Query::<PagingParameters>::extract(&test_request.request).await.unwrap();
    let filter_parameters = Query::<PastOrUpcomingParameters>::extract(&test_request.request)
        .await
        .unwrap();
    let response: Result<WebPayload<UserTransferActivitySummary>, ApiError> = transfers::activity((
        database.connection.clone().into(),
        paging_parameters,
        filter_parameters,
        auth_user.clone(),
    ))
    .await;

    let response = response.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let payload = response.payload();
    assert_eq!(payload.data.len(), 1);
    let data = payload.data.first().unwrap();
    assert_eq!(data.event, event.for_display(connection).unwrap());
    assert_eq!(data.ticket_activity_items.len(), 1);
    let activity_items = data.ticket_activity_items.get(&ticket.id).unwrap();
    assert_eq!(activity_items.len(), 1);
    if let ActivityItem::Transfer {
        transfer_id,
        action,
        status,
        ticket_ids,
        ..
    } = activity_items.first().unwrap()
    {
        assert_eq!(*transfer_id, transfer.id);
        assert_eq!(action, &"Started".to_string());
        assert_eq!(*status, TransferStatus::Pending);
        assert_eq!(ticket_ids, &vec![ticket.id]);
    }
}

#[actix_rt::test]
pub async fn show_by_transfer_key() {
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
        .for_user(&user)
        .for_event(&event)
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

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = transfer.transfer_key;
    let response: HttpResponse = transfers::show_by_transfer_key((database.connection.clone().into(), path))
        .await
        .into();

    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    let found_transfer: DisplayTransfer = serde_json::from_str(&body).unwrap();
    assert_eq!(found_transfer.id, transfer.id);
    assert_eq!(found_transfer.status, TransferStatus::Pending);
}

#[actix_rt::test]
async fn index() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let user2 = database.create_user().finish();
    let user3 = database.create_user().finish();
    database.create_order().for_user(&user).quantity(1).is_paid().finish();
    database.create_order().for_user(&user2).quantity(1).is_paid().finish();
    let ticket = &TicketInstance::find_for_user(user.id, connection).unwrap()[0];
    let ticket2 = &TicketInstance::find_for_user(user2.id, connection).unwrap()[0];

    let transfer = Transfer::create(user.id, Uuid::new_v4(), None, None, false)
        .commit(connection)
        .unwrap();
    transfer.add_transfer_ticket(ticket.id, connection).unwrap();
    transfer.update_associated_orders(connection).unwrap();
    transfer
        .update(
            TransferEditableAttributes {
                destination_user_id: Some(user3.id),
                ..Default::default()
            },
            connection,
        )
        .unwrap();
    let transfer2 = Transfer::create(user2.id, Uuid::new_v4(), None, None, false)
        .commit(connection)
        .unwrap();
    transfer2.add_transfer_ticket(ticket2.id, connection).unwrap();
    transfer2.update_associated_orders(connection).unwrap();
    transfer2
        .update(
            TransferEditableAttributes {
                destination_user_id: Some(user.id),
                ..Default::default()
            },
            connection,
        )
        .unwrap();

    // Outgoing
    let expected_transfers = Transfer::find_for_user_for_display(
        user.id,
        None,
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

    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);
    let test_request = TestRequest::create_with_uri("/transfers?source_or_destination=source");
    let paging_parameters = Query::<PagingParameters>::extract(&test_request.request).await.unwrap();
    let filter_parameters = Query::<TransferFilters>::extract(&test_request.request).await.unwrap();
    let mut path = Path::<OptionalPathParameters>::extract(&test_request.request)
        .await
        .unwrap();
    path.id = None;
    let response: Result<WebPayload<DisplayTransfer>, ApiError> = transfers::index((
        database.connection.clone().into(),
        paging_parameters,
        filter_parameters,
        path,
        auth_user.clone(),
    ))
    .await;

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

    // Incoming
    let expected_transfers = Transfer::find_for_user_for_display(
        user.id,
        None,
        SourceOrDestination::Destination,
        None,
        None,
        None,
        None,
        connection,
    )
    .unwrap()
    .data;
    assert_eq!(expected_transfers.len(), 1);
    assert_eq!(expected_transfers[0].ticket_ids, vec![ticket2.id]);

    let test_request = TestRequest::create_with_uri("/transfers?source_or_destination=destination");
    let paging_parameters = Query::<PagingParameters>::extract(&test_request.request).await.unwrap();
    let filter_parameters = Query::<TransferFilters>::extract(&test_request.request).await.unwrap();
    let mut path = Path::<OptionalPathParameters>::extract(&test_request.request)
        .await
        .unwrap();
    path.id = None;
    let response: Result<WebPayload<DisplayTransfer>, ApiError> = transfers::index((
        database.connection.clone().into(),
        paging_parameters,
        filter_parameters,
        path,
        auth_user,
    ))
    .await;

    let response = response.unwrap();
    let mut expected_tags: HashMap<String, Value> = HashMap::new();
    let no_datetime: Option<NaiveDateTime> = None;
    expected_tags.insert("source_or_destination".to_string(), json!("destination"));
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
}
