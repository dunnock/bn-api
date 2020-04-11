use chrono::prelude::*;
use diesel;
use diesel::dsl::{count, exists, select, sql};
use diesel::expression::dsl;
use diesel::prelude::*;
use diesel::sql_types::{Array, Bool, Text, Uuid as dUuid};
use models::*;
use schema::{
    assets, events, order_transfers, orders, organizations, ticket_instances, ticket_types, transfer_tickets, transfers,
};
use serde_json::Value;
use std::cmp::Ordering;
use tari_client::*;
use utils::errors::ConvertToDatabaseError;
use utils::errors::DatabaseError;
use utils::errors::ErrorCode;
use utils::pagination::Paginate;
use uuid::Uuid;
use validator::*;
use validators::{self, *};

pub static TRANSFER_DRIP_NOTIFICATION_DAYS_PRIOR_TO_EVENT: &'static [i64] = &[7, 1, 0];
pub const TRANSFER_DRIP_NOTIFICATION_HOURS_PRIOR_TO_EVENT: i64 = 3;

#[derive(Insertable, Serialize, Deserialize, PartialEq, Debug)]
#[table_name = "transfers"]
pub struct NewTransfer {
    pub source_user_id: Uuid,
    pub transfer_key: Uuid,
    pub status: TransferStatus,
    pub transfer_message_type: Option<TransferMessageType>,
    pub transfer_address: Option<String>,
    pub direct: bool,
}

#[derive(Clone, Queryable, Identifiable, Insertable, Serialize, Deserialize, PartialEq, Debug)]
#[table_name = "transfers"]
pub struct Transfer {
    pub id: Uuid,
    pub source_user_id: Uuid,
    pub destination_user_id: Option<Uuid>,
    pub transfer_key: Uuid,
    pub status: TransferStatus,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub transfer_message_type: Option<TransferMessageType>,
    pub transfer_address: Option<String>,
    pub cancelled_by_user_id: Option<Uuid>,
    pub direct: bool,
    pub destination_temporary_user_id: Option<Uuid>,
}

#[derive(AsChangeset, Default, Deserialize)]
#[table_name = "transfers"]
pub struct TransferEditableAttributes {
    pub status: Option<TransferStatus>,
    pub destination_user_id: Option<Uuid>,
    pub cancelled_by_user_id: Option<Uuid>,
}

#[derive(Clone, Queryable, Deserialize, Serialize, PartialEq, Debug)]
pub struct DisplayTransfer {
    pub id: Uuid,
    pub source_user_id: Uuid,
    pub destination_user_id: Option<Uuid>,
    pub transfer_key: Uuid,
    pub status: TransferStatus,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub transfer_message_type: Option<TransferMessageType>,
    pub transfer_address: Option<String>,
    pub ticket_ids: Vec<Uuid>,
    pub event_ids: Vec<Uuid>,
    pub direct: bool,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct ProcessTransferDripPayload {
    pub source_or_destination: SourceOrDestination,
    pub event_id: Uuid,
}

impl PartialOrd for Transfer {
    fn partial_cmp(&self, other: &Transfer) -> Option<Ordering> {
        Some(self.id.cmp(&other.id))
    }
}

impl Transfer {
    pub fn temporary_user_id(transfer_address: Option<String>) -> Option<Uuid> {
        transfer_address.map(|a| Uuid::new_v3(&Uuid::nil(), &a))
    }

