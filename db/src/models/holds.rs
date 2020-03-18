use chrono::prelude::*;
use diesel;
use diesel::dsl::{self, sql};
use diesel::prelude::*;
use diesel::sql_types::BigInt;
use models::*;
use schema::holds;
use std::borrow::Cow;
use utils::errors::*;
use uuid::Uuid;
use validator::Validate;
use validator::*;
use validators::{self, *};

#[derive(Clone, Deserialize, Identifiable, Queryable, Serialize, PartialEq, Debug)]
pub struct Hold {
    pub id: Uuid,
    pub name: String,
    pub parent_hold_id: Option<Uuid>,
    pub event_id: Uuid,
    pub redemption_code: Option<String>,
    pub discount_in_cents: Option<i64>,
    pub end_at: Option<NaiveDateTime>,
    pub max_per_user: Option<i64>,
    pub hold_type: HoldTypes,
    pub ticket_type_id: Uuid,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub deleted_at: Option<NaiveDateTime>,
}

#[derive(AsChangeset, Default, Validate)]
#[table_name = "holds"]
pub struct UpdateHoldAttributes {
    pub name: Option<String>,
    pub redemption_code: Option<Option<String>>,
    pub hold_type: Option<HoldTypes>,
    pub discount_in_cents: Option<Option<i64>>,
    #[validate(email(message = "Email is invalid"))]
    pub email: Option<Option<String>>,
    pub phone: Option<Option<String>>,
    pub end_at: Option<Option<NaiveDateTime>>,
    pub max_per_user: Option<Option<i64>>,
}

impl Hold {
    /// Constructor for creating a simple hold that is accessible via a redemption code. For
    /// creating a comp, use `create_comp_for_person`.
    pub fn create_hold(
        name: String,
        event_id: Uuid,
        redemption_code: Option<String>,
        discount_in_cents: Option<u32>,
        end_at: Option<NaiveDateTime>,
        max_per_user: Option<u32>,
        hold_type: HoldTypes,
        ticket_type_id: Uuid,
    ) -> NewHold {
        NewHold {
            name,
            parent_hold_id: None,
            event_id,
            email: None,
            phone: None,
            redemption_code: redemption_code.map(|r| r.to_uppercase()),
            discount_in_cents: discount_in_cents.and_then(|discount| Some(discount as i64)),
            end_at,
            max_per_user: max_per_user.map(|m| m as i64),
            hold_type,
            ticket_type_id,
        }
    }

    /// Creates a `quantity` of tickets in a hold (comp) for a person. `name` should
    /// be the name of the person, but does not have to be. For creating a simple hold,
    /// use `create_hold`.
    pub fn create_comp_for_person(
        name: String,
        current_user_id: Option<Uuid>,
        hold_id: Uuid,
        email: Option<String>,
        phone: Option<String>,
        redemption_code: String,
        end_at: Option<NaiveDateTime>,
        max_per_user: Option<u32>,
        quantity: u32,
        conn: &PgConnection,
    ) -> Result<Hold, DatabaseError> {
        let hold = Hold::find(hold_id, conn)?;

        let new_hold = NewHold {
            name,
            parent_hold_id: Some(hold_id),
            event_id: hold.event_id,
            email,
            phone,
            redemption_code: Some(redemption_code.to_uppercase()),
            discount_in_cents: None,
            end_at,
            max_per_user: max_per_user.map(|m| m as i64),
            hold_type: HoldTypes::Comp,
            ticket_type_id: hold.ticket_type_id,
        };

        let new_hold = new_hold.commit(current_user_id, conn)?;

        new_hold.set_quantity(current_user_id, quantity, conn)?;

        Ok(new_hold)
    }

    /// Updates a hold. Note, the quantity in the hold must be updated using
    /// `set_quantity`.
    pub fn update(&self, update_attrs: UpdateHoldAttributes, conn: &PgConnection) -> Result<Hold, DatabaseError> {
        let mut update_attrs = update_attrs;
        if update_attrs.hold_type == Some(HoldTypes::Comp) {
            // Remove discount
            update_attrs.discount_in_cents = Some(None);
        }

        self.validate_record(&update_attrs, conn)?;

        let updated_hold: Hold = diesel::update(
            holds::table
                .filter(holds::id.eq(self.id))
                .filter(holds::updated_at.eq(self.updated_at)),
        )
        .set((update_attrs, holds::updated_at.eq(dsl::now)))
        .get_result(conn)
        .to_db_error(ErrorCode::UpdateError, "Could not update hold")?;

        if updated_hold.end_at != self.end_at {
            updated_hold.update_automatic_clear_domain_action(conn)?;
        }

        Ok(updated_hold)
    }

