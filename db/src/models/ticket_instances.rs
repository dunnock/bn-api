use chrono::prelude::*;
use chrono::Duration;
use diesel;
use diesel::dsl::*;
use diesel::expression::dsl;
use diesel::prelude::*;
use diesel::sql_types;
use diesel::sql_types::{Array, BigInt, Bool, Integer, Nullable, Text, Timestamp, Uuid as dUuid};
use itertools::Itertools;
use log::Level::Debug;
use log::Level::Error;
use models::*;
use rand;
use rand::Rng;
use schema::{
    assets, events, order_items, orders, organizations, ticket_instances, ticket_types, transfers, users, wallets,
};
use std::cmp;
use tari_client::*;
use utils::errors::*;
use uuid::Uuid;
use validators::*;

const TICKET_NUMBER_LENGTH: usize = 8;

#[derive(Clone, Debug, Identifiable, PartialEq, Deserialize, Serialize, Queryable, QueryableByName)]
#[table_name = "ticket_instances"]
pub struct TicketInstance {
    pub id: Uuid,
    pub asset_id: Uuid,
    pub token_id: i32,
    pub hold_id: Option<Uuid>,
    pub order_item_id: Option<Uuid>,
    pub wallet_id: Uuid,
    pub reserved_until: Option<NaiveDateTime>,
    pub redeem_key: Option<String>,
    pub status: TicketInstanceStatus,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub redeemed_by_user_id: Option<Uuid>,
    pub redeemed_at: Option<NaiveDateTime>,
    pub first_name_override: Option<String>,
    pub last_name_override: Option<String>,
    pub check_in_source: Option<CheckInSource>,
    parent_id: Option<Uuid>,
    pub listing_id: Option<Uuid>,
}

#[derive(AsChangeset, Clone, Deserialize, Serialize)]
#[table_name = "ticket_instances"]
pub struct UpdateTicketInstanceAttributes {
    #[serde(default, deserialize_with = "double_option_deserialize_unless_blank")]
    pub first_name_override: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option_deserialize_unless_blank")]
    pub last_name_override: Option<Option<String>>,
}

