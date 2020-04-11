use chrono::{NaiveDateTime, Utc};
use dev::times;
use diesel;
use diesel::dsl;
use diesel::expression::sql_literal::sql;
use diesel::prelude::*;
use diesel::sql_types::{Nullable, Text, Uuid as dUuid};
use itertools::Itertools;
use models::*;
use schema::{
    assets, events, fee_schedules, organizations, ticket_instances, ticket_pricing, ticket_type_codes, ticket_types,
};
use serde_with::rust::double_option;
use std::cmp;
use std::cmp::Ordering;
use utils::errors::*;
use uuid::Uuid;
use validator::*;
use validators::{self, *};

#[derive(Associations, Clone, Debug, Identifiable, PartialEq, Queryable, QueryableByName, Serialize)]
#[table_name = "ticket_types"]
#[belongs_to(Event)]
pub struct TicketType {
    pub id: Uuid,
    pub event_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub status: TicketTypeStatus,
    pub start_date: Option<NaiveDateTime>,
    pub end_date: Option<NaiveDateTime>,
    pub increment: i32,
    pub limit_per_person: i32,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
    pub price_in_cents: i64,
    pub cancelled_at: Option<NaiveDateTime>,
    pub parent_id: Option<Uuid>,
    pub rank: i32,
    pub visibility: TicketTypeVisibility,
    pub additional_fee_in_cents: i64,
    pub deleted_at: Option<NaiveDateTime>,
    pub end_date_type: TicketTypeEndDateType,
    pub web_sales_enabled: bool,
    pub box_office_sales_enabled: bool,
    pub app_sales_enabled: bool,
    pub rarity_id: Option<Uuid>,
    pub ticket_type_type: TicketTypeType,
    pub promo_image_url: Option<String>,
    pub content_url: Option<String>,
}

impl PartialOrd for TicketType {
    fn partial_cmp(&self, other: &TicketType) -> Option<Ordering> {
        Some(self.id.cmp(&other.id))
    }
}

#[derive(AsChangeset, Default, Deserialize, Serialize)]
#[table_name = "ticket_types"]
pub struct TicketTypeEditableAttributes {
    pub name: Option<String>,
    #[serde(default, deserialize_with = "double_option_deserialize_unless_blank")]
    pub description: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option::deserialize")]
    pub start_date: Option<Option<NaiveDateTime>>,
    #[serde(default, deserialize_with = "double_option::deserialize")]
    pub end_date: Option<Option<NaiveDateTime>>,
    pub increment: Option<i32>,
    pub limit_per_person: Option<i32>,
    pub price_in_cents: Option<i64>,
    pub visibility: Option<TicketTypeVisibility>,
    #[serde(default, deserialize_with = "double_option::deserialize")]
    pub parent_id: Option<Option<Uuid>>,
    #[serde(default)]
    pub additional_fee_in_cents: Option<i64>,
    pub end_date_type: Option<TicketTypeEndDateType>,
    pub web_sales_enabled: Option<bool>,
    pub box_office_sales_enabled: Option<bool>,
    pub app_sales_enabled: Option<bool>,
    pub rank: Option<i32>,
}

impl TicketType {
    // Properties at the top

    pub fn event(&self, conn: &PgConnection) -> Result<Event, DatabaseError> {
        let res: Event = ticket_types::table
            .inner_join(events::table)
            .filter(ticket_types::id.eq(self.id))
            .select(events::all_columns)
            .load::<EventData>(conn)
            .to_db_error(ErrorCode::QueryError, "Could not retrieve event for ticket type")
            .map(EventData::vec_into_events)
            .expect_single()?;
        Ok(res)
    }

    pub fn fee_schedule(&self, conn: &PgConnection) -> Result<FeeSchedule, DatabaseError> {
        ticket_types::table
            .inner_join(events::table.inner_join(organizations::table.inner_join(fee_schedules::table)))
            .filter(ticket_types::id.eq(self.id))
            .select(fee_schedules::all_columns)
            .first(conn)
            .to_db_error(ErrorCode::QueryError, "Could not retrieve fee schedule for ticket type")
    }