    pub fn update_automatic_clear_domain_action(&self, conn: &PgConnection) -> Result<(), DatabaseError> {
        match self.end_at {
            Some(end_at) => {
                let now = Utc::now().naive_utc();
                let run_time = if end_at < now { now } else { end_at };

                match DomainAction::upcoming_domain_action(
                    Some(Tables::Holds),
                    Some(self.id),
                    DomainActionTypes::ReleaseHoldInventory,
                    conn,
                )? {
                    Some(action) => {
                        action.set_scheduled_at(run_time, conn)?;
                    }
                    None => {
                        let mut action = DomainAction::create(
                            None,
                            DomainActionTypes::ReleaseHoldInventory,
                            None,
                            json!({}),
                            Some(Tables::Holds),
                            Some(self.id),
                        );
                        action.schedule_at(run_time);
                        action.commit(conn)?;
                    }
                }
            }
            None => {
                // Does not end, check for a domain action and cancel it, else do nothing
                if let Some(action) =
                    DomainAction::upcoming_domain_action(None, None, DomainActionTypes::ReleaseHoldInventory, conn)?
                {
                    action.set_cancelled(conn)?;
                }
            }
        }

        Ok(())
    }

    pub fn find(id: Uuid, conn: &PgConnection) -> Result<Hold, DatabaseError> {
        holds::table
            .filter(holds::id.eq(id))
            .first(conn)
            .to_db_error(ErrorCode::QueryError, "Could not retrieve hold")
    }

    pub fn confirm_hold_valid(&self) -> Result<(), DatabaseError> {
        if let Some(end_at) = self.end_at {
            let now = Utc::now().naive_utc();
            if now > end_at {
                let mut errors = ValidationErrors::new();
                let mut validation_error = create_validation_error("invalid", "Hold not valid for current datetime");
                validation_error.add_param(Cow::from("hold_id"), &self.id);
                validation_error.add_param(Cow::from("end_at"), &self.end_at);
                errors.add("hold_id", validation_error);
                return Err(errors.into());
            }
        }
        Ok(())
    }

    pub fn purchased_ticket_count(&self, user: &User, conn: &PgConnection) -> Result<i64, DatabaseError> {
        use schema::*;
        holds::table
            .left_join(order_items::table.on(order_items::hold_id.eq(holds::id.nullable())))
            .left_join(orders::table.on(orders::id.eq(order_items::order_id)))
            .filter(
                orders::on_behalf_of_user_id
                    .eq(user.id)
                    .or(orders::on_behalf_of_user_id.is_null().and(orders::user_id.eq(user.id))),
            )
            .filter(orders::status.eq(OrderStatus::Paid))
            .select(sql::<BigInt>("CAST(COALESCE(SUM(order_items.quantity), 0) AS BIGINT)"))
            .first(conn)
            .to_db_error(
                ErrorCode::QueryError,
                "Could not check hold purchased ticket count for user",
            )
    }

    pub fn find_by_parent_id(
        parent_id: Uuid,
        hold_type: Option<HoldTypes>,
        page: u32,
        limit: u32,
        conn: &PgConnection,
    ) -> Result<Payload<Hold>, DatabaseError> {
        let total: i64 = holds::table
            .filter(holds::parent_hold_id.eq(parent_id))
            .filter(holds::deleted_at.is_null())
            .filter(holds::hold_type.nullable().eq(hold_type).or(hold_type.is_none()))
            .count()
            .first(conn)
            .to_db_error(ErrorCode::QueryError, "Could not get total holds for parent hold")?;

        let paging = Paging::new(page, limit);
        let mut payload = Payload::new(
            holds::table
                .filter(holds::parent_hold_id.eq(parent_id))
                .filter(holds::deleted_at.is_null())
                .filter(holds::hold_type.nullable().eq(hold_type).or(hold_type.is_none()))
                .order_by((holds::hold_type, holds::name))
                .limit(limit as i64)
                .offset((page * limit) as i64)
                .load(conn)
                .to_db_error(ErrorCode::QueryError, "Could not retrieve holds")?,
            paging,
        );

        // TODO: remove this when other structs implement paging
        payload.paging.total = total as u64;
        payload.paging.page = page;
        payload.paging.limit = limit;
        Ok(payload)
    }