    pub fn drip_header(
        &self,
        event: &Event,
        source_or_destination: SourceOrDestination,
        include_links: bool,
        environment: Environment,
        conn: &PgConnection,
    ) -> Result<String, DatabaseError> {
        if self.transfer_address.is_none() {
            return DatabaseError::business_process_error(
                "Cannot build drip header for transfer missing destination address",
            );
        }

        let units_until_event = if environment == Environment::Staging {
            event.minutes_until_event()
        } else {
            event.days_until_event()
        };
        Ok(match units_until_event {
            Some(units_until_event) => match source_or_destination {
                SourceOrDestination::Source => {
                    let mut destination_address = self.transfer_address.clone().unwrap();
                    if include_links && self.transfer_message_type == Some(TransferMessageType::Email) {
                        destination_address =
                            format!("<a href='mailto:{}'>{}</a>", destination_address, destination_address);
                    }
                    match units_until_event {
                            0 => {
                                let (is_pm, hour) = event.get_all_localized_times(event.venue(conn)?.as_ref()).event_start.unwrap().hour12();
                                let time_of_day_text = (if !is_pm || hour < 5 { "today" } else { "tonight" }).to_string();
                                format!("Time to take action! The show is {} and those tickets you sent to {} still haven't been claimed. Give them a nudge!", time_of_day_text, destination_address)
                            },
                            1 => format!("Uh oh! The show is tomorrow and those tickets you sent to {} still haven't been claimed. Give them a nudge!", destination_address),
                            _ => format!("Those tickets you sent to {} still haven't been claimed. Give them a nudge!", destination_address),
                        }
                }
                SourceOrDestination::Destination => {
                    let source_user = User::find(self.source_user_id, conn)?;
                    let mut name = Transfer::sender_name(&source_user);

                    if include_links && source_user.email.is_some() {
                        name = format!("<a href='mailto:{}'>{}</a>", source_user.email.unwrap(), name);
                    }
                    match units_until_event {
                            0 => {
                                let (is_pm, hour) = event.get_all_localized_times(event.venue(conn)?.as_ref()).event_start.unwrap().hour12();
                                let time_of_day_text = (if !is_pm || hour < 5 { "today" } else { "tonight" }).to_string();
                                format!("Time to take action! The event is {} and the tickets {} sent you are still waiting!", time_of_day_text, name)
                            },
                            1 => format!("Get your tickets! The event is TOMORROW and you still need to get the tickets that {} sent you!", name),
                            7 => format!("The event is only one week away and you still need to get the tickets that {} sent you!", name),
                            _ => format!("You still need to get the tickets that {} sent you!", name)
                        }
                }
            },
            None => "".to_string(),
        })
    }

    pub fn receive_url(&self, front_end_url: &str, conn: &PgConnection) -> Result<String, DatabaseError> {
        Ok(format!(
            "{}/tickets/transfers/receive?sender_user_id={}&transfer_key={}&num_tickets={}&signature={}",
            front_end_url,
            self.source_user_id,
            self.transfer_key,
            self.transfer_tickets(conn)?.len(),
            self.signature(conn)?
        )
        .to_string())
    }

    pub fn into_authorization(&self, conn: &PgConnection) -> Result<TransferAuthorization, DatabaseError> {
        Ok(TransferAuthorization {
            transfer_key: self.transfer_key,
            sender_user_id: self.source_user_id,
            num_tickets: self.transfer_tickets(conn)?.len() as u32,
            signature: self.signature(conn)?,
        })
    }

    pub fn sender_name(user: &User) -> String {
        if let (Some(first_name), Some(last_name)) = (user.first_name.clone(), user.last_name.clone()) {
            vec![
                first_name,
                last_name
                    .chars()
                    .next()
                    .map(|c| format!("{}.", c))
                    .unwrap_or("".to_string()),
            ]
            .join(" ")
        } else {
            "another user".to_string()
        }
    }

    pub fn log_drip_domain_event(
        &self,
        source_or_destination: SourceOrDestination,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        let domain_event_type = match source_or_destination {
            SourceOrDestination::Source => DomainEventTypes::TransferTicketDripSourceSent,
            SourceOrDestination::Destination => DomainEventTypes::TransferTicketDripDestinationSent,
        };

        DomainEvent::create(
            domain_event_type,
            "Transfer drip sent".to_string(),
            Tables::Transfers,
            Some(self.id),
            None,
            None,
        )
        .commit(conn)?;

        Ok(())
    }

    pub fn transfer_ticket_count(&self, conn: &PgConnection) -> Result<i64, DatabaseError> {
        transfer_tickets::table
            .filter(transfer_tickets::transfer_id.eq(self.id))
            .select(count(transfer_tickets::id))
            .first(conn)
            .to_db_error(ErrorCode::QueryError, "Could not check transfer ticket count")
    }

    pub fn signature(&self, conn: &PgConnection) -> Result<String, DatabaseError> {
        let mut message: String = self.transfer_key.to_string();
        message.push_str(self.source_user_id.to_string().as_str());
        message.push_str(self.transfer_ticket_count(conn)?.to_string().as_str());
        let secret_key = Wallet::find_default_for_user(self.source_user_id, conn)?.secret_key;
        Ok(convert_bytes_to_hexstring(&cryptographic_signature(
            &message,
            &convert_hexstring_to_bytes(&secret_key),
        )?))
    }