    pub fn find_by_ids(ids: &Vec<Uuid>, conn: &PgConnection) -> Result<Vec<TicketType>, DatabaseError> {
        ticket_types::table
            .filter(ticket_types::id.eq_any(ids))
            .order_by(ticket_types::rank)
            .then_order_by(ticket_types::name)
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Error loading ticket types")
    }

    pub fn status(&self, box_office_pricing: bool, conn: &PgConnection) -> Result<TicketTypeStatus, DatabaseError> {
        let ticket_pricings = self.valid_ticket_pricing(true, conn)?;
        let current_ticket_pricing = self.current_ticket_pricing(box_office_pricing, conn).optional()?;
        let available = self.valid_available_ticket_count(conn)?;
        let mut status = self.status;
        let now = Utc::now().naive_utc();
        if self.status == TicketTypeStatus::Published {
            if available == 0 || self.increment > available as i32 {
                status = TicketTypeStatus::SoldOut;
            } else {
                if current_ticket_pricing.is_none() {
                    status = TicketTypeStatus::NoActivePricing;
                    let min_pricing = ticket_pricings.iter().min_by_key(|p| p.start_date);
                    let max_pricing = ticket_pricings.iter().max_by_key(|p| p.end_date);

                    if min_pricing.map(|p| p.start_date).unwrap_or(self.start_date(conn)?) > now {
                        status = TicketTypeStatus::OnSaleSoon;
                    }

                    if max_pricing.map(|p| p.end_date).unwrap_or(self.end_date(conn)?) < now {
                        status = TicketTypeStatus::SaleEnded;
                    }
                }
            }
        }

        Ok(status)
    }

    /// Creates a ticket type. `Event::add_ticket_type` should be used in most scenarios
    pub(crate) fn create(
        event_id: Uuid,
        name: String,
        description: Option<String>,
        start_date: Option<NaiveDateTime>,
        end_date: Option<NaiveDateTime>,
        end_date_type: TicketTypeEndDateType,
        increment: Option<i32>,
        limit_per_person: i32,
        price_in_cents: i64,
        visibility: TicketTypeVisibility,
        parent_id: Option<Uuid>,
        additional_fee_in_cents: i64,
        app_sales_enabled: bool,
        web_sales_enabled: bool,
        box_office_sales_enabled: bool,
        ticket_type_type: TicketTypeType,
        rarity_id: Option<Uuid>,
        promo_image_url: Option<String>,
        content_url: Option<String>,
    ) -> NewTicketType {
        NewTicketType {
            event_id,
            name,
            description,
            status: TicketTypeStatus::Published,
            start_date,
            end_date,
            increment,
            limit_per_person,
            price_in_cents,
            visibility,
            parent_id,
            additional_fee_in_cents,
            end_date_type,
            app_sales_enabled,
            web_sales_enabled,
            box_office_sales_enabled,
            ticket_type_type,
            rarity_id,
            promo_image_url,
            content_url,
        }
    }

    pub fn update(
        self,
        attributes: TicketTypeEditableAttributes,
        current_user_id: Option<Uuid>,
        conn: &PgConnection,
    ) -> Result<TicketType, DatabaseError> {
        let mut attributes = attributes;

        // Clear end date if not manual as value is ignored
        if attributes.end_date_type.unwrap_or(self.end_date_type) != TicketTypeEndDateType::Manual {
            attributes.end_date = Some(None);
        }

        self.validate_record(&mut attributes, conn)?;
        let result: TicketType = diesel::update(&self)
            .set((&attributes, ticket_types::updated_at.eq(dsl::now)))
            .get_result(conn)
            .to_db_error(ErrorCode::UpdateError, "Could not update ticket_types")?;

        DomainEvent::create(
            DomainEventTypes::TicketTypeUpdated,
            format!("Ticket type '{}' updated", &self.name),
            Tables::TicketTypes,
            Some(self.id),
            current_user_id,
            Some(json!(attributes)),
        )
        .commit(conn)?;

        //Delete the old default ticket pricing and create a new default to preserve the purchase history
        let ticket_pricing = TicketPricing::get_default(self.id, conn).optional()?;

        let mut previous_start_date: Option<NaiveDateTime> = None;
        match ticket_pricing {
            Some(tp) => {
                previous_start_date = Some(tp.start_date);
                tp.destroy(current_user_id, conn)?;
            }
            None => (),
        }

        let mut start_date = result.start_date(conn)?;

        // If the ticket type has a parent, the start date is dependent on the
        // sales of that ticket type.
        if let Some(parent) = result.parent(conn)? {
            if parent.valid_available_ticket_count(conn)? == 0 {
                // Try keep the previous date if it was set.
                start_date = if previous_start_date.unwrap_or(times::infinity()) < Utc::now().naive_utc() {
                    previous_start_date.unwrap()
                } else {
                    Utc::now().naive_utc()
                }
            }
        }

        self.add_ticket_pricing(
            result.name.clone(),
            // TODO: Replace with nullable
            start_date,
            result.end_date(conn)?,
            result.price_in_cents,
            false,
            Some(TicketPricingStatus::Default),
            current_user_id,
            conn,
        )?;

        result.shift_other_rank(self.rank, conn)?;

        Ok(result)
    }