    pub fn all(conn: &PgConnection) -> Result<Vec<Hold>, DatabaseError> {
        holds::table
            .order_by(holds::name.asc())
            .load(conn)
            .to_db_error(ErrorCode::QueryError, "Could not retrieve holds")
    }

    pub fn find_for_event(
        event_id: Uuid,
        include_children: bool,
        conn: &PgConnection,
    ) -> Result<Vec<Hold>, DatabaseError> {
        let mut query = holds::table
            .filter(holds::event_id.eq(event_id))
            .filter(holds::deleted_at.is_null())
            .into_boxed();

        if !include_children {
            query = query.filter(holds::parent_hold_id.is_null());
        }

        query
            .order_by(holds::name.asc())
            .load(conn)
            .to_db_error(ErrorCode::QueryError, "Could not retrieve holds for event")
    }

    fn validate_record(&self, update_attrs: &UpdateHoldAttributes, conn: &PgConnection) -> Result<(), DatabaseError> {
        let mut validation_errors = validators::append_validation_error(
            update_attrs.validate(),
            "discount_in_cents",
            Hold::discount_in_cents_valid(
                update_attrs.hold_type.clone().unwrap_or(self.hold_type.clone()),
                update_attrs.discount_in_cents.unwrap_or(self.discount_in_cents),
            ),
        );
        if let Some(ref redemption_code_field) = update_attrs.redemption_code {
            if let Some(ref redemption_code) = redemption_code_field {
                validation_errors = validators::append_validation_error(
                    validation_errors,
                    "redemption_code",
                    redemption_code_unique_per_event_validation(
                        Some(self.id),
                        "holds".into(),
                        redemption_code.clone(),
                        self.event_id,
                        conn,
                    )?,
                );
            }
        }

        Ok(validation_errors?)
    }

    /// Creates a new hold, dividing the quantity in the calling hold (`self`). This is done so that
    /// there isn't a time where the tickets return to the main pool, or the parent hold and are
    /// reserved in between creation of the new hold. Note that the new hold created will have the
    /// same `parent_hold_id` as `self` (the original hold) and not `self`, as might be expected.
    /// If tickets are released from this new hold, or the original hold, they will be returned
    /// to the parent hold, being either the main pool or the parent hold in `parent_hold_id`.
    pub fn split(
        &self,
        current_user_id: Option<Uuid>,
        name: String,
        email: Option<String>,
        phone: Option<String>,
        redemption_code: String,
        quantity: u32,
        discount_in_cents: Option<u32>,
        hold_type: HoldTypes,
        end_at: Option<NaiveDateTime>,
        max_per_user: Option<u32>,
        child: bool,
        conn: &PgConnection,
    ) -> Result<Hold, DatabaseError> {
        let new_hold = NewHold {
            name,
            parent_hold_id: if child { Some(self.id) } else { self.parent_hold_id },
            event_id: self.event_id,
            email,
            phone,
            redemption_code: Some(redemption_code.to_uppercase()),
            discount_in_cents: discount_in_cents.map(|m| m as i64),
            end_at,
            max_per_user: max_per_user.map(|m| m as i64),
            hold_type,
            ticket_type_id: self.ticket_type_id,
        };

        let new_hold = new_hold.commit(current_user_id, conn)?;

        TicketInstance::add_to_hold(
            current_user_id,
            new_hold.id,
            self.ticket_type_id,
            quantity,
            Some(self.id),
            conn,
        )?;
        Ok(new_hold)
    }

    /// Deletes a hold by first setting the quantity to 0 and then deleting the record.
    pub fn destroy(self, current_user_id: Option<Uuid>, conn: &PgConnection) -> Result<(), DatabaseError> {
        self.set_quantity(current_user_id, 0, conn)?;

        diesel::update(holds::table.filter(holds::id.eq(self.id)))
            .set((holds::deleted_at.eq(dsl::now), holds::updated_at.eq(dsl::now)))
            .execute(conn)
            .to_db_error(ErrorCode::DeleteError, "Could not delete hold")?;

        DomainEvent::create(
            DomainEventTypes::HoldDeleted,
            format!("Hold  {} deleted", self.name),
            Tables::Holds,
            Some(self.id),
            current_user_id,
            None,
        )
        .commit(conn)?;

        Ok(())
    }