impl TicketInstance {
    pub fn event(&self, conn: &PgConnection) -> Result<Event, DatabaseError> {
        ticket_instances::table
            .inner_join(assets::table.on(assets::id.eq(ticket_instances::asset_id)))
            .inner_join(ticket_types::table.on(ticket_types::id.eq(assets::ticket_type_id)))
            .inner_join(events::table.on(events::id.eq(ticket_types::event_id)))
            .filter(ticket_instances::id.eq(self.id))
            .select(events::all_columns)
            .first(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load event for ticket instance")
    }

    pub fn redeem_key_unique_per_event(
        id: Uuid,
        redeem_key: String,
        conn: &PgConnection,
    ) -> Result<bool, DatabaseError> {
        let event_id: Uuid = ticket_types::table
            .inner_join(assets::table.on(ticket_types::id.eq(assets::ticket_type_id)))
            .inner_join(ticket_instances::table.on(assets::id.eq(ticket_instances::asset_id)))
            .filter(ticket_instances::id.eq(id))
            .select(ticket_types::event_id)
            .get_result(conn)
            .to_db_error(ErrorCode::QueryError, "Could not get event_id for ticket instance")?;

        select(exists(
            ticket_instances::table
                .inner_join(assets::table.on(assets::id.eq(ticket_instances::asset_id)))
                .inner_join(ticket_types::table.on(ticket_types::id.eq(assets::ticket_type_id)))
                .filter(ticket_instances::id.ne(id))
                .filter(ticket_instances::redeem_key.eq(redeem_key))
                .filter(ticket_types::event_id.eq(event_id)),
        ))
        .get_result(conn)
        .to_db_error(
            ErrorCode::QueryError,
            "Could not confirm if redeem key is unique per event",
        )
        .map(|exists: bool| !exists)
    }

    pub fn parse_ticket_number(id: Uuid) -> String {
        let id_string = id.to_string();
        id_string[id_string.len() - TICKET_NUMBER_LENGTH..].to_string()
    }

    pub fn organization(&self, conn: &PgConnection) -> Result<Organization, DatabaseError> {
        ticket_instances::table
            .inner_join(assets::table.on(assets::id.eq(ticket_instances::asset_id)))
            .inner_join(ticket_types::table.on(ticket_types::id.eq(assets::ticket_type_id)))
            .inner_join(events::table.on(events::id.eq(ticket_types::event_id)))
            .inner_join(organizations::table.on(organizations::id.eq(events::organization_id)))
            .filter(ticket_instances::id.eq(self.id))
            .select(organizations::all_columns)
            .first(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load organization for ticket instance")
    }

    pub fn owner(&self, conn: &PgConnection) -> Result<User, DatabaseError> {
        ticket_instances::table
            .inner_join(wallets::table.on(wallets::id.eq(ticket_instances::wallet_id)))
            .inner_join(users::table.on(users::id.nullable().eq(wallets::user_id)))
            .filter(ticket_instances::id.eq(self.id))
            .select(users::all_columns)
            .first(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load owner for ticket instance")
    }

    pub fn ticket_type(&self, conn: &PgConnection) -> Result<TicketType, DatabaseError> {
        ticket_instances::table
            .inner_join(assets::table.on(ticket_instances::asset_id.eq(assets::id)))
            .inner_join(ticket_types::table.on(assets::ticket_type_id.eq(ticket_types::id)))
            .filter(ticket_instances::id.eq(self.id))
            .select(ticket_types::all_columns)
            .first(conn)
            .to_db_error(ErrorCode::QueryError, "Unable to load ticket type for ticket instance")
    }

    pub fn find(id: Uuid, conn: &PgConnection) -> Result<TicketInstance, DatabaseError> {
        ticket_instances::table
            .find(id)
            .first(conn)
            .to_db_error(ErrorCode::QueryError, "Unable to load ticket")
    }

    pub fn release(
        &self,
        status: TicketInstanceStatus,
        user_id: Uuid,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        let query = include_str!("../queries/release_tickets.sql");
        let new_status = if self.ticket_type(conn)?.status == TicketTypeStatus::Cancelled {
            TicketInstanceStatus::Nullified
        } else {
            TicketInstanceStatus::Available
        };

        let tickets: Vec<TicketInstance> = diesel::sql_query(query)
            .bind::<Nullable<dUuid>, _>(self.order_item_id)
            .bind::<BigInt, _>(1)
            .bind::<Array<Text>, _>(vec![status])
            .bind::<dUuid, _>(self.id)
            .bind::<Text, _>(new_status)
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Could not release ticket")?;

        if tickets.len() != 1 {
            return DatabaseError::validation_error("quantity", "Could not release the ticket");
        }

        if new_status == TicketInstanceStatus::Nullified {
            tickets[0].create_nullified_domain_event(Some(user_id), conn)?;
        }

        Ok(())
    }

    // Check if a ticket has been transferred from the original user. Ignore the case when it was done by a Box Office user
    pub fn was_transferred(&self, conn: &PgConnection) -> Result<bool, DatabaseError> {
        ticket_instances::table
            .inner_join(wallets::table.on(ticket_instances::wallet_id.eq(wallets::id)))
            .left_join(
                order_items::table
                    .on(ticket_instances::order_item_id.eq(order_items::id.nullable())),
            )
            .left_join(
                orders::table.on(order_items::order_id.eq(orders::id)),
            )
            .filter(ticket_instances::id.eq(self.id))
            .select(
                sql::<Bool>("CASE WHEN order_items.id IS NULL THEN TRUE ELSE (orders.user_id <> wallets.user_id AND (orders.on_behalf_of_user_id IS NULL OR wallets.user_id <> orders.on_behalf_of_user_id)) END")
            )
            .get_result(conn)
            .to_db_error(ErrorCode::QueryError, "Unable to check if ticket instance was transferred")
    }

    pub fn find_for_display(
        id: Uuid,
        conn: &PgConnection,
    ) -> Result<(DisplayEvent, Option<DisplayUser>, DisplayTicket), DatabaseError> {
        let ticket_intermediary = ticket_instances::table
            .inner_join(assets::table.on(ticket_instances::asset_id.eq(assets::id)))
            .inner_join(order_items::table.on(ticket_instances::order_item_id.eq(order_items::id.nullable())))
            .inner_join(ticket_types::table.on(assets::ticket_type_id.eq(ticket_types::id)))
            .inner_join(wallets::table.on(ticket_instances::wallet_id.eq(wallets::id)))
            .inner_join(events::table.on(ticket_types::event_id.eq(events::id)))
            .left_join(transfers::table.on(sql("transfers.id = (
                    SELECT tt.transfer_id
                    FROM transfer_tickets tt
                    JOIN transfers t
                    ON tt.transfer_id = t.id
                    WHERE tt.ticket_instance_id = ticket_instances.id
                    AND t.status = 'Pending'
                )")))
            .filter(ticket_instances::id.eq(id))
            .select((
                ticket_instances::id,
                order_items::order_id,
                sql::<BigInt>(
                    "cast(unit_price_in_cents +
                    coalesce((
                        select sum(unit_price_in_cents)
                        from order_items
                        where parent_id = ticket_instances.order_item_id),
                    0) as BigInt)
                    ",
                ),
                assets::ticket_type_id,
                ticket_types::name,
                wallets::user_id,
                events::id,
                events::venue_id,
                ticket_instances::status,
                ticket_instances::redeem_key,
                events::redeem_date,
                events::event_start,
                sql::<Bool>("transfers.id is not null AS pending_transfer"),
                ticket_instances::first_name_override,
                ticket_instances::last_name_override,
                transfers::id.nullable(),
                transfers::transfer_key.nullable(),
                transfers::transfer_address.nullable(),
                ticket_instances::check_in_source,
                ticket_types::promo_image_url,
            ))
            .first::<DisplayTicketIntermediary>(conn)
            .to_db_error(ErrorCode::QueryError, "Unable to load ticket")?;
        let event = Event::find(ticket_intermediary.event_id, conn)?.for_display(conn)?;
        let user: Option<DisplayUser> = match ticket_intermediary.user_id {
            Some(uid) => Some(User::find(uid, conn)?.into()),
            None => None,
        };
        Ok((event, user, ticket_intermediary.into()))
    }

    pub fn find_by_event_id_redeem_key(
        event_id: Uuid,
        redeem_key: String,
        conn: &PgConnection,
    ) -> Result<TicketInstance, DatabaseError> {
        ticket_instances::table
            .inner_join(assets::table.on(ticket_instances::asset_id.eq(assets::id)))
            .inner_join(order_items::table.on(ticket_instances::order_item_id.eq(order_items::id.nullable())))
            .inner_join(ticket_types::table.on(assets::ticket_type_id.eq(ticket_types::id)))
            .filter(ticket_types::event_id.eq(event_id))
            .filter(ticket_instances::redeem_key.eq(redeem_key))
            .select(ticket_instances::all_columns)
            .get_result(conn)
            .to_db_error(ErrorCode::QueryError, "Unable to load ticket")
    }

    pub fn find_for_processing(
        id: Uuid,
        event_id: Uuid,
        conn: &PgConnection,
    ) -> Result<ProcessingTicketIntermediary, DatabaseError> {
        let ticket_intermediary = ticket_instances::table
            .inner_join(assets::table.on(ticket_instances::asset_id.eq(assets::id)))
            .inner_join(ticket_types::table.on(assets::ticket_type_id.eq(ticket_types::id)))
            .inner_join(wallets::table.on(ticket_instances::wallet_id.eq(wallets::id)))
            .inner_join(events::table.on(ticket_types::event_id.eq(events::id)))
            .filter(ticket_instances::id.eq(id))
            .filter(events::id.eq(event_id))
            .select((
                ticket_instances::id,
                ticket_instances::asset_id,
                ticket_instances::token_id,
                ticket_instances::wallet_id,
                ticket_types::name,
                wallets::user_id,
                events::id,
                events::venue_id,
            ))
            .first::<ProcessingTicketIntermediary>(conn)
            .to_db_error(ErrorCode::QueryError, "Unable to load ticket")?;

        //let event = Event::find(ticket_intermediary.event_id, conn)?.for_display(conn)?;
        Ok(ticket_intermediary)
    }

    pub fn find_for_user_for_display(
        user_id: Uuid,
        event_id: Option<Uuid>,
        start_time: Option<NaiveDateTime>,
        end_time: Option<NaiveDateTime>,
        conn: &PgConnection,
    ) -> Result<Vec<(DisplayEvent, Vec<DisplayTicket>)>, DatabaseError> {
        let mut query = ticket_instances::table
            .inner_join(assets::table.on(ticket_instances::asset_id.eq(assets::id)))
            .inner_join(order_items::table.on(ticket_instances::order_item_id.eq(order_items::id.nullable())))
            .inner_join(ticket_types::table.on(assets::ticket_type_id.eq(ticket_types::id)))
            .inner_join(wallets::table.on(ticket_instances::wallet_id.eq(wallets::id)))
            .inner_join(events::table.on(ticket_types::event_id.eq(events::id)))
            .left_join(transfers::table.on(sql("transfers.id = (
                        SELECT tt.transfer_id
                        FROM transfer_tickets tt
                        JOIN transfers t
                        ON tt.transfer_id = t.id
                        WHERE tt.ticket_instance_id = ticket_instances.id
                        AND t.status = 'Pending'
                    )")))
            .filter(
                events::event_end.ge(start_time.unwrap_or_else(|| NaiveDate::from_ymd(1970, 1, 1).and_hms(0, 0, 0))),
            )
            .filter(events::event_end.le(end_time.unwrap_or_else(|| NaiveDate::from_ymd(3970, 1, 1).and_hms(0, 0, 0))))
            .filter(wallets::user_id.eq(user_id))
            .into_boxed();

        if let Some(event_id) = event_id {
            query = query.filter(events::id.eq(event_id));
        }

        let tickets = query
            .select((
                ticket_instances::id,
                order_items::order_id,
                sql::<BigInt>(
                    "cast(unit_price_in_cents +
                    coalesce((
                        select sum(unit_price_in_cents)
                        from order_items
                        where parent_id = ticket_instances.order_item_id),
                    0) as BigInt)
                    ",
                ),
                assets::ticket_type_id,
                ticket_types::name,
                wallets::user_id,
                events::id,
                events::venue_id,
                ticket_instances::status,
                ticket_instances::redeem_key,
                events::redeem_date,
                events::event_start,
                sql::<Bool>("transfers.id is not null AS pending_transfer"),
                ticket_instances::first_name_override,
                ticket_instances::last_name_override,
                transfers::id.nullable(),
                transfers::transfer_key.nullable(),
                transfers::transfer_address.nullable(),
                ticket_instances::check_in_source,
                ticket_types::promo_image_url,
            ))
            .order_by(events::event_start.asc())
            .then_order_by(events::name.asc())
            .then_order_by(events::id.asc())
            .load::<DisplayTicketIntermediary>(conn)
            .to_db_error(ErrorCode::QueryError, "Unable to load user tickets")?;

        let mut grouped_display_tickets = Vec::new();
        for (key, group) in &tickets.into_iter().group_by(|ticket| ticket.event_id) {
            let event = Event::find(key, conn)?.for_display(conn)?;
            let display_tickets: Vec<DisplayTicket> = group.into_iter().map(|ticket| ticket.into()).collect();
            grouped_display_tickets.push((event, display_tickets));
        }

        Ok(grouped_display_tickets)
    }

    pub fn find_for_user(user_id: Uuid, conn: &PgConnection) -> Result<Vec<TicketInstance>, DatabaseError> {
        ticket_instances::table
            .inner_join(wallets::table.on(ticket_instances::wallet_id.eq(wallets::id)))
            .filter(wallets::user_id.eq(user_id))
            .select(ticket_instances::all_columns)
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load ticket instances")
    }

    pub fn find_by_ids(
        ticket_instance_ids: &[Uuid],
        conn: &PgConnection,
    ) -> Result<Vec<TicketInstance>, DatabaseError> {
        ticket_instances::table
            .filter(ticket_instances::id.eq_any(ticket_instance_ids))
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load Ticket Instances")
    }

    pub fn create_single(
        asset_id: Uuid,
        tari_id: i32,
        wallet_id: Uuid,
        conn: &PgConnection,
    ) -> Result<TicketInstance, DatabaseError> {
        let new_row = NewTicketInstance {
            asset_id,
            token_id: tari_id as i32,
            wallet_id,
        };

        diesel::insert_into(ticket_instances::table)
            .values(&new_row)
            .get_result(conn)
            .to_db_error(ErrorCode::InsertError, "Could not create ticket instance")
    }

    pub fn create_multiple(
        asset_id: Uuid,
        starting_tari_id: u32,
        quantity: u32,
        wallet_id: Uuid,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        let mut new_rows = Vec::<NewTicketInstance>::new();
        new_rows.reserve(1_000);
        for x in 0..quantity {
            new_rows.push(NewTicketInstance {
                asset_id,
                token_id: (starting_tari_id + x) as i32,
                wallet_id,
            });

            if x % 1_000 == 0 {
                diesel::insert_into(ticket_instances::table)
                    .values(&new_rows)
                    .execute(conn)
                    .to_db_error(ErrorCode::InsertError, "Could not create ticket instances")?;
                new_rows.truncate(0);
            }
        }

        if !new_rows.is_empty() {
            diesel::insert_into(ticket_instances::table)
                .values(&new_rows)
                .execute(conn)
                .to_db_error(ErrorCode::InsertError, "Could not create ticket instances")?;
        }
        Ok(())
    }

    pub fn reserve_tickets(
        order_item: &OrderItem,
        expires_at: Option<NaiveDateTime>,
        ticket_type_id: Uuid,
        ticket_holding_id: Option<Uuid>,
        quantity: u32,
        conn: &PgConnection,
    ) -> Result<Vec<TicketInstance>, DatabaseError> {
        let order_expires_at = expires_at.ok_or(DatabaseError::new(
            ErrorCode::BusinessProcessError,
            Some("Expiration date was not set on cart prior to reserving tickets".to_string()),
        ))?;

        let query = include_str!("../queries/reserve_tickets.sql");
        let q = diesel::sql_query(query)
            .bind::<sql_types::Uuid, _>(order_item.id)
            .bind::<sql_types::Timestamp, _>(order_expires_at)
            .bind::<sql_types::Uuid, _>(ticket_type_id)
            .bind::<sql_types::Nullable<sql_types::Uuid>, _>(ticket_holding_id)
            .bind::<BigInt, _>(quantity as i64);
        let tickets: Vec<TicketInstance> = q
            .get_results(conn)
            .to_db_error(ErrorCode::UpdateError, "Could not reserve tickets")?;

        if tickets.len() as u32 != quantity {
            if (tickets.len() as u32) < quantity {
                jlog!(
                    Debug,
                    &format!(
                        "Could not reserve {} tickets, only {} tickets were available",
                        quantity,
                        tickets.len()
                    )
                );

                return DatabaseError::validation_error(
                    "quantity",
                    "Could not reserve tickets, not enough tickets are available",
                );
            } else {
                jlog!(Error, "Reserved too many tickets", {"quantity_requested": quantity, "quantity_reserved": quantity, "tickets":&tickets});
                // This is an unlikely scenario
                return DatabaseError::business_process_error(&format!(
                    "Reserved too many tickets, expected {} tickets, reserved {}",
                    quantity,
                    tickets.len()
                ));
            }
        }

        Ok(tickets)
    }

    fn validate_record(&self, update_attrs: &UpdateTicketInstanceAttributes) -> Result<(), DatabaseError> {
        let mut validation_errors = Ok(());
        let first_name = update_attrs
            .first_name_override
            .clone()
            .unwrap_or(self.first_name_override.clone());
        let last_name = update_attrs
            .last_name_override
            .clone()
            .unwrap_or(self.last_name_override.clone());

        if first_name.is_some() && last_name.is_none() {
            validation_errors = append_validation_error(
                validation_errors,
                "last_name_override",
                Err(create_validation_error(
                    "required",
                    "Ticket last name required if first name provided",
                )),
            );
        } else if first_name.is_none() && last_name.is_some() {
            validation_errors = append_validation_error(
                validation_errors,
                "first_name_override",
                Err(create_validation_error(
                    "required",
                    "Ticket first name required if last name provided",
                )),
            );
        }
        Ok(validation_errors?)
    }

    pub fn release_tickets(
        order_item: &OrderItem,
        quantity: u32,
        user_id: Option<Uuid>,
        conn: &PgConnection,
    ) -> Result<Vec<TicketInstance>, DatabaseError> {
        let query = include_str!("../queries/release_tickets.sql");
        let ticket_type = order_item.ticket_type(conn)?;
        let new_status = if ticket_type.is_some() && ticket_type.unwrap().status == TicketTypeStatus::Cancelled {
            TicketInstanceStatus::Nullified
        } else {
            TicketInstanceStatus::Available
        };
        let ticket_instance_id: Option<Uuid> = None;
        let q = diesel::sql_query(query)
            .bind::<sql_types::Uuid, _>(order_item.id)
            .bind::<BigInt, _>(quantity as i64)
            // Removes nullified or reserved tickets from a user's cart
            .bind::<Array<Text>, _>(vec![
                TicketInstanceStatus::Nullified,
                TicketInstanceStatus::Reserved,
            ])
            .bind::<Nullable<dUuid>, _>(ticket_instance_id)
            .bind::<Text, _>(new_status);
        let tickets: Vec<TicketInstance> = q
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Could not release tickets")?;

        if tickets.len() as u32 != quantity {
            return DatabaseError::validation_error("quantity", "Could not release the correct amount of tickets");
        }

        if new_status == TicketInstanceStatus::Nullified {
            for ticket in &tickets {
                ticket.create_nullified_domain_event(user_id, conn)?;
            }
        }

        Ok(tickets)
    }

    pub fn find_children(&self, conn: &PgConnection) -> Result<Vec<TicketInstance>, DatabaseError> {
        ticket_instances::table
            .filter(ticket_instances::parent_id.eq(self.id))
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Could not find ticket instances for parent")
    }

    pub(crate) fn add_to_loot_box_instance(
        _current_user_id: Option<Uuid>,
        parent_ticket_instance_id: Uuid,
        event_id: Uuid,
        ticket_type_id: Option<Uuid>,
        min_rarity_id: Option<Uuid>,
        max_rarity_id: Option<Uuid>,
        quantity: i64,
        conn: &PgConnection,
    ) -> Result<Vec<TicketInstance>, DatabaseError> {
        println!("{:?}", parent_ticket_instance_id.to_string());
        println!("{:?}", event_id.to_string());
        println!(
            "{:?}",
            [
                ticket_type_id.unwrap_or(Uuid::nil()).to_string(),
                min_rarity_id.unwrap_or(Uuid::nil()).to_string(),
                max_rarity_id.unwrap_or(Uuid::nil()).to_string()
            ]
        );
        let query = include_str!("../queries/add_tickets_to_loot_box_instance.sql");
        let q = diesel::sql_query(query)
            .bind::<sql_types::Uuid, _>(parent_ticket_instance_id)
            .bind::<sql_types::Uuid, _>(event_id)
            .bind::<sql_types::Nullable<sql_types::Uuid>, _>(ticket_type_id)
            .bind::<sql_types::Nullable<sql_types::Uuid>, _>(min_rarity_id)
            .bind::<sql_types::Nullable<sql_types::Uuid>, _>(max_rarity_id)
            .bind::<BigInt, _>(quantity);

        let tickets: Vec<TicketInstance> = q
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Could not add tickets to the loot box")?;

        if tickets.len() as i64 != quantity {
            if (tickets.len() as i64) < quantity {
                jlog!(
                    Debug,
                    &format!(
                        "Could not reserve {} tickets, only {} tickets were available",
                        quantity,
                        tickets.len()
                    )
                );

                return DatabaseError::validation_error(
                    "quantity",
                    "Could not reserve tickets, not enough tickets are available",
                );
            } else {
                jlog!(Error, "Reserved too many tickets for loot box", {"quantity_requested": quantity, "quantity_reserved": quantity, "tickets":&tickets});
                // This is an unlikely scenario
                return DatabaseError::business_process_error(&format!(
                    "Reserved too many tickets, expected {} tickets, reserved {}",
                    quantity,
                    tickets.len()
                ));
            }
        }

        //        for ticket in tickets.iter() {
        //            DomainEvent::create(
        //                DomainEventTypes::TicketInstanceAddedToHold,
        //                "Ticket added to hold".to_string(),
        //                Tables::TicketInstances,
        //                Some(ticket.id),
        //                current_user_id,
        //                Some(json!({"hold_id": hold_id, "from_hold_id": from_hold_id})),
        //            )
        //                .commit(conn)?;
        //        }

        Ok(tickets)
    }

    pub(crate) fn add_to_hold(
        current_user_id: Option<Uuid>,
        hold_id: Uuid,
        ticket_type_id: Uuid,
        quantity: u32,
        from_hold_id: Option<Uuid>,
        conn: &PgConnection,
    ) -> Result<Vec<TicketInstance>, DatabaseError> {
        let query = include_str!("../queries/add_tickets_to_hold.sql");
        let q = diesel::sql_query(query)
            .bind::<sql_types::Uuid, _>(hold_id)
            .bind::<sql_types::Uuid, _>(ticket_type_id)
            .bind::<BigInt, _>(quantity as i64)
            .bind::<Nullable<sql_types::Uuid>, _>(from_hold_id);

        let tickets: Vec<TicketInstance> = q
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Could not add tickets to the hold")?;

        if tickets.len() as u32 != quantity {
            if (tickets.len() as u32) < quantity {
                jlog!(
                    Debug,
                    &format!(
                        "Could not reserve {} tickets, only {} tickets were available",
                        quantity,
                        tickets.len()
                    )
                );

                return DatabaseError::validation_error(
                    "quantity",
                    "Could not reserve tickets, not enough tickets are available",
                );
            } else {
                jlog!(Error, "Reserved too many tickets for hold", {"quantity_requested": quantity, "quantity_reserved": quantity, "tickets":&tickets});
                // This is an unlikely scenario
                return DatabaseError::business_process_error(&format!(
                    "Reserved too many tickets, expected {} tickets, reserved {}",
                    quantity,
                    tickets.len()
                ));
            }
        }

        for ticket in tickets.iter() {
            DomainEvent::create(
                DomainEventTypes::TicketInstanceAddedToHold,
                "Ticket added to hold".to_string(),
                Tables::TicketInstances,
                Some(ticket.id),
                current_user_id,
                Some(json!({"hold_id": hold_id, "from_hold_id": from_hold_id})),
            )
            .commit(conn)?;
        }

        Ok(tickets)
    }

    pub fn add_to_listing(
        current_user_id: Option<Uuid>,
        owner_wallet_id: Uuid,
        listing_id: Uuid,
        ticket_type_id: Uuid,
        quantity: u32,
        conn: &PgConnection,
    ) -> Result<Vec<TicketInstance>, DatabaseError> {
        let query = include_str!("../queries/add_tickets_to_listing.sql");
        let q = diesel::sql_query(query)
            .bind::<sql_types::Uuid, _>(owner_wallet_id)
            .bind::<sql_types::Uuid, _>(ticket_type_id)
            .bind::<BigInt, _>(quantity as i64)
            .bind::<sql_types::Uuid, _>(listing_id);

        let tickets: Vec<TicketInstance> = q
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Could not add tickets to the listing")?;

        if tickets.len() as u32 != quantity {
            if (tickets.len() as u32) < quantity {
                jlog!(
                    Debug,
                    &format!(
                        "Could not reserve {} tickets, only {} tickets were available",
                        quantity,
                        tickets.len()
                    )
                );

                return DatabaseError::validation_error(
                    "quantity",
                    "Could not reserve tickets, not enough tickets are available",
                );
            } else {
                jlog!(Error, "Reserved too many tickets for listing", {"quantity_requested": quantity, "quantity_reserved": quantity, "tickets":&tickets});
                // This is an unlikely scenario
                return DatabaseError::business_process_error(&format!(
                    "Reserved too many tickets, expected {} tickets, reserved {}",
                    quantity,
                    tickets.len()
                ));
            }
        }

        for ticket in tickets.iter() {
            DomainEvent::create(
                DomainEventTypes::TicketInstanceAddedToListing,
                "Ticket added to listing".to_string(),
                Tables::TicketInstances,
                Some(ticket.id),
                current_user_id,
                Some(json!({ "listing_id": listing_id })),
            )
            .commit(conn)?;
        }

        Ok(tickets)
    }

    pub fn release_from_listing(
        current_user_id: Option<Uuid>,
        listing_id: Uuid,
        ticket_type_id: Uuid,
        quantity: u32,
        conn: &PgConnection,
    ) -> Result<Vec<TicketInstance>, DatabaseError> {
        let query = include_str!("../queries/release_tickets_from_listing.sql");
        let q = diesel::sql_query(query)
            .bind::<sql_types::Uuid, _>(listing_id)
            .bind::<sql_types::Uuid, _>(ticket_type_id)
            .bind::<BigInt, _>(quantity as i64);

        let tickets: Vec<TicketInstance> = q
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Could not release tickets from the listing")?;

        if tickets.len() as u32 != quantity {
            return DatabaseError::validation_error("quantity", "Could not release the correct amount of tickets");
        }

        for ticket in tickets.iter() {
            DomainEvent::create(
                DomainEventTypes::TicketInstanceReleasedFromListing,
                "Ticket released from listing".to_string(),
                Tables::TicketInstances,
                Some(ticket.id),
                current_user_id,
                Some(json!({ "listing_id": listing_id })),
            )
            .commit(conn)?;
        }

        Ok(tickets)
    }

    pub fn release_from_hold(
        current_user_id: Option<Uuid>,
        hold_id: Uuid,
        ticket_type_id: Uuid,
        quantity: u32,
        conn: &PgConnection,
    ) -> Result<Vec<TicketInstance>, DatabaseError> {
        let query = include_str!("../queries/release_tickets_from_hold.sql");
        let q = diesel::sql_query(query)
            .bind::<sql_types::Uuid, _>(hold_id)
            .bind::<sql_types::Uuid, _>(ticket_type_id)
            .bind::<BigInt, _>(quantity as i64);

        let tickets: Vec<TicketInstance> = q
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Could not release tickets from the hold")?;

        if tickets.len() as u32 != quantity {
            return DatabaseError::validation_error("quantity", "Could not release the correct amount of tickets");
        }

        for ticket in tickets.iter() {
            DomainEvent::create(
                DomainEventTypes::TicketInstanceReleasedFromHold,
                "Ticket released from hold".to_string(),
                Tables::TicketInstances,
                Some(ticket.id),
                current_user_id,
                Some(json!({"from_hold_id": hold_id, "to_hold_id": ticket.hold_id})),
            )
            .commit(conn)?;
        }

        Ok(tickets)
    }

    pub fn count_for_hold(
        hold_id: Uuid,
        ticket_type_id: Uuid,
        include_children: bool,
        conn: &PgConnection,
    ) -> Result<(u32, u32), DatabaseError> {
        #[derive(Queryable)]
        struct R {
            ticket_count: Option<i64>,
            available_count: Option<i64>,
        };

        let mut query = ticket_instances::table
            .inner_join(assets::table)
            .filter(assets::ticket_type_id.eq(ticket_type_id))
            .into_boxed();

        if include_children {
            query = query.filter(
                sql("ticket_instances.hold_id in
                    (WITH RECURSIVE holds_r(id) AS (
                     SELECT h.id
                     FROM holds AS h
                     WHERE h.id = ")
                .bind::<dUuid, _>(hold_id)
                .sql(
                    "UNION ALL
                     SELECT h.id
                     FROM holds_r AS p, holds AS h
                     WHERE h.parent_hold_id = p.id
                    )
                    SELECT id FROM holds_r)",
                ),
            );
        } else {
            query = query.filter(ticket_instances::hold_id.eq(hold_id));
        }

        let result = query
            .select((
                sql::<sql_types::Nullable<sql_types::BigInt>>("COUNT(DISTINCT ticket_instances.id)"),
                sql::<sql_types::Nullable<sql_types::BigInt>>(
                    "SUM(CASE WHEN ticket_instances.status IN ('Available', 'Reserved') THEN 1 ELSE 0 END)",
                ),
            ))
            .first::<R>(conn)
            .to_db_error(
                ErrorCode::QueryError,
                "Could not retrieve the number of tickets in this hold",
            )
            .optional()?;

        match result {
            Some(r) => Ok((
                r.ticket_count.unwrap_or(0) as u32,
                r.available_count.unwrap_or(0) as u32,
            )),
            None => Ok((0, 0)),
        }
    }

    pub fn find_for_order_item(order_item_id: Uuid, conn: &PgConnection) -> Result<Vec<TicketInstance>, DatabaseError> {
        ticket_instances::table
            .filter(ticket_instances::order_item_id.eq(order_item_id))
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load Ticket Instances")
    }

    pub fn find_ids_for_order(order_id: Uuid, conn: &PgConnection) -> Result<Vec<Uuid>, DatabaseError> {
        ticket_instances::table
            .inner_join(order_items::table.on(ticket_instances::order_item_id.eq(order_items::id.nullable())))
            .filter(order_items::order_id.eq(order_id))
            .select(ticket_instances::id)
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load Ticket Instances")
    }

    pub fn update_reserved_time(
        order_item: &OrderItem,
        reserved_time: NaiveDateTime,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        if order_item.item_type != OrderItemTypes::Tickets {
            return Ok(());
        }
        let rows_affected = diesel::update(
            ticket_instances::table.filter(
                ticket_instances::order_item_id
                    .eq(order_item.id)
                    .and(ticket_instances::reserved_until.gt(dsl::now.nullable())),
            ),
        )
        .set((
            ticket_instances::reserved_until.eq(reserved_time),
            ticket_instances::updated_at.eq(dsl::now),
        ))
        .execute(conn)
        .to_db_error(
            ErrorCode::UpdateError,
            "Could not update ticket_instance reserved time.",
        )?;
        if rows_affected == 0 {
            jlog!(Debug, "Could not update reserved ticket time", { "order_item_id": order_item.id, "reserved_time": reserved_time});
            return DatabaseError::concurrency_error("Could not update reserved ticket time");
        };
        Ok(())
    }

    // Note: Transfer mechanism should be used in most cases over this method
    pub fn set_wallet(&self, wallet: &Wallet, conn: &PgConnection) -> Result<(), DatabaseError> {
        diesel::update(ticket_instances::table.filter(ticket_instances::id.eq(self.id)))
            .set((
                ticket_instances::wallet_id.eq(wallet.id()),
                ticket_instances::updated_at.eq(dsl::now),
            ))
            .execute(conn)
            .to_db_error(ErrorCode::UpdateError, "Could not update ticket_instance wallet.")?;
        Ok(())
    }

    pub fn mark_as_purchased(order_item: &OrderItem, user_id: Uuid, conn: &PgConnection) -> Result<(), DatabaseError> {
        let wallet = Wallet::find_for_user(user_id, conn)?;

        if wallet.is_empty() {
            return Err(DatabaseError::new(
                ErrorCode::InternalError,
                Some("User does not have a wallet associated with them".to_string()),
            ));
        }

        let tickets = diesel::update(ticket_instances::table.filter(ticket_instances::order_item_id.eq(order_item.id)))
            .set((
                ticket_instances::wallet_id.eq(wallet[0].id()),
                ticket_instances::status.eq(TicketInstanceStatus::Purchased),
                ticket_instances::updated_at.eq(dsl::now),
            ))
            .get_results::<TicketInstance>(conn)
            .to_db_error(
                ErrorCode::UpdateError,
                "Could not update ticket_instance status to purchased.",
            )?;

        //Generate redeem codes for the tickets
        for t in &tickets {
            let key = t.associate_redeem_key(conn)?;

            DomainEvent::create(
                DomainEventTypes::TicketInstancePurchased,
                "Ticket purchased".to_string(),
                Tables::TicketInstances,
                Some(t.id),
                Some(user_id),
                Some(json!({
                "order_id" : order_item.order_id, "wallet_id": wallet[0].id(), "order_item_id": order_item.id, "redeem_key": key}
            ))).commit(conn)?;
        }
        Ok(())
    }

    pub fn associate_redeem_key(&self, conn: &PgConnection) -> Result<String, DatabaseError> {
        let mut key = generate_redeem_key(9);
        loop {
            if TicketInstance::redeem_key_unique_per_event(self.id, key.clone(), conn)? {
                break;
            }
            key = generate_redeem_key(9);
        }

        diesel::update(self)
            .set(ticket_instances::redeem_key.eq(key.clone()))
            .execute(conn)
            .to_db_error(ErrorCode::InternalError, "Could not write redeem key")?;

        Ok(key)
    }

    pub fn has_pending_transfer(&self, conn: &PgConnection) -> Result<bool, DatabaseError> {
        Ok(TransferTicket::pending_transfer(self.id, conn)?.is_some())
    }

    pub fn redeem_ticket(
        ticket_id: Uuid,
        redeem_key: String,
        user_id: Uuid,
        check_in_source: CheckInSource,
        conn: &PgConnection,
    ) -> Result<RedeemResults, DatabaseError> {
        let ticket: TicketInstance = ticket_instances::table
            .find(ticket_id)
            .first(conn)
            .to_db_error(ErrorCode::QueryError, "Unable to load ticket")?;
        if ticket.has_pending_transfer(conn)? {
            return Ok(RedeemResults::TicketTransferInProcess);
        } else if ticket.status == TicketInstanceStatus::Purchased
            && ticket.redeem_key.is_some()
            && ticket.redeem_key.clone().unwrap() == redeem_key
        {
            diesel::update(ticket_instances::table.filter(ticket_instances::id.eq(ticket_id)))
                .set((
                    ticket_instances::status.eq(TicketInstanceStatus::Redeemed),
                    ticket_instances::redeemed_by_user_id.eq(user_id),
                    ticket_instances::redeemed_at.eq(dsl::now),
                    ticket_instances::check_in_source.eq(check_in_source),
                    ticket_instances::updated_at.eq(dsl::now),
                ))
                .execute(conn)
                .to_db_error(ErrorCode::UpdateError, "Could not set ticket to Redeemed")?;

            DomainEvent::create(
                DomainEventTypes::TicketInstanceRedeemed,
                "Ticket redeemed".to_string(),
                Tables::TicketInstances,
                Some(ticket.id),
                Some(user_id),
                None,
            )
            .commit(conn)?;
        } else if ticket.status == TicketInstanceStatus::Redeemed {
            return Ok(RedeemResults::TicketAlreadyRedeemed);
        } else {
            return Ok(RedeemResults::TicketInvalid);
        }
        Ok(RedeemResults::TicketRedeemSuccess)
    }

    pub fn show_redeemable_ticket(ticket_id: Uuid, conn: &PgConnection) -> Result<RedeemableTicket, DatabaseError> {
        let tickets_and_counts = Event::guest_list_tickets(None, Some(ticket_id), None, &None, None, conn)?;

        match tickets_and_counts.0.get(0) {
            Some(ticket_data) => {
                return Ok(ticket_data.clone());
            }
            None => {
                return Err(DatabaseError::new(
                    ErrorCode::QueryError,
                    Some("Unable to load ticket".to_string()),
                ));
            }
        }
    }

    pub fn direct_transfer(
        from_user: &User,
        ticket_ids: &[Uuid],
        address: &str,
        sent_via: TransferMessageType,
        to_user_id: Uuid,
        conn: &PgConnection,
    ) -> Result<Transfer, DatabaseError> {
        let transfer =
            TicketInstance::create_transfer(from_user, ticket_ids, Some(address), Some(sent_via), true, conn)?;
        let wallet = Wallet::find_default_for_user(from_user.id, conn)?;
        let receiver_wallet = Wallet::find_default_for_user(to_user_id, conn)?;
        TicketInstance::receive_ticket_transfer(
            transfer.into_authorization(conn)?,
            &wallet,
            to_user_id,
            receiver_wallet.id,
            conn,
        )?;

        // Reload transfer, confirm completed
        let transfer = Transfer::find(transfer.id, conn)?;
        if transfer.status != TransferStatus::Completed {
            return Err(DatabaseError::new(
                ErrorCode::BusinessProcessError,
                Some("Failed to complete transfer directly to user".to_string()),
            ));
        }

        Ok(transfer)
    }

    fn verify_tickets_belong_to_user(
        user_id: Uuid,
        ticket_ids: &[Uuid],
        conn: &PgConnection,
    ) -> Result<(Uuid, Vec<(Uuid, NaiveDateTime)>), DatabaseError> {
        let tickets = TicketInstance::find_for_user(user_id, conn)?;
        let mut ticket_ids_and_updated_at = vec![];
        let mut all_tickets_valid = true;
        let mut has_redeemed_tickets = false;
        let mut wallet_id = Uuid::nil();

        for ti in ticket_ids {
            let mut found_and_purchased = false;
            for t in &tickets {
                if t.id == *ti && t.status == TicketInstanceStatus::Purchased {
                    found_and_purchased = true;
                    ticket_ids_and_updated_at.push((*ti, t.updated_at));
                    wallet_id = t.wallet_id;
                    break;
                } else if t.id == *ti && t.status == TicketInstanceStatus::Redeemed {
                    has_redeemed_tickets = true;
                    break;
                }
            }
            if !found_and_purchased {
                all_tickets_valid = false;
                break;
            }
        }

        if has_redeemed_tickets {
            return DatabaseError::business_process_error("Redeemed tickets cannot be transferred");
        } else if !all_tickets_valid || tickets.len() == 0 {
            return DatabaseError::business_process_error("User does not own all requested tickets");
        }

        Ok((wallet_id, ticket_ids_and_updated_at))
    }

    pub fn create_transfer(
        user: &User,
        ticket_ids: &[Uuid],
        address: Option<&str>,
        sent_via: Option<TransferMessageType>,
        direct: bool,
        conn: &PgConnection,
    ) -> Result<Transfer, DatabaseError> {
        //Confirm that tickets are purchased and owned by user
        let (wallet_id, ticket_ids_and_updated_at) =
            TicketInstance::verify_tickets_belong_to_user(user.id, ticket_ids, conn)?;

        //Generate transfer_key and store keys and set transfer_expiry date
        let transfer_key = Uuid::new_v4();

        let mut update_count = 0;
        Transfer::cancel_by_ticket_instance_ids(ticket_ids, &user, Some(transfer_key), conn)?;

        let transfer_data = Some(json!({
            "sent_via": sent_via,
            "address": address,
            "sender_wallet_id": wallet_id,
            "transfer_key": &transfer_key
        }));
        let transfer =
            Transfer::create(user.id, transfer_key, sent_via, address.map(|a| a.to_string()), direct).commit(conn)?;
        for (t_id, _) in ticket_ids_and_updated_at {
            transfer.add_transfer_ticket(t_id, conn)?;
            update_count += 1;
        }
        if transfer.event_ended(conn)? {
            return Err(DatabaseError::new(
                ErrorCode::BusinessProcessError,
                Some("Cannot transfer ticket, event has ended.".to_string()),
            ));
        }

        transfer.update_associated_orders(conn)?;

        // Log transfer event after associating transfer tickets
        DomainEvent::create(
            DomainEventTypes::TransferTicketStarted,
            "Transfer ticket started".to_string(),
            Tables::Transfers,
            Some(transfer.id),
            Some(transfer.source_user_id),
            transfer_data,
        )
        .commit(conn)?;

        if update_count != ticket_ids.len() {
            return Err(DatabaseError::new(
                ErrorCode::UpdateError,
                Some("Could not update ticket instances".to_string()),
            ));
        }
        Ok(transfer)
    }

    pub fn update(
        self,
        attrs: UpdateTicketInstanceAttributes,
        current_user_id: Uuid,
        conn: &PgConnection,
    ) -> Result<TicketInstance, DatabaseError> {
        if self.status == TicketInstanceStatus::Redeemed {
            return DatabaseError::business_process_error("Unable to update ticket as it has already been redeemed.");
        } else if self.status != TicketInstanceStatus::Purchased {
            return DatabaseError::business_process_error("Unable to update ticket as it is not purchased.");
        }

        self.validate_record(&attrs)?;

        DomainEvent::create(
            DomainEventTypes::TicketInstanceUpdated,
            "Ticket instance updated".into(),
            Tables::TicketInstances,
            Some(self.id),
            Some(current_user_id),
            Some(json!(attrs)),
        )
        .commit(conn)?;

        diesel::update(&self)
            .set((attrs, ticket_instances::updated_at.eq(dsl::now)))
            .get_result(conn)
            .to_db_error(ErrorCode::UpdateError, "Could not update ticket instance")
    }

    pub fn receive_ticket_transfer(
        transfer_authorization: TransferAuthorization,
        sender_wallet: &Wallet,
        receiver_user_id: Uuid,
        receiver_wallet_id: Uuid,
        conn: &PgConnection,
    ) -> Result<Vec<TicketInstance>, DatabaseError> {
        //Validate signature
        let mut header: String = transfer_authorization.transfer_key.to_string();
        header.push_str(transfer_authorization.sender_user_id.to_string().as_str());
        header.push_str(transfer_authorization.num_tickets.to_string().as_str());
        if !cryptographic_verify(
            &convert_hexstring_to_bytes(&transfer_authorization.signature),
            &header,
            &convert_hexstring_to_bytes(&sender_wallet.public_key),
        ) {
            return Err(DatabaseError::new(
                ErrorCode::InternalError,
                Some("ECDSA Signature is not valid".to_string()),
            ));
        }

        let transfer = Transfer::find_by_transfer_key(transfer_authorization.transfer_key, conn)?;
        match transfer.status {
            TransferStatus::Cancelled => {
                return Err(DatabaseError::new(
                    ErrorCode::BusinessProcessError,
                    Some("The transfer has been cancelled.".to_string()),
                ));
            }
            TransferStatus::EventEnded => {
                return Err(DatabaseError::new(
                    ErrorCode::BusinessProcessError,
                    Some("Cannot transfer ticket, event has ended.".to_string()),
                ));
            }
            _ => (),
        }

        //Confirm that transfer authorization time has not passed and that the sender still owns the tickets
        //being transfered
        let tickets = transfer.tickets(conn)?;
        let mut own_all = true;
        let mut ticket_ids_to_transfer: Vec<(Uuid, NaiveDateTime)> = Vec::new();
        for t in &tickets {
            if t.wallet_id != sender_wallet.id {
                own_all = false;
                break;
            }
            ticket_ids_to_transfer.push((t.id, t.updated_at));
        }

        if !own_all || tickets.len() != transfer_authorization.num_tickets as usize {
            return Err(DatabaseError::new(
                ErrorCode::BusinessProcessError,
                Some("These tickets have already been transferred.".to_string()),
            ));
        }

        //Perform transfer
        let mut update_count = 0;
        for (t_id, updated_at) in &ticket_ids_to_transfer {
            let name_override: Option<String> = None;
            update_count += diesel::update(
                ticket_instances::table
                    .filter(ticket_instances::id.eq(t_id))
                    .filter(ticket_instances::updated_at.eq(updated_at)),
            )
            .set((
                ticket_instances::wallet_id.eq(receiver_wallet_id),
                ticket_instances::updated_at.eq(dsl::now),
                ticket_instances::first_name_override.eq(&name_override),
                ticket_instances::last_name_override.eq(&name_override),
            ))
            .execute(conn)
            .to_db_error(ErrorCode::UpdateError, "Could not update ticket instance")?;
        }

        transfer.complete(
            receiver_user_id,
            Some(json!({"receiver_wallet_id": receiver_wallet_id, "sender_wallet_id": sender_wallet.id, "num_tickets": transfer_authorization.num_tickets, "transfer_key": transfer_authorization.transfer_key})),
            conn
        )?;

        if update_count != transfer_authorization.num_tickets as usize {
            return Err(DatabaseError::new(
                ErrorCode::UpdateError,
                Some("Could not update ticket instances".to_string()),
            ));
        }

        Ok(tickets)
    }

    fn create_nullified_domain_event(&self, user_id: Option<Uuid>, conn: &PgConnection) -> Result<(), DatabaseError> {
        DomainEvent::create(
            DomainEventTypes::TicketInstanceNullified,
            "Ticket nullified".to_string(),
            Tables::TicketInstances,
            Some(self.id),
            user_id,
            None,
        )
        .commit(conn)?;

        Ok(())
    }

    pub fn nullify_tickets(
        asset_id: Uuid,
        quantity: u32,
        user_id: Uuid,
        conn: &PgConnection,
    ) -> Result<Vec<TicketInstance>, DatabaseError> {
        let query = include_str!("../queries/nullify_tickets.sql");
        let q = diesel::sql_query(query)
            .bind::<sql_types::Uuid, _>(asset_id)
            .bind::<sql_types::BigInt, _>(quantity as i64);
        let updated_ticket_instances: Vec<TicketInstance> = q
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Could not nullify tickets")?;

        for ticket_instance in &updated_ticket_instances {
            ticket_instance.create_nullified_domain_event(Some(user_id), conn)?;
        }
        Ok(updated_ticket_instances)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TransferAuthorization {
    pub transfer_key: Uuid,
    pub sender_user_id: Uuid,
    pub num_tickets: u32,
    pub signature: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct DisplayTicket {
    pub id: Uuid,
    pub order_id: Uuid,
    pub price_in_cents: u32,
    pub ticket_type_id: Uuid,
    pub ticket_type_name: String,
    pub status: TicketInstanceStatus,
    pub redeem_key: Option<String>,
    pub pending_transfer: bool,
    pub first_name_override: Option<String>,
    pub last_name_override: Option<String>,
    pub transfer_id: Option<Uuid>,
    pub transfer_key: Option<Uuid>,
    pub transfer_address: Option<String>,
    pub check_in_source: Option<CheckInSource>,
    pub promo_image_url: Option<String>,
}

#[derive(Queryable, QueryableByName)]
pub struct DisplayTicketIntermediary {
    #[sql_type = "dUuid"]
    pub id: Uuid,
    #[sql_type = "dUuid"]
    pub order_id: Uuid,
    #[sql_type = "BigInt"]
    pub unit_price_in_cents: i64,
    #[sql_type = "dUuid"]
    pub ticket_type_id: Uuid,
    #[sql_type = "Text"]
    pub name: String,
    #[sql_type = "Nullable<dUuid>"]
    pub user_id: Option<Uuid>,
    #[sql_type = "dUuid"]
    pub event_id: Uuid,
    #[sql_type = "Nullable<dUuid>"]
    pub venue_id: Option<Uuid>,
    #[sql_type = "Text"]
    pub status: TicketInstanceStatus,
    #[sql_type = "Nullable<Text>"]
    pub redeem_key: Option<String>,
    #[sql_type = "Nullable<Timestamp>"]
    pub redeem_date: Option<NaiveDateTime>,
    #[sql_type = "Nullable<Timestamp>"]
    pub event_start: Option<NaiveDateTime>,
    #[sql_type = "Bool"]
    pub pending_transfer: bool,
    #[sql_type = "Nullable<Text>"]
    pub first_name_override: Option<String>,
    #[sql_type = "Nullable<Text>"]
    pub last_name_override: Option<String>,
    #[sql_type = "Nullable<dUuid>"]
    pub transfer_id: Option<Uuid>,
    #[sql_type = "Nullable<dUuid>"]
    pub transfer_key: Option<Uuid>,
    #[sql_type = "Nullable<Text>"]
    pub transfer_address: Option<String>,
    #[sql_type = "Nullable<Text>"]
    pub check_in_source: Option<CheckInSource>,
    #[sql_type = "Nullable<Text>"]
    pub promo_image_url: Option<String>,
}

impl From<DisplayTicketIntermediary> for DisplayTicket {
    fn from(ticket_intermediary: DisplayTicketIntermediary) -> Self {
        let day_before_event_start = ticket_intermediary
            .event_start
            .map(|event_start| event_start - Duration::hours(24));

        let redemption_allowed_after_date = ticket_intermediary
            .redeem_date
            .map(|redeem_date| {
                if day_before_event_start.is_none() {
                    redeem_date
                } else {
                    // Whichever one is earlier
                    cmp::min(day_before_event_start.unwrap(), redeem_date)
                }
            })
            .or(day_before_event_start)
            .unwrap();

        let redeem_key = if Utc::now().naive_utc() > redemption_allowed_after_date {
            ticket_intermediary.redeem_key.clone()
        } else {
            None
        };

        DisplayTicket {
            id: ticket_intermediary.id,
            order_id: ticket_intermediary.order_id,
            price_in_cents: ticket_intermediary.unit_price_in_cents as u32,
            ticket_type_id: ticket_intermediary.ticket_type_id,
            ticket_type_name: ticket_intermediary.name.clone(),
            status: ticket_intermediary.status.clone(),
            pending_transfer: ticket_intermediary.pending_transfer,
            first_name_override: ticket_intermediary.first_name_override,
            last_name_override: ticket_intermediary.last_name_override,
            redeem_key,
            transfer_id: ticket_intermediary.transfer_id,
            transfer_key: ticket_intermediary.transfer_key,
            transfer_address: ticket_intermediary.transfer_address,
            check_in_source: ticket_intermediary.check_in_source,
            promo_image_url: ticket_intermediary.promo_image_url,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ProcessingTicket {
    pub id: Uuid,
    pub asset_id: Uuid,
    pub token_id: i32,
    pub wallet_id: Uuid,
    pub ticket_type_name: String,
}

#[derive(Queryable, QueryableByName)]
pub struct ProcessingTicketIntermediary {
    #[sql_type = "dUuid"]
    pub id: Uuid,
    #[sql_type = "dUuid"]
    pub asset_id: Uuid,
    #[sql_type = "Integer"]
    pub token_id: i32,
    #[sql_type = "dUuid"]
    pub wallet_id: Uuid,
    #[sql_type = "Text"]
    pub name: String,
    #[sql_type = "Nullable<dUuid>"]
    pub user_id: Option<Uuid>,
    #[sql_type = "dUuid"]
    pub event_id: Uuid,
    #[sql_type = "Nullable<dUuid>"]
    pub venue_id: Option<Uuid>,
}

impl From<ProcessingTicketIntermediary> for ProcessingTicket {
    fn from(ticket_intermediary: ProcessingTicketIntermediary) -> Self {
        ProcessingTicket {
            id: ticket_intermediary.id.clone(),
            asset_id: ticket_intermediary.asset_id.clone(),
            token_id: ticket_intermediary.token_id.clone(),
            wallet_id: ticket_intermediary.wallet_id.clone(),
            ticket_type_name: ticket_intermediary.name.clone(),
        }
    }
}

#[derive(Insertable)]
#[table_name = "ticket_instances"]
struct NewTicketInstance {
    asset_id: Uuid,
    token_id: i32,
    wallet_id: Uuid,
}

#[derive(Debug, PartialEq)]
pub enum RedeemResults {
    TicketRedeemSuccess,
    TicketAlreadyRedeemed,
    TicketInvalid,
    TicketTransferInProcess,
}

fn generate_redeem_key(len: u32) -> String {
    let hash_char_list = vec![
        '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'M', 'N', 'P',
        'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
    ];
    (0..len)
        .map(|_| hash_char_list[rand::thread_rng().gen_range(0, hash_char_list.len())])
        .collect()
}