    pub fn update_rank_only(self, new_rank: i32, conn: &PgConnection) -> Result<TicketType, DatabaseError> {
        let result: TicketType = diesel::update(&self)
            .set((ticket_types::rank.eq(new_rank), ticket_types::updated_at.eq(dsl::now)))
            .get_result(conn)
            .to_db_error(ErrorCode::UpdateError, "Could not update ticket type rank")?;

        Ok(result)
    }

    pub fn cancel(self, conn: &PgConnection) -> Result<TicketType, DatabaseError> {
        let result: TicketType = diesel::update(&self)
            .set((
                ticket_types::status.eq(TicketTypeStatus::Cancelled),
                ticket_types::cancelled_at.eq(dsl::now.nullable()),
                ticket_types::updated_at.eq(dsl::now),
            ))
            .get_result(conn)
            .to_db_error(ErrorCode::UpdateError, "Could not cancel ticket_type")?;

        Ok(result)
    }

    pub fn delete(self, conn: &PgConnection) -> Result<(), DatabaseError> {
        diesel::update(&self)
            .set((
                ticket_types::deleted_at.eq(dsl::now.nullable()),
                ticket_types::updated_at.eq(dsl::now),
            ))
            .execute(conn)
            .to_db_error(ErrorCode::UpdateError, "Could not delete ticket_type")?;

        Ok(())
    }

    pub fn parent(&self, conn: &PgConnection) -> Result<Option<TicketType>, DatabaseError> {
        match self.parent_id {
            Some(parent_id) => Ok(Some(TicketType::find(parent_id, conn)?)),
            None => Ok(None),
        }
    }

    /// Gets the start date if it is present, or the parent's end date
    pub fn start_date(&self, conn: &PgConnection) -> Result<NaiveDateTime, DatabaseError> {
        match self.start_date {
            Some(start_date) => Ok(start_date),
            None => Ok(cmp::min(
                TicketType::find(
                    self.parent_id.ok_or_else(|| {
                        return DatabaseError::business_process_error::<Uuid>(
                            "Ticket type must have a start date or start after another ticket type",
                        )
                        .unwrap_err();
                    })?,
                    conn,
                )?
                .end_date(conn)?,
                self.end_date(conn)?,
            )),
        }
    }

    pub fn end_date(&self, conn: &PgConnection) -> Result<NaiveDateTime, DatabaseError> {
        if let Some(end_date) = self.end_date {
            Ok(end_date)
        } else {
            let event = self.event(conn)?;
            let end_date = match self.end_date_type {
                TicketTypeEndDateType::Manual => {
                    return DatabaseError::business_process_error::<NaiveDateTime>(
                        "Manual ticket type end date must have value",
                    );
                }
                TicketTypeEndDateType::EventStart => event.event_start,
                TicketTypeEndDateType::EventEnd => event.event_end,
                TicketTypeEndDateType::DoorTime => event.door_time,
            };

            if end_date.is_none() {
                return DatabaseError::business_process_error("Could not fetch end date for ticket type from event");
            }

            Ok(end_date.unwrap())
        }
    }

