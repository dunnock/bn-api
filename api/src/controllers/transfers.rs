use actix_web::{http::StatusCode, HttpResponse, Path, Query, State};
use crate::auth::user::User;
use bigneon_db::models::{User as DbUser, *};
use chrono::prelude::*;
use crate::communications::{mailers, smsers};
use crate::controllers::tickets::transfer_tickets_on_blockchain;
use crate::db::Connection;
use diesel::PgConnection;
use crate::errors::*;
use crate::helpers::application;
use itertools::Itertools;
use crate::models::*;
use crate::server::AppState;

#[derive(Deserialize, Clone)]
pub struct TransferFilters {
    source_or_destination: Option<String>,
    start_utc: Option<NaiveDateTime>,
    end_utc: Option<NaiveDateTime>,
}

pub fn show_by_transfer_key(
    (connection, path): (Connection, Path<PathParameters>),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let transfer = Transfer::find_by_transfer_key(path.id, connection)?;
    // if you have the transfer key, you can view the transfer
    Ok(HttpResponse::Ok().json(&transfer.for_display(connection)?))
}

pub fn index(
    (connection, paging_query, filter_query, path, auth_user): (
        Connection,
        Query<PagingParameters>,
        Query<TransferFilters>,
        Path<OptionalPathParameters>,
        User,
    ),
) -> Result<WebPayload<DisplayTransfer>, BigNeonError> {
    let connection = connection.get();
    let mut lookup_user_id = auth_user.id();

    match path.id {
        Some(order_id) => {
            let order = Order::find(order_id, connection)?;
            lookup_user_id = order.on_behalf_of_user_id.unwrap_or(order.user_id);
            if order.on_behalf_of_user_id != Some(auth_user.id())
                && !(order.on_behalf_of_user_id.is_none() && order.user_id == auth_user.id())
            {
                auth_user.requires_scope_for_order(Scopes::TransferRead, &order, connection)?;
            } else {
                auth_user.requires_scope(Scopes::TransferReadOwn)?;
            }
        }
        None => {
            auth_user.requires_scope(Scopes::TransferReadOwn)?;
        }
    }

    let source_or_destination = match filter_query
        .source_or_destination
        .clone()
        .unwrap_or("source".to_string())
        .as_str()
    {
        "source" => SourceOrDestination::Source,
        _ => SourceOrDestination::Destination,
    };

    let mut payload = Transfer::find_for_user_for_display(
        lookup_user_id,
        path.id,
        source_or_destination,
        filter_query.start_utc,
        filter_query.end_utc,
        Some(paging_query.limit()),
        Some(paging_query.page()),
        connection,
    )?;
    payload
        .paging
        .tags
        .insert("start_utc".to_string(), json!(filter_query.start_utc));
    payload
        .paging
        .tags
        .insert("end_utc".to_string(), json!(filter_query.end_utc));
    payload.paging.tags.insert(
        "source_or_destination".to_string(),
        json!(filter_query.source_or_destination),
    );
    Ok(WebPayload::new(StatusCode::OK, payload))
}

pub fn activity(
    (connection, paging_query, past_or_upcoming_query, auth_user): (
        Connection,
        Query<PagingParameters>,
        Query<PastOrUpcomingParameters>,
        User,
    ),
) -> Result<WebPayload<UserTransferActivitySummary>, BigNeonError> {
    let connection = connection.get();
    auth_user.requires_scope(Scopes::TransferReadOwn)?;

    let payload = auth_user.user.transfer_activity_by_event_tickets(
        paging_query.page(),
        paging_query.limit.unwrap_or(std::u32::MAX),
        paging_query.dir.unwrap_or(SortingDir::Desc),
        past_or_upcoming_query
            .past_or_upcoming
            .unwrap_or(PastOrUpcoming::Upcoming),
        connection,
    )?;

    Ok(WebPayload::new(StatusCode::OK, payload))
}

pub fn cancel(
    (connection, path, auth_user, state): (Connection, Path<PathParameters>, User, State<AppState>),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let transfer = Transfer::find(path.id, connection)?;
    check_transfer_cancel_access(&transfer, &auth_user, connection)?;

    let previous_status = transfer.status;
    let transfer_tickets = transfer.tickets(connection)?;
    let source_user = DbUser::find(transfer.source_user_id, connection)?;

    let transfer = transfer.cancel(&auth_user.user, None, connection)?;

    // Transfer tickets back to the original wallet
    if previous_status != TransferStatus::Pending {
        let source_user_wallet = source_user.default_wallet(connection)?;
        for (destination_user_wallet_id, tickets) in &transfer_tickets
            .into_iter()
            .sorted_by_key(|ti| ti.wallet_id)
            .into_iter()
            .group_by(|ti| ti.wallet_id)
        {
            let destination_user_wallet = Wallet::find(destination_user_wallet_id, connection)?;
            transfer_tickets_on_blockchain(
                &tickets.collect_vec(),
                connection,
                &*state.config.tari_client,
                &destination_user_wallet,
                &source_user_wallet,
            )?;
        }
    }

    if let Some(transfer_message_type) = transfer.transfer_message_type {
        if let Some(transfer_address) = &transfer.transfer_address {
            match transfer_message_type {
                TransferMessageType::Phone => {
                    smsers::tickets::transfer_cancelled(
                        &state.config,
                        transfer_address.clone(),
                        &source_user,
                        connection,
                    )?;
                }
                TransferMessageType::Email => {
                    mailers::tickets::transfer_cancelled(
                        &state.config,
                        transfer_address.clone(),
                        &source_user,
                        &transfer,
                        connection,
                    )?;
                }
            }
        }
    }

    if let Some(source_email) = source_user.email.clone() {
        mailers::tickets::transfer_cancelled_receipt(&state.config, source_email, &source_user, &transfer, connection)?;
    }

    Ok(HttpResponse::Ok().json(&transfer.for_display(connection)?))
}

fn check_transfer_cancel_access(
    transfer: &Transfer,
    user: &User,
    connection: &PgConnection,
) -> Result<(), BigNeonError> {
    if transfer.status == TransferStatus::Completed {
        if !user.has_scope(Scopes::TransferCancelAccepted)? {
            application::forbidden::<HttpResponse>(
                "You do not have access to cancel this transfer as it is completed",
            )?;
        }
    } else if transfer.source_user_id != user.id() {
        let mut valid = true;
        let events = transfer.events(connection)?;
        for event in events {
            let org = event.organization(connection)?;
            valid =
                valid && user.has_scope_for_organization_event(Scopes::TransferCancel, &org, event.id, connection)?;
        }

        if !valid {
            application::forbidden::<HttpResponse>("You do not have access to this transfer")?;
        }
    } else {
        user.requires_scope(Scopes::TransferCancelOwn)?;
    }
    Ok(())
}