    pub fn discount_in_cents_valid(
        hold_type: HoldTypes,
        discount_in_cents: Option<i64>,
    ) -> Result<(), ValidationError> {
        if hold_type == HoldTypes::Discount && discount_in_cents.is_none() {
            let validation_error = create_validation_error("required", "Discount required for hold type Discount");
            return Err(validation_error);
        }

        Ok(())
    }

    /// Changes the quantity of tickets reserved in this hold. If the quantity is
    /// higher, it will attempt to reserve more tickets from either the main pool,
    /// or from the parent hold if `parent_hold_id` is not `None`. Likewise, if the
    /// quantity is lower, it will release the reserved tickets back to either the
    /// main pool or the parent hold.
    pub fn set_quantity(&self, user_id: Option<Uuid>, quantity: u32, conn: &PgConnection) -> Result<(), DatabaseError> {
        let (count, _available) = self.quantity(conn)?;
        if count < quantity {
            TicketInstance::add_to_hold(
                user_id,
                self.id,
                self.ticket_type_id,
                quantity - count,
                self.parent_hold_id,
                conn,
            )?;
            DomainEvent::create(
                DomainEventTypes::HoldQuantityChanged,
                format!("Hold quantity increased from {} to {}", count, quantity),
                Tables::Holds,
                Some(self.id),
                user_id,
                Some(json!({"old_quantity": count, "new_quantity": quantity})),
            )
            .commit(conn)?;
        }
        if count > quantity {
            if self.parent_hold_id.is_some() {
                TicketInstance::add_to_hold(
                    user_id,
                    self.parent_hold_id.unwrap(),
                    self.ticket_type_id,
                    count - quantity,
                    Some(self.id),
                    conn,
                )?;
                DomainEvent::create(
                    DomainEventTypes::HoldQuantityChanged,
                    format!("Hold quantity decreased from {} to {}, returned to parent hold", count, quantity),
                    Tables::Holds,
                    Some(self.id),
                    user_id,
                    Some(json!({"old_quantity": count, "new_quantity": quantity, "parent_hold_id": &self.parent_hold_id})),
                )
                    .commit(conn)?;
            } else {
                TicketInstance::release_from_hold(user_id, self.id, self.ticket_type_id, count - quantity, conn)?;
                DomainEvent::create(
                    DomainEventTypes::HoldQuantityChanged,
                    format!("Hold quantity decreased from {} to {}", count, quantity),
                    Tables::Holds,
                    Some(self.id),
                    user_id,
                    Some(json!({"old_quantity": count, "new_quantity": quantity})),
                )
                .commit(conn)?;
            }
        }

        Ok(())
    }

    pub fn remove_available_quantity(
        &self,
        current_user_id: Option<Uuid>,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        // Recursively remove from children
        let children: Vec<Hold> = holds::table
            .filter(holds::parent_hold_id.eq(self.id))
            .load(conn)
            .to_db_error(ErrorCode::QueryError, "Could not find children for hold")?;
        for child in children {
            child.remove_available_quantity(current_user_id, conn)?;
        }

        // Children have returned their quantity to the parent so returning remaining available ticket inventory
        let (total, remaining) = self.quantity(conn)?;
        let sold_quantity = total - remaining;
        self.set_quantity(current_user_id, sold_quantity, conn)?;

        Ok(())
    }

    pub fn quantity(&self, conn: &PgConnection) -> Result<(u32, u32), DatabaseError> {
        TicketInstance::count_for_hold(self.id, self.ticket_type_id, false, conn)
    }

    pub fn children_quantity(&self, conn: &PgConnection) -> Result<(u32, u32), DatabaseError> {
        let (total_quantity, total_available) =
            TicketInstance::count_for_hold(self.id, self.ticket_type_id, true, conn)?;
        let (hold_quantity, hold_available) =
            TicketInstance::count_for_hold(self.id, self.ticket_type_id, false, conn)?;
        Ok((total_quantity - hold_quantity, total_available - hold_available))
    }