    pub fn find_for_code(code_id: Uuid, conn: &PgConnection) -> Result<Vec<TicketType>, DatabaseError> {
        ticket_types::table
            .inner_join(ticket_type_codes::table.on(ticket_type_codes::ticket_type_id.eq(ticket_types::id)))
            .filter(ticket_type_codes::code_id.eq(code_id))
            .select(ticket_types::all_columns)
            .order_by(ticket_types::rank)
            .then_order_by(ticket_types::name)
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Could not find ticket types for code")
    }

    pub fn find(id: Uuid, conn: &PgConnection) -> Result<TicketType, DatabaseError> {
        ticket_types::table
            .filter(ticket_types::id.eq(id))
            .get_result(conn)
            .to_db_error(ErrorCode::QueryError, "Could not find ticket type")
    }

    pub fn validate_record(
        &self,
        attributes: &mut TicketTypeEditableAttributes,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        if attributes.end_date_type.unwrap_or(self.end_date_type) == TicketTypeEndDateType::Manual
            && (attributes.end_date == Some(None) || (attributes.end_date.is_none() && self.end_date.is_none()))
        {
            return Ok(validators::append_validation_error(
                Ok(()),
                "end_date",
                Err(create_validation_error(
                    "required",
                    "End date required for manual end date type",
                )),
            )?);
        }

        let new_end_date = attributes.end_date.unwrap_or(None).unwrap_or(self.end_date(conn)?);
        let new_parent_id = attributes.parent_id.unwrap_or(self.parent_id);

        if attributes.start_date.is_some() || attributes.parent_id.is_some() {
            let new_start_date = attributes.start_date.unwrap_or(self.start_date);

            // If there is a parent id, start date must be null
            if let Some(parent_id) = new_parent_id {
                if new_start_date.is_some() {
                    return Ok(validators::simple_error(
                        "start_date",
                        "Start date cannot be specified if the ticket type is set to start after another ticket type",
                    )?);
                }
                // Otherwise the end date of the parent must be before the end date
                else {
                    return Ok(validators::append_validation_error(
                        Ok(()),
                        "start_date",
                        validators::start_date_valid(TicketType::find(parent_id, conn)?.end_date(conn)?, new_end_date),
                    )?);
                }
            } else {
                // otherwise if a start date is not provided, it starts immediately
                if attributes.start_date == Some(None) {
                    attributes.start_date = Some(Some(times::zero()));
                }
            }
        }

        // if either the start or end date is set, we must validate for range
        if attributes.end_date.is_some() || attributes.start_date.is_some() {
            let new_start_date = attributes.start_date.unwrap_or(self.start_date);
            return Ok(validators::append_validation_error(
                Ok(()),
                "start_date",
                validators::start_date_valid(new_start_date.unwrap_or(self.start_date(conn)?), new_end_date),
            )?);
        }

        Ok(())
    }

    pub fn validate_ticket_pricing(&self, conn: &PgConnection) -> Result<(), DatabaseError> {
        let mut validation_errors: Result<(), ValidationErrors> = Ok(());

        // Default pricing is not included in validation
        for ticket_pricing in self.ticket_pricing(false, conn)? {
            validation_errors = validators::append_validation_error(
                validation_errors,
                "ticket_pricing",
                TicketPricing::ticket_pricing_no_overlapping_periods(
                    ticket_pricing.id,
                    self.id,
                    ticket_pricing.start_date,
                    ticket_pricing.end_date,
                    ticket_pricing.is_box_office_only,
                    ticket_pricing.status,
                    conn,
                )?,
            );
            validation_errors = validators::append_validation_error(
                validation_errors,
                "ticket_pricing.start_date",
                TicketPricing::ticket_pricing_does_not_overlap_ticket_type_start_date(self, ticket_pricing.start_date)?,
            );
            validation_errors = validators::append_validation_error(
                validation_errors,
                "ticket_pricing.end_date",
                TicketPricing::ticket_pricing_does_not_overlap_ticket_type_end_date(
                    self,
                    ticket_pricing.end_date,
                    conn,
                )?,
            );
        }

        Ok(validation_errors?)
    }