    pub fn create_drip_actions(&self, event: &Event, conn: &PgConnection) -> Result<(), DatabaseError> {
        for source_or_destination in vec![SourceOrDestination::Destination, SourceOrDestination::Source] {
            DomainAction::create(
                None,
                DomainActionTypes::ProcessTransferDrip,
                None,
                json!(ProcessTransferDripPayload {
                    event_id: event.id,
                    source_or_destination,
                }),
                Some(Tables::Transfers),
                Some(self.id),
            )
            .commit(conn)?;
        }
        Ok(())
    }

    pub fn can_process_drips(&self, conn: &PgConnection) -> Result<bool, DatabaseError> {
        Ok(self.status == TransferStatus::Pending
            && self.events_have_not_ended(conn)?
            && self.transfer_address.is_some())
    }

    pub fn events_have_not_ended(&self, conn: &PgConnection) -> Result<bool, DatabaseError> {
        select(exists(
            transfer_tickets::table
                .inner_join(ticket_instances::table.on(ticket_instances::id.eq(transfer_tickets::ticket_instance_id)))
                .inner_join(assets::table.on(assets::id.eq(ticket_instances::asset_id)))
                .inner_join(ticket_types::table.on(ticket_types::id.eq(assets::ticket_type_id)))
                .inner_join(events::table.on(events::id.eq(ticket_types::event_id)))
                .filter(transfer_tickets::transfer_id.eq(self.id))
                .filter(events::event_end.gt(dsl::now.nullable())),
        ))
        .get_result(conn)
        .to_db_error(ErrorCode::QueryError, "Could not confirm if transfer has active events")
    }

    pub fn event_ended(&self, conn: &PgConnection) -> Result<bool, DatabaseError> {
        Ok(self
            .events(conn)?
            .iter()
            .find(|e| {
                e.event_end
                    .unwrap_or_else(|| NaiveDate::from_ymd(3970, 1, 1).and_hms(0, 0, 0))
                    < Utc::now().naive_utc()
            })
            .is_some())
    }

    pub fn find_by_transfer_key(transfer_key: Uuid, conn: &PgConnection) -> Result<Transfer, DatabaseError> {
        let mut transfer: Transfer = transfers::table
            .filter(transfers::transfer_key.eq(transfer_key))
            .select(transfers::all_columns)
            .distinct()
            .get_result(conn)
            .to_db_error(ErrorCode::QueryError, "Error loading transfers")?;

        if transfer.event_ended(conn)? && transfer.status == TransferStatus::Pending {
            transfer.status = TransferStatus::EventEnded;
        }
        Ok(transfer)
    }

    pub fn find(id: Uuid, conn: &PgConnection) -> Result<Transfer, DatabaseError> {
        let mut transfer: Transfer = transfers::table
            .filter(transfers::id.eq(id))
            .first(conn)
            .to_db_error(ErrorCode::QueryError, "Could not find transfer")?;

        if transfer.event_ended(conn)? && transfer.status == TransferStatus::Pending {
            transfer.status = TransferStatus::EventEnded;
        }
        Ok(transfer)
    }