    pub fn event(&self, conn: &PgConnection) -> Result<Event, DatabaseError> {
        use schema::*;
        events::table
            .filter(events::id.eq(self.event_id))
            .first(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load event for code")
    }

    pub fn organization(&self, conn: &PgConnection) -> Result<Organization, DatabaseError> {
        use schema::*;
        events::table
            .inner_join(organizations::table)
            .filter(events::id.eq(self.event_id))
            .select(organizations::all_columns)
            .first(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load organization for hold")
    }

    pub fn find_by_redemption_code(
        redemption_code: &str,
        event_id: Option<Uuid>,
        conn: &PgConnection,
    ) -> Result<Hold, DatabaseError> {
        let mut query = holds::table
            .filter(holds::redemption_code.eq(redemption_code.to_uppercase()))
            .filter(holds::deleted_at.is_null())
            .into_boxed();
        if let Some(e) = event_id {
            query = query.filter(holds::event_id.eq(e))
        }

        query
            .first(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load hold with that redeem key")
    }

    pub fn find_by_ticket_type(ticket_type_id: Uuid, conn: &PgConnection) -> Result<Vec<Hold>, DatabaseError> {
        holds::table
            .filter(holds::ticket_type_id.eq(ticket_type_id))
            .load(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load holds by ticket type")
    }

    pub fn into_display(self, conn: &PgConnection) -> Result<DisplayHold, DatabaseError> {
        let (quantity, available) = self.quantity(conn)?;

        Ok(DisplayHold {
            id: self.id,
            name: self.name,
            parent_hold_id: self.parent_hold_id,
            event_id: self.event_id,
            redemption_code: self.redemption_code,
            discount_in_cents: self.discount_in_cents,
            max_per_user: self.max_per_user,
            email: self.email,
            phone: self.phone,
            hold_type: self.hold_type,
            available,
            quantity,
        })
    }

    pub fn comps(&self, conn: &PgConnection) -> Result<Vec<Hold>, DatabaseError> {
        Ok(Hold::find_by_parent_id(self.id, Some(HoldTypes::Comp), 0, 100000, conn)?.data)
    }
}

#[derive(Insertable, Validate, Serialize)]
#[table_name = "holds"]
pub struct NewHold {
    pub name: String,
    pub parent_hold_id: Option<Uuid>,
    pub event_id: Uuid,
    #[validate(email(message = "Email is invalid"))]
    pub email: Option<String>,
    pub phone: Option<String>,
    pub redemption_code: Option<String>,
    pub discount_in_cents: Option<i64>,
    pub end_at: Option<NaiveDateTime>,
    pub max_per_user: Option<i64>,
    pub hold_type: HoldTypes,
    pub ticket_type_id: Uuid,
}

impl NewHold {
    pub fn commit(mut self, current_user_id: Option<Uuid>, conn: &PgConnection) -> Result<Hold, DatabaseError> {
        if self.hold_type == HoldTypes::Comp {
            self.discount_in_cents = None
        }
        self.validate_record(conn)?;
        let hold: Hold = diesel::insert_into(holds::table)
            .values(&self)
            .get_result(conn)
            .to_db_error(ErrorCode::InsertError, "Could not create hold")?;
        DomainEvent::create(
            DomainEventTypes::HoldCreated,
            format!("Hold {} created", self.name),
            Tables::Holds,
            Some(hold.id),
            current_user_id,
            Some(json!(&hold)),
        )
        .commit(conn)?;

        hold.update_automatic_clear_domain_action(conn)?;

        Ok(hold)
    }

    fn validate_record(&self, conn: &PgConnection) -> Result<(), DatabaseError> {
        let mut validation_errors = validators::append_validation_error(
            self.validate(),
            "discount_in_cents",
            Hold::discount_in_cents_valid(self.hold_type.clone(), self.discount_in_cents),
        );
        if let Some(ref redemption_code) = self.redemption_code {
            validation_errors = validators::append_validation_error(
                validation_errors,
                "redemption_code",
                redemption_code_unique_per_event_validation(
                    None,
                    "holds".into(),
                    redemption_code.clone(),
                    self.event_id,
                    conn,
                )?,
            );
        }

        Ok(validation_errors?)
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct DisplayHold {
    pub id: Uuid,
    pub parent_hold_id: Option<Uuid>,
    pub hold_type: HoldTypes,
    pub name: String,
    pub event_id: Uuid,
    pub redemption_code: Option<String>,
    pub discount_in_cents: Option<i64>,
    pub max_per_user: Option<i64>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub available: u32,
    pub quantity: u32,
}