    pub fn check_for_sold_out_triggers(
        &self,
        current_user_id: Option<Uuid>,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        if self.valid_available_ticket_count(conn)? > 0 {
            return Ok(());
        }

        // Find child ticket types
        for child in self.find_dependent_ticket_types(conn)? {
            if child.start_date.is_none() {
                child.start_sales(current_user_id, conn)?;
            }
        }

        // Ideally we want to track this event, but there's a chance that this can be
        // abused to insert a lot of these events in the db, so might re-enable it at
        // a later stage.
        //        DomainEvent::create(
        //            DomainEventTypes::TicketTypeSoldOut,
        //            format!("Ticket type '{}' has sold out", self.name),
        //            Tables::TicketTypes,
        //            Some(self.id),
        //            None,
        //            None,
        //        )
        //        .commit(conn)?;

        Ok(())
    }

    pub fn start_sales(self, current_user_id: Option<Uuid>, conn: &PgConnection) -> Result<(), DatabaseError> {
        if self.start_date.unwrap_or(times::infinity()) > times::now() {
            DomainEvent::create(
                DomainEventTypes::TicketTypeSalesStarted,
                format!("Ticket sales have started for '{}'", self.name),
                Tables::TicketTypes,
                Some(self.id),
                current_user_id,
                None,
            )
            .commit(conn)?;

            let pricings = self.ticket_pricing(true, conn)?;
            let pricing = pricings
                .into_iter()
                .sorted_by(|a, b| Ord::cmp(&a.start_date, &b.start_date))
                .pop();
            match pricing {
                Some(price) => price.start_sales(current_user_id, conn)?,
                None => (),
            }
        }

        Ok(())
    }
    pub fn find_dependent_ticket_types(&self, conn: &PgConnection) -> Result<Vec<TicketType>, DatabaseError> {
        ticket_types::table
            .filter(ticket_types::parent_id.eq(Some(self.id)))
            .load(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load dependent ticket types")
    }

    pub fn find_by_event_id(
        event_id: Uuid,
        filter_access_tokens: bool,
        redemption_code: Option<String>,
        conn: &PgConnection,
    ) -> Result<Vec<TicketType>, DatabaseError> {
        if filter_access_tokens {
            let query = r#"
                    SELECT DISTINCT tt.*
                    FROM ticket_types tt
                    LEFT JOIN (
                        SELECT ttc.ticket_type_id, c.redemption_code, c.start_date, c.end_date
                        FROM ticket_type_codes ttc
                        JOIN codes c ON ttc.code_id = c.id
                        JOIN ticket_types tt ON tt.id = ttc.ticket_type_id
                        WHERE c.code_type = 'Access' AND tt.event_id = $1 AND c.deleted_at IS NULL
                    ) ttc ON ttc.ticket_type_id = tt.id
                    WHERE tt.event_id = $1
                    AND tt.deleted_at is null
                    AND (
                        ttc.redemption_code is null
                        OR (
                            ttc.redemption_code = $2 and ttc.start_date <= now() and ttc.end_date >= now()
                        )
                    )
                    ORDER BY tt.rank, tt.name
                    "#;

            diesel::sql_query(query)
                .bind::<dUuid, _>(event_id)
                .bind::<Nullable<Text>, _>(redemption_code)
                .load(conn)
                .to_db_error(ErrorCode::QueryError, "Could not load ticket types for event")
        } else {
            ticket_types::table
                .filter(ticket_types::event_id.eq(event_id))
                .order_by(ticket_types::rank)
                .then_order_by(ticket_types::name)
                .load(conn)
                .to_db_error(ErrorCode::QueryError, "Could not load ticket types for event")
        }
    }

    pub fn ticket_count(&self, conn: &PgConnection) -> Result<u32, DatabaseError> {
        let valid_ticket_count: i64 = ticket_instances::table
            .inner_join(assets::table)
            .filter(assets::ticket_type_id.eq(self.id))
            .select(dsl::count(ticket_instances::id))
            .first(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load ticket count for ticket type")?;
        Ok(valid_ticket_count as u32)
    }

    pub fn valid_ticket_count(&self, conn: &PgConnection) -> Result<u32, DatabaseError> {
        let valid_ticket_count: i64 = ticket_instances::table
            .inner_join(assets::table)
            .filter(assets::ticket_type_id.eq(self.id))
            .filter(ticket_instances::status.ne(TicketInstanceStatus::Nullified))
            .select(dsl::count(ticket_instances::id))
            .first(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load ticket count for ticket type")?;
        Ok(valid_ticket_count as u32)
    }

    pub fn valid_unsold_ticket_count(&self, conn: &PgConnection) -> Result<u32, DatabaseError> {
        let valid_unsold_ticket_count: i64 = ticket_instances::table
            .inner_join(assets::table)
            .filter(assets::ticket_type_id.eq(self.id))
            .filter(
                ticket_instances::status.eq_any(vec![TicketInstanceStatus::Available, TicketInstanceStatus::Reserved]),
            )
            .select(dsl::count(ticket_instances::id))
            .first(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load ticket count for ticket type")?;
        Ok(valid_unsold_ticket_count as u32)
    }

    pub fn valid_sold_and_reserved_ticket_count(&self, conn: &PgConnection) -> Result<u32, DatabaseError> {
        let valid_unsold_ticket_count: i64 = ticket_instances::table
            .inner_join(assets::table)
            .filter(assets::ticket_type_id.eq(self.id))
            .filter(ticket_instances::status.eq_any(vec![
                TicketInstanceStatus::Purchased,
                TicketInstanceStatus::Reserved,
                TicketInstanceStatus::Redeemed,
            ]))
            .select(dsl::count(ticket_instances::id))
            .first(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load ticket count for ticket type")?;
        Ok(valid_unsold_ticket_count as u32)
    }

    pub fn valid_available_ticket_count(&self, conn: &PgConnection) -> Result<u32, DatabaseError> {
        let query = ticket_instances::table
            .inner_join(assets::table)
            .filter(
                assets::ticket_type_id.eq(self.id).and(
                    ticket_instances::status
                        .eq(TicketInstanceStatus::Available)
                        .or(sql("(ticket_instances.status=")
                            .bind::<Text, _>(TicketInstanceStatus::Reserved)
                            .sql(" AND ticket_instances.reserved_until < CURRENT_TIMESTAMP)")),
                ),
            )
            .filter(ticket_instances::hold_id.is_null())
            .select(dsl::count(ticket_instances::id));

        let valid_available_ticket_count: i64 = query
            .first(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load ticket count for ticket type")?;
        Ok(valid_available_ticket_count as u32)
    }

    pub fn current_ticket_pricing(
        &self,
        box_office_pricing: bool,
        conn: &PgConnection,
    ) -> Result<TicketPricing, DatabaseError> {
        TicketPricing::get_current_ticket_pricing(self.id, box_office_pricing, false, conn)
    }

    pub fn ticket_pricing(
        &self,
        include_default: bool,
        conn: &PgConnection,
    ) -> Result<Vec<TicketPricing>, DatabaseError> {
        let mut query = ticket_pricing::table
            .filter(ticket_pricing::ticket_type_id.eq(self.id))
            .into_boxed();

        if include_default {
            query = query.filter(
                ticket_pricing::status
                    .eq(TicketPricingStatus::Published)
                    .or(ticket_pricing::status.eq(TicketPricingStatus::Default)),
            );
        } else {
            query = query.filter(ticket_pricing::status.eq(TicketPricingStatus::Published));
        }
        query
            .order_by(ticket_pricing::name)
            .load(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load ticket pricing for ticket type")
    }

    pub fn is_event_available_for_sale(ticket_type_id: &Uuid, conn: &PgConnection) -> Result<bool, DatabaseError> {
        let valid_ticket_count: i64 = ticket_types::table
            .inner_join(events::table)
            .filter(ticket_types::id.eq(ticket_type_id))
            .filter(events::status.ne(EventStatus::Draft.to_string()))
            .filter(events::cancelled_at.is_null())
            .filter(events::deleted_at.is_null())
            .filter(
                events::event_end
                    .is_null()
                    .or(events::event_end.ge(dsl::now.nullable())),
            )
            .filter(ticket_types::status.ne(TicketTypeStatus::Cancelled.to_string()))
            .select(dsl::count(ticket_types::id))
            .first(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load ticket pricing for ticket type")?;
        if valid_ticket_count <= 0 {
            return Ok(false);
        }
        Ok(true)
    }

    pub fn valid_ticket_pricing(
        &self,
        include_default: bool,
        conn: &PgConnection,
    ) -> Result<Vec<TicketPricing>, DatabaseError> {
        let mut query = ticket_pricing::table
            .filter(ticket_pricing::ticket_type_id.eq(self.id))
            .filter(ticket_pricing::status.ne(TicketPricingStatus::Deleted))
            .order_by(ticket_pricing::name)
            .into_boxed();
        if !include_default {
            query = query.filter(ticket_pricing::status.ne(TicketPricingStatus::Default));
        }
        query
            .load(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load ticket pricing for ticket type")
    }

    pub fn add_ticket_pricing(
        &self,
        name: String,
        start_date: NaiveDateTime,
        end_date: NaiveDateTime,
        price_in_cents: i64,
        is_box_office_only: bool,
        status: Option<TicketPricingStatus>,
        current_user_id: Option<Uuid>,
        conn: &PgConnection,
    ) -> Result<TicketPricing, DatabaseError> {
        let desc = format!("Ticket pricing '{}' added", &name);
        let result = TicketPricing::create(
            self.id,
            name,
            start_date,
            end_date,
            price_in_cents,
            is_box_office_only,
            status,
            None,
        )
        .commit(current_user_id, conn)?;

        DomainEvent::create(
            DomainEventTypes::TicketPricingAdded,
            desc,
            Tables::TicketTypes,
            Some(self.id),
            current_user_id,
            Some(json!({"ticket_pricing_id": result.id})),
        )
        .commit(conn)?;
        Ok(result)
    }

    pub fn shift_other_rank(&self, from_rank: i32, conn: &PgConnection) -> Result<(), DatabaseError> {
        if self.rank == from_rank {
            return Ok(());
        }
        let min_rank = cmp::min(self.rank, from_rank);
        let max_rank = cmp::max(self.rank, from_rank);

        let ticket_types: Vec<TicketType> = ticket_types::table
            .filter(ticket_types::event_id.eq(self.event_id))
            .filter(ticket_types::rank.ge(min_rank))
            .filter(ticket_types::rank.le(max_rank))
            .filter(ticket_types::id.ne(self.id))
            .order_by(ticket_types::rank.asc())
            .load(conn)
            .to_db_error(ErrorCode::QueryError, "Could not update ticket type ranks")?;

        // Update other ticket types ranks
        for other_ticket_type in ticket_types {
            let new_rank = if self.rank > from_rank {
                other_ticket_type.rank - 1
            } else {
                other_ticket_type.rank + 1
            };
            other_ticket_type.update_rank_only(new_rank, conn)?;
        }

        Ok(())
    }
}

#[derive(Insertable)]
#[table_name = "ticket_types"]
pub struct NewTicketType {
    pub event_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub status: TicketTypeStatus,
    pub start_date: Option<NaiveDateTime>,
    pub end_date: Option<NaiveDateTime>,
    pub increment: Option<i32>,
    pub limit_per_person: i32,
    pub price_in_cents: i64,
    pub visibility: TicketTypeVisibility,
    pub parent_id: Option<Uuid>,
    pub additional_fee_in_cents: i64,
    pub end_date_type: TicketTypeEndDateType,
    pub app_sales_enabled: bool,
    pub web_sales_enabled: bool,
    pub ticket_type_type: TicketTypeType,
    pub box_office_sales_enabled: bool,
    pub rarity_id: Option<Uuid>,
    pub promo_image_url: Option<String>,
    pub content_url: Option<String>,
}

impl NewTicketType {
    pub fn commit(mut self, current_user_id: Option<Uuid>, conn: &PgConnection) -> Result<TicketType, DatabaseError> {
        self.validate_record(conn)?;

        // Clear end date if not manual as value is ignored
        if self.end_date_type != TicketTypeEndDateType::Manual {
            self.end_date = None;
        }
        let rank: Option<i32> = ticket_types::table
            .filter(ticket_types::event_id.eq(self.event_id))
            .select(dsl::max(ticket_types::rank))
            .first(conn)
            .to_db_error(ErrorCode::QueryError, "Could not find the correct rank for ticket type")?;

        let result: TicketType = diesel::insert_into(ticket_types::table)
            .values(&self)
            .get_result(conn)
            .to_db_error(ErrorCode::InsertError, "Could not create ticket type")?;

        let result: TicketType = diesel::update(&result)
            .set(ticket_types::rank.eq(rank.unwrap_or(-1) + 1))
            .get_result(conn)
            .to_db_error(ErrorCode::UpdateError, "Could not update rank of ticket type")?;

        DomainEvent::create(
            DomainEventTypes::TicketTypeCreated,
            format!("Ticket type '{}' created", &result.name),
            Tables::TicketTypes,
            Some(result.id),
            current_user_id,
            Some(json!(&result)),
        )
        .commit(conn)?;

        result.add_ticket_pricing(
            result.name.clone(),
            result.start_date(conn)?,
            result.end_date(conn)?,
            result.price_in_cents,
            false,
            Some(TicketPricingStatus::Default),
            current_user_id,
            conn,
        )?;

        Ok(result)
    }

    pub fn validate_record(&self, conn: &PgConnection) -> Result<(), DatabaseError> {
        if self.end_date_type == TicketTypeEndDateType::Manual && self.end_date.is_none() {
            return Ok(validators::append_validation_error(
                Ok(()),
                "end_date",
                Err(create_validation_error(
                    "required",
                    "End date required for manual end date type",
                )),
            )?);
        }

        let validation_errors = if self.parent_id.is_some() {
            Ok(())
        } else {
            validators::append_validation_error(
                Ok(()),
                "start_date",
                validators::start_date_valid(self.start_date(conn)?, self.end_date(conn)?),
            )
        };
        let validation_errors = validators::append_validation_error(
            validation_errors,
            "additional_fee_in_cents",
            validators::validate_greater_than_or_equal(
                self.additional_fee_in_cents,
                0,
                "additional_fee_in_cents_lt_0",
                "Additional fee cannot be negative",
            ),
        );
        let org = Organization::find_for_event(self.event_id, conn)?;

        let validation_errors = validators::append_validation_error(
            validation_errors,
            "additional_fee_in_cents",
            validators::validate_less_than_or_equal(
                self.additional_fee_in_cents,
                org.max_additional_fee_in_cents,
                "additional_fee_in_cents_gt_max",
                "Additional fee above the maximum allowed amount",
            ),
        );

        Ok(validation_errors?)
    }

    pub fn end_date(&self, conn: &PgConnection) -> Result<NaiveDateTime, DatabaseError> {
        if let Some(end_date) = self.end_date {
            Ok(end_date)
        } else {
            let event = Event::find(self.event_id, conn)?;
            let end_date = match self.end_date_type {
                TicketTypeEndDateType::Manual => {
                    return DatabaseError::business_process_error::<NaiveDateTime>(
                        "Manual ticket type end date must have value",
                    );
                }
                TicketTypeEndDateType::EventStart => event.event_start,
                TicketTypeEndDateType::EventEnd => event.event_end,
                TicketTypeEndDateType::DoorTime => event.door_time,
            };

            if end_date.is_none() {
                return DatabaseError::business_process_error("Could not fetch end date for ticket type from event");
            }

            Ok(end_date.unwrap())
        }
    }

    /// Gets the start date if it is present, or the parent's end date
    pub fn start_date(&self, conn: &PgConnection) -> Result<NaiveDateTime, DatabaseError> {
        match self.start_date {
            Some(start_date) => Ok(start_date),
            None =>
                Ok(cmp::min(TicketType::find(
                    self.parent_id.ok_or_else(|| {
                        return DatabaseError::business_process_error::<Uuid>(
                            "Could not create ticket type, it must have a start date or start after another ticket type",
                        ).unwrap_err();
                    })?,
                    conn,
                )?.end_date(conn)?, self.end_date(conn)?))
        }
    }
}