    pub fn find_for_user_for_display(
        user_id: Uuid,
        order_id: Option<Uuid>,
        source_or_destination: SourceOrDestination,
        start_time: Option<NaiveDateTime>,
        end_time: Option<NaiveDateTime>,
        limit: Option<u32>,
        page: Option<u32>,
        conn: &PgConnection,
    ) -> Result<Payload<DisplayTransfer>, DatabaseError> {
        let limit = limit.unwrap_or(100);
        let page = page.unwrap_or(0);

        let mut query = transfers::table
            .inner_join(transfer_tickets::table.on(transfer_tickets::transfer_id.eq(transfers::id)))
            .inner_join(ticket_instances::table.on(transfer_tickets::ticket_instance_id.eq(ticket_instances::id)))
            .inner_join(assets::table.on(assets::id.eq(ticket_instances::asset_id)))
            .inner_join(ticket_types::table.on(assets::ticket_type_id.eq(ticket_types::id)))
            .inner_join(events::table.on(events::id.eq(ticket_types::event_id)))
            .left_join(order_transfers::table.on(order_transfers::transfer_id.eq(transfers::id)))
            .order_by(transfers::created_at.desc())
            .into_boxed();

        match source_or_destination {
            SourceOrDestination::Source => {
                query = query.filter(transfers::source_user_id.eq(user_id));
            }
            SourceOrDestination::Destination => {
                query = query.filter(transfers::destination_user_id.eq(Some(user_id)));
            }
        }

        if let Some(order_id) = order_id {
            query = query.filter(order_transfers::order_id.eq(order_id));
        }

        if let Some(start_time) = start_time {
            query = query.filter(transfers::created_at.ge(start_time));
        }

        if let Some(end_time) = end_time {
            query = query.filter(transfers::created_at.le(end_time));
        }

        let (transfers, transfer_count): (Vec<DisplayTransfer>, i64) = query
            .select(transfers::all_columns)
            .select((
                transfers::id,
                transfers::source_user_id,
                transfers::destination_user_id,
                transfers::transfer_key,
                sql::<Text>(
                    "
                    CASE
                    WHEN transfers.status = 'Pending' THEN
                    COALESCE(MAX('EventEnded') FILTER(WHERE events.event_end < now()), 'Pending')
                    ELSE transfers.status END as status
                ",
                ),
                transfers::created_at,
                transfers::updated_at,
                transfers::transfer_message_type,
                transfers::transfer_address,
                sql::<Array<dUuid>>(
                    "
                    ARRAY_AGG(DISTINCT transfer_tickets.ticket_instance_id)
                ",
                ),
                sql::<Array<dUuid>>(
                    "
                    ARRAY_AGG(DISTINCT events.id)
                ",
                ),
                transfers::direct,
            ))
            .group_by((
                transfers::id,
                transfers::source_user_id,
                transfers::destination_user_id,
                transfers::transfer_key,
                transfers::created_at,
                transfers::updated_at,
                transfers::transfer_message_type,
                transfers::transfer_address,
                transfers::direct,
            ))
            .paginate(page as i64)
            .per_page(limit as i64)
            .load_and_count_pages(conn)
            .to_db_error(ErrorCode::QueryError, "Unable to load transfers")?;

        let mut payload = Payload::new(transfers, Paging::new(page, limit));
        payload.paging.total = transfer_count as u64;
        Ok(payload)
    }

    pub fn for_display(&self, conn: &PgConnection) -> Result<DisplayTransfer, DatabaseError> {
        let ticket_ids: Vec<Uuid> = self
            .transfer_tickets(conn)?
            .iter()
            .map(|tt| tt.ticket_instance_id)
            .collect();
        let event_ids = self.events(conn)?.iter().map(|e| e.id).collect();

        Ok(DisplayTransfer {
            id: self.id,
            source_user_id: self.source_user_id,
            destination_user_id: self.destination_user_id,
            transfer_key: self.transfer_key,
            status: self.status,
            created_at: self.created_at,
            updated_at: self.updated_at,
            transfer_message_type: self.transfer_message_type,
            transfer_address: self.transfer_address.clone(),
            ticket_ids,
            event_ids,
            direct: self.direct,
        })
    }

    pub fn find_pending_by_ticket_instance_ids(
        ticket_instance_ids: &[Uuid],
        conn: &PgConnection,
    ) -> Result<Vec<Transfer>, DatabaseError> {
        transfers::table
            .inner_join(transfer_tickets::table.on(transfer_tickets::transfer_id.eq(transfers::id)))
            .inner_join(ticket_instances::table.on(transfer_tickets::ticket_instance_id.eq(ticket_instances::id)))
            .inner_join(assets::table.on(assets::id.eq(ticket_instances::asset_id)))
            .inner_join(ticket_types::table.on(assets::ticket_type_id.eq(ticket_types::id)))
            .inner_join(events::table.on(events::id.eq(ticket_types::event_id)))
            .filter(transfer_tickets::ticket_instance_id.eq_any(ticket_instance_ids))
            .filter(transfers::status.eq(TransferStatus::Pending))
            .filter(events::event_end.gt(Some(Utc::now().naive_utc())))
            .select(transfers::all_columns)
            .distinct()
            .load(conn)
            .to_db_error(ErrorCode::QueryError, "Error loading transfers")
    }

    pub fn tickets(&self, conn: &PgConnection) -> Result<Vec<TicketInstance>, DatabaseError> {
        let transfer_tickets = self.transfer_tickets(conn)?;
        let ticket_ids: Vec<Uuid> = transfer_tickets.iter().map(|tt| tt.ticket_instance_id).collect();
        TicketInstance::find_by_ids(&ticket_ids, conn)
    }

    pub fn transfer_tickets(&self, conn: &PgConnection) -> Result<Vec<TransferTicket>, DatabaseError> {
        transfer_tickets::table
            .filter(transfer_tickets::transfer_id.eq(self.id))
            .load(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load transfer tickets")
    }

    pub fn cancel_by_ticket_instance_ids(
        ticket_instance_ids: &[Uuid],
        user: &User,
        new_transfer_key: Option<Uuid>,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        let transfers: Vec<Transfer> = transfers::table
            .inner_join(transfer_tickets::table.on(transfer_tickets::transfer_id.eq(transfers::id)))
            .filter(transfer_tickets::ticket_instance_id.eq_any(ticket_instance_ids))
            .filter(transfers::status.eq(TransferStatus::Pending))
            .select(transfers::all_columns)
            .distinct()
            .load(conn)
            .to_db_error(ErrorCode::QueryError, "Error loading transfers")?;

        for transfer in transfers {
            transfer.cancel(&user, new_transfer_key, conn)?;
        }

        Ok(())
    }

    pub fn regenerate_redeem_keys(&self, conn: &PgConnection) -> Result<(), DatabaseError> {
        for ticket in self.tickets(conn)? {
            ticket.associate_redeem_key(conn)?;
        }

        Ok(())
    }

    pub fn was_retransferred(&self, conn: &PgConnection) -> Result<bool, DatabaseError> {
        let query = r#"
            SELECT EXISTS (
                SELECT 1
                FROM transfer_tickets tt
                JOIN transfers t ON tt.transfer_id = t.id
                JOIN transfer_tickets tt2 ON tt2.ticket_instance_id = tt.ticket_instance_id AND tt.id <> tt2.id
                JOIN transfers t2 ON tt2.transfer_id = t2.id
                WHERE tt.transfer_id = $1
                AND t2.created_at >= t.created_at
                AND t2.status IN ('Pending', 'Completed')
            );
        "#;

        #[derive(QueryableByName)]
        struct R {
            #[sql_type = "Bool"]
            exists: bool,
        }

        Ok(diesel::sql_query(query)
            .bind::<dUuid, _>(self.id)
            .get_results::<R>(conn)
            .to_db_error(
                ErrorCode::QueryError,
                "Could not check if accepted transfer eligible for cancelling",
            )?
            .pop()
            .map(|r| r.exists)
            .unwrap_or(false))
    }

    pub fn contains_redeemed_tickets(&self, conn: &PgConnection) -> Result<bool, DatabaseError> {
        Ok(self
            .tickets(conn)?
            .iter()
            .map(|t| t.redeemed_at.is_some())
            .any(|r| r == true))
    }

    pub fn cancel(
        &self,
        user: &User,
        new_transfer_key: Option<Uuid>,
        conn: &PgConnection,
    ) -> Result<Transfer, DatabaseError> {
        let has_cancel_accepted_permission = (self.status == TransferStatus::Pending
            || self.status == TransferStatus::EventEnded
            || self.status == TransferStatus::Completed)
            && user.get_global_scopes().contains(&Scopes::TransferCancelAccepted);
        if !has_cancel_accepted_permission && self.status != TransferStatus::Pending {
            return DatabaseError::business_process_error("Transfer cannot be cancelled as it is no longer pending");
        } else if has_cancel_accepted_permission && self.was_retransferred(conn)? {
            return DatabaseError::business_process_error(
                "Transfer cannot be cancelled as it contains tickets involved in a transfer to another user from the destination user",
            );
        } else if self.contains_redeemed_tickets(conn)? {
            return DatabaseError::business_process_error(
                "Transfer cannot be cancelled as it contains tickets that have been redeemed",
            );
        }
        let previous_status = self.status;
        let transfer = self.update(
            TransferEditableAttributes {
                status: Some(TransferStatus::Cancelled),
                cancelled_by_user_id: Some(user.id),
                ..Default::default()
            },
            conn,
        )?;

        // If transfer was cancelled prior to acceptance we do not update redeem key as it never leaked to recipient
        if self.status != TransferStatus::Pending {
            self.regenerate_redeem_keys(conn)?;
            let wallet = Wallet::find_default_for_user(self.source_user_id, conn)?;
            let name_override: Option<String> = None;

            let mut update_count = 0;
            let tickets = self.tickets(conn)?;
            for ticket in &tickets {
                update_count += diesel::update(
                    ticket_instances::table
                        .filter(ticket_instances::id.eq(ticket.id))
                        .filter(ticket_instances::updated_at.eq(ticket.updated_at)),
                )
                .set((
                    ticket_instances::wallet_id.eq(wallet.id),
                    ticket_instances::updated_at.eq(dsl::now),
                    ticket_instances::first_name_override.eq(&name_override),
                    ticket_instances::last_name_override.eq(&name_override),
                ))
                .execute(conn)
                .to_db_error(ErrorCode::UpdateError, "Could not update ticket instance")?;
            }

            if update_count != tickets.len() {
                return Err(DatabaseError::new(
                    ErrorCode::UpdateError,
                    Some("Could not update ticket instances".to_string()),
                ));
            }
        }

        DomainEvent::create(
            DomainEventTypes::TransferTicketCancelled,
            "Ticket transfer was cancelled".to_string(),
            Tables::Transfers,
            Some(self.id),
            Some(user.id),
            Some(json!({"old_transfer_key": self.transfer_key, "new_transfer_key": &new_transfer_key, "previous_status": previous_status })),
        )
        .commit(conn)?;

        Ok(transfer)
    }

    pub fn complete(
        &self,
        destination_user_id: Uuid,
        additional_data: Option<Value>,
        conn: &PgConnection,
    ) -> Result<Transfer, DatabaseError> {
        if self.status != TransferStatus::Pending {
            return DatabaseError::business_process_error("Transfer cannot be completed as it is no longer pending");
        }

        let transfer = self.update(
            TransferEditableAttributes {
                status: Some(TransferStatus::Completed),
                destination_user_id: Some(destination_user_id),
                ..Default::default()
            },
            conn,
        )?;

        self.regenerate_redeem_keys(conn)?;

        DomainEvent::create(
            DomainEventTypes::TransferTicketCompleted,
            "Transfer ticket completed".to_string(),
            Tables::Transfers,
            Some(self.id),
            None,
            additional_data.clone(),
        )
        .commit(conn)?;

        User::find(self.source_user_id, conn)?.update_genre_info(conn)?;
        User::find(destination_user_id, conn)?.update_genre_info(conn)?;

        Ok(transfer)
    }

    fn transfer_key_unique(
        transfer_key: Uuid,
        conn: &PgConnection,
    ) -> Result<Result<(), ValidationError>, DatabaseError> {
        let transfer_key_in_use = select(exists(
            transfers::table.filter(transfers::transfer_key.eq(transfer_key)),
        ))
        .get_result(conn)
        .to_db_error(
            ErrorCode::QueryError,
            "Could not check if transfer transfer_key is unique",
        )?;

        if transfer_key_in_use {
            let validation_error = create_validation_error("uniqueness", "Transfer key is already in use");
            return Ok(Err(validation_error));
        }

        Ok(Ok(()))
    }

    pub fn add_transfer_ticket(
        &self,
        ticket_instance_id: Uuid,
        conn: &PgConnection,
    ) -> Result<TransferTicket, DatabaseError> {
        TransferTicket::create(ticket_instance_id, self.id).commit(conn)
    }

    pub fn find_pending(conn: &PgConnection) -> Result<Vec<Transfer>, DatabaseError> {
        transfers::table
            .inner_join(transfer_tickets::table.on(transfer_tickets::transfer_id.eq(transfers::id)))
            .inner_join(ticket_instances::table.on(transfer_tickets::ticket_instance_id.eq(ticket_instances::id)))
            .inner_join(assets::table.on(assets::id.eq(ticket_instances::asset_id)))
            .inner_join(ticket_types::table.on(assets::ticket_type_id.eq(ticket_types::id)))
            .inner_join(events::table.on(events::id.eq(ticket_types::event_id)))
            .filter(transfers::status.eq(TransferStatus::Pending))
            .filter(events::event_end.gt(Some(Utc::now().naive_utc())))
            .select(transfers::all_columns)
            .distinct()
            .load(conn)
            .to_db_error(ErrorCode::QueryError, "Error loading transfers")
    }

    pub fn create(
        source_user_id: Uuid,
        transfer_key: Uuid,
        transfer_message_type: Option<TransferMessageType>,
        transfer_address: Option<String>,
        direct: bool,
    ) -> NewTransfer {
        NewTransfer {
            transfer_address,
            transfer_message_type,
            source_user_id,
            transfer_key,
            direct,
            status: TransferStatus::Pending,
        }
    }

    pub fn organizations(&self, conn: &PgConnection) -> Result<Vec<Organization>, DatabaseError> {
        transfer_tickets::table
            .inner_join(ticket_instances::table.on(ticket_instances::id.eq(transfer_tickets::ticket_instance_id)))
            .inner_join(assets::table.on(assets::id.eq(ticket_instances::asset_id)))
            .inner_join(ticket_types::table.on(ticket_types::id.eq(assets::ticket_type_id)))
            .inner_join(events::table.on(events::id.eq(ticket_types::event_id)))
            .inner_join(organizations::table.on(events::organization_id.eq(organizations::id)))
            .filter(transfer_tickets::transfer_id.eq(self.id))
            .select(organizations::all_columns)
            .distinct()
            .load(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load transfer organizations")
    }

    pub fn events(&self, conn: &PgConnection) -> Result<Vec<Event>, DatabaseError> {
        transfer_tickets::table
            .inner_join(ticket_instances::table.on(ticket_instances::id.eq(transfer_tickets::ticket_instance_id)))
            .inner_join(assets::table.on(assets::id.eq(ticket_instances::asset_id)))
            .inner_join(ticket_types::table.on(ticket_types::id.eq(assets::ticket_type_id)))
            .inner_join(events::table.on(events::id.eq(ticket_types::event_id)))
            .filter(transfer_tickets::transfer_id.eq(self.id))
            .select(events::all_columns)
            .distinct()
            .load::<EventData>(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load transfer events")
            .map(EventData::vec_into_events)
    }

    pub fn orders(&self, conn: &PgConnection) -> Result<Vec<Order>, DatabaseError> {
        order_transfers::table
            .inner_join(orders::table.on(orders::id.eq(order_transfers::order_id)))
            .filter(order_transfers::transfer_id.eq(self.id))
            .select(orders::all_columns)
            .then_order_by(orders::created_at.desc())
            .load(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load transfer orders")
    }

    pub fn update_associated_orders(&self, conn: &PgConnection) -> Result<(), DatabaseError> {
        if self.status != TransferStatus::Pending {
            return DatabaseError::business_process_error("Transfer cannot be updated as it is no longer pending");
        }
        let query = r#"
            INSERT INTO order_transfers (order_id, transfer_id)
            SELECT DISTINCT o.id, t.id
            FROM transfers t
            JOIN transfer_tickets tt ON t.id = tt.transfer_id
            JOIN ticket_instances ti ON ti.id = tt.ticket_instance_id
            JOIN order_items oi ON ti.order_item_id = oi.id
            JOIN orders o ON o.id = oi.order_id
            WHERE t.id = $1
            AND COALESCE(o.on_behalf_of_user_id, o.user_id) = $2
        "#;

        diesel::sql_query(query)
            .bind::<dUuid, _>(self.id)
            .bind::<dUuid, _>(self.source_user_id)
            .execute(conn)
            .to_db_error(ErrorCode::QueryError, "Could not update associated orders")?;
        Ok(())
    }

    pub fn update(
        &self,
        attributes: TransferEditableAttributes,
        conn: &PgConnection,
    ) -> Result<Transfer, DatabaseError> {
        DatabaseError::wrap(
            ErrorCode::UpdateError,
            "Could not update transfer",
            diesel::update(self)
                .set((attributes, transfers::updated_at.eq(dsl::now)))
                .get_result(conn),
        )
    }
}

impl NewTransfer {
    pub fn commit(&self, conn: &PgConnection) -> Result<Transfer, DatabaseError> {
        self.validate_record(conn)?;
        let transfer: Transfer = DatabaseError::wrap(
            ErrorCode::InsertError,
            "Could not create new transfer",
            diesel::insert_into(transfers::table)
                .values((
                    self,
                    transfers::destination_temporary_user_id
                        .eq(Transfer::temporary_user_id(self.transfer_address.clone())),
                ))
                .get_result(conn),
        )?;
        Ok(transfer)
    }

    fn validate_record(&self, conn: &PgConnection) -> Result<(), DatabaseError> {
        let validation_errors = validators::append_validation_error(
            Ok(()),
            "transfer_key",
            Transfer::transfer_key_unique(self.transfer_key, conn)?,
        );

        Ok(validation_errors?)
    }
}
