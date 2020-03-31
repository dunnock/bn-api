use chrono::prelude::*;
use diesel;
use diesel::dsl::{self, sql};
use diesel::prelude::*;
use diesel::sql_types::{Array, BigInt, Nullable, Text, Timestamp, Uuid as dUuid};
use models::*;
use schema::{codes, order_items, orders};
use std::borrow::Cow;
use test::times;
use utils::errors::*;
use uuid::Uuid;
use validator::*;
use validators::{self, *};

#[derive(Clone, Debug, Deserialize, Identifiable, PartialEq, Queryable, Serialize)]
pub struct Code {
    pub id: Uuid,
    pub name: String,
    pub event_id: Uuid,
    pub code_type: CodeTypes,
    pub redemption_code: String,
    pub max_uses: i64,
    pub discount_in_cents: Option<i64>,
    pub start_date: NaiveDateTime,
    pub end_date: NaiveDateTime,
    pub max_tickets_per_user: Option<i64>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub discount_as_percentage: Option<i64>,
    pub deleted_at: Option<NaiveDateTime>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CodeAvailability {
    #[serde(flatten)]
    pub code: Code,
    pub available: Option<i64>,
}

#[derive(Debug, Deserialize, PartialEq, QueryableByName, Serialize)]
pub struct DisplayCode {
    #[sql_type = "dUuid"]
    pub id: Uuid,
    #[sql_type = "Text"]
    pub name: String,
    #[sql_type = "dUuid"]
    pub event_id: Uuid,
    #[sql_type = "Text"]
    pub code_type: CodeTypes,
    #[sql_type = "Array<Text>"]
    pub redemption_codes: Vec<String>,
    #[sql_type = "BigInt"]
    pub max_uses: i64,
    #[sql_type = "Nullable<BigInt>"]
    pub discount_in_cents: Option<i64>,
    #[sql_type = "Nullable<BigInt>"]
    pub discount_as_percentage: Option<i64>,
    #[sql_type = "Nullable<Timestamp>"]
    pub start_date: Option<NaiveDateTime>,
    #[sql_type = "Nullable<Timestamp>"]
    pub end_date: Option<NaiveDateTime>,
    #[sql_type = "Nullable<BigInt>"]
    pub max_tickets_per_user: Option<i64>,
    #[sql_type = "Timestamp"]
    pub created_at: NaiveDateTime,
    #[sql_type = "Timestamp"]
    pub updated_at: NaiveDateTime,
    #[sql_type = "Array<dUuid>"]
    pub ticket_type_ids: Vec<Uuid>,
    #[sql_type = "Nullable<Timestamp>"]
    pub deleted_at: Option<NaiveDateTime>,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct DisplayCodeAvailability {
    #[serde(flatten)]
    pub display_code: DisplayCode,
    pub available: Option<i64>,
}

#[derive(AsChangeset, Debug, Default, Deserialize, Validate)]
#[table_name = "codes"]
pub struct UpdateCodeAttributes {
    pub name: Option<String>,
    pub redemption_code: Option<String>,
    pub max_uses: Option<i64>,
    pub discount_in_cents: Option<Option<i64>>,
    pub discount_as_percentage: Option<Option<i64>>,
    pub start_date: Option<NaiveDateTime>,
    pub end_date: Option<NaiveDateTime>,
    pub max_tickets_per_user: Option<Option<i64>>,
}

impl Code {
    pub fn purchased_ticket_count(&self, user: &User, conn: &PgConnection) -> Result<i64, DatabaseError> {
        codes::table
            .left_join(order_items::table.on(order_items::code_id.eq(codes::id.nullable())))
            .left_join(orders::table.on(orders::id.eq(order_items::order_id)))
            .filter(
                orders::on_behalf_of_user_id
                    .eq(user.id)
                    .or(orders::on_behalf_of_user_id.is_null().and(orders::user_id.eq(user.id))),
            )
            .filter(orders::status.eq(OrderStatus::Paid))
            .filter(codes::id.eq(self.id))
            .select(sql::<BigInt>("CAST(COALESCE(SUM(order_items.quantity), 0) AS BIGINT)"))
            .first(conn)
            .to_db_error(
                ErrorCode::QueryError,
                "Could not check code purchased ticket count for user",
            )
    }

    pub fn find_by_redemption_code_with_availability(
        redemption_code: &str,
        event_id: Option<Uuid>,
        conn: &PgConnection,
    ) -> Result<CodeAvailability, DatabaseError> {
        let code: Code = match event_id {
            Some(e) => codes::table
                .filter(codes::redemption_code.eq(redemption_code.to_uppercase()))
                .filter(codes::event_id.eq(e))
                .filter(codes::deleted_at.is_null())
                .first(conn)
                .to_db_error(ErrorCode::QueryError, "Could not load code with that redeem code")?,
            None => codes::table
                .filter(codes::redemption_code.eq(redemption_code.to_uppercase()))
                .filter(codes::deleted_at.is_null())
                .first(conn)
                .to_db_error(ErrorCode::QueryError, "Could not load code with that redeem code")?,
        };

        let available = code.available(conn)?;
        Ok(CodeAvailability { code, available })
    }

    pub fn available(&self, conn: &PgConnection) -> Result<Option<i64>, DatabaseError> {
        Code::availablity_by_code_id_max_uses(self.id, self.max_uses, conn)
    }

    // TODO: retire this method in the future after we make max_uses an optional field and refactor the logic to removex 0 == infinite behavior
    fn availablity_by_code_id_max_uses(
        id: Uuid,
        max_uses: i64,
        conn: &PgConnection,
    ) -> Result<Option<i64>, DatabaseError> {
        if max_uses == 0 {
            Ok(None)
        } else {
            Ok(Some(max_uses - Code::find_number_of_uses(id, None, conn)?))
        }
    }

    pub fn find_number_of_uses(
        code_id: Uuid,
        order_id_to_exclude: Option<Uuid>,
        conn: &PgConnection,
    ) -> Result<i64, DatabaseError> {
        order_items::table
            .inner_join(orders::table.on(order_items::order_id.eq(orders::id)))
            .filter(order_items::order_id.ne(order_id_to_exclude.unwrap_or(Uuid::nil())))
            .filter(order_items::code_id.eq(code_id))
            // Exclude any fully refunded order items
            .filter(sql("(order_items.quantity - order_items.refunded_quantity) <> 0"))
            .filter(order_items::item_type.eq(OrderItemTypes::Tickets))
            .filter(
                orders::expires_at
                    .gt(dsl::now.nullable())
                    .or(orders::status.eq(OrderStatus::Paid)),
            )
            .select(sql::<BigInt>("COALESCE(COUNT(DISTINCT orders.id), 0)"))
            .first::<i64>(conn)
            .to_db_error(ErrorCode::QueryError, "Error loading redemption code use count")
    }

    pub fn update_ticket_types(&self, ticket_type_ids: Vec<Uuid>, conn: &PgConnection) -> Result<(), DatabaseError> {
        let existing_ticket_type_ids = TicketType::find_for_code(self.id, conn)?
            .into_iter()
            .map(|tt| tt.id)
            .collect::<Vec<Uuid>>();
        let pending_deletion = existing_ticket_type_ids
            .clone()
            .into_iter()
            .filter(|id| !ticket_type_ids.contains(id))
            .collect::<Vec<Uuid>>();
        let pending_addition = ticket_type_ids
            .into_iter()
            .filter(|id| !existing_ticket_type_ids.contains(id))
            .collect::<Vec<Uuid>>();
        TicketTypeCode::destroy_multiple(self.id, pending_deletion, conn)?;

        for ticket_type_id in pending_addition {
            TicketTypeCode::create(ticket_type_id, self.id).commit(conn)?;
        }
        Ok(())
    }

    pub fn for_display(&self, conn: &PgConnection) -> Result<DisplayCodeAvailability, DatabaseError> {
        let ticket_type_ids = TicketType::find_for_code(self.id, conn)?
            .into_iter()
            .map(|tt| tt.id)
            .collect::<Vec<Uuid>>();

        let end_date;
        if self.end_date == times::infinity() {
            end_date = None;
        } else {
            end_date = Some(self.end_date);
        }

        let start_date;
        if self.start_date == times::zero() {
            start_date = None;
        } else {
            start_date = Some(self.start_date);
        }

        let display_code = DisplayCode {
            id: self.id,
            name: self.name.clone(),
            event_id: self.event_id,
            code_type: self.code_type.clone(),
            redemption_codes: vec![self.redemption_code.clone()],
            max_uses: self.max_uses,
            discount_in_cents: self.discount_in_cents,
            discount_as_percentage: self.discount_as_percentage,
            start_date,
            end_date,
            max_tickets_per_user: self.max_tickets_per_user,
            created_at: self.created_at,
            updated_at: self.updated_at,
            ticket_type_ids,
            deleted_at: None,
        };

        let available = self.available(conn)?;
        Ok(DisplayCodeAvailability {
            display_code,
            available,
        })
    }

    pub fn create(
        name: String,
        event_id: Uuid,
        code_type: CodeTypes,
        redemption_code: String,
        max_uses: u32,
        discount_in_cents: Option<u32>,
        discount_as_percentage: Option<u32>,
        start_date: NaiveDateTime,
        end_date: NaiveDateTime,
        max_tickets_per_user: Option<u32>,
    ) -> NewCode {
        NewCode {
            name,
            event_id,
            code_type,
            redemption_code,
            max_uses: max_uses as i64,
            discount_in_cents: discount_in_cents.map(|max| max as i64),
            discount_as_percentage: discount_as_percentage.map(|max| max as i64),
            start_date,
            end_date,
            max_tickets_per_user: max_tickets_per_user.map(|max| max as i64),
        }
    }

    pub fn confirm_code_valid(&self) -> Result<(), DatabaseError> {
        let now = Utc::now().naive_utc();
        if now < self.start_date || now > self.end_date {
            let mut errors = ValidationErrors::new();
            let mut validation_error = create_validation_error("invalid", "Code not valid for current datetime");
            validation_error.add_param(Cow::from("code_id"), &self.id);
            validation_error.add_param(Cow::from("start_date"), &self.start_date);
            validation_error.add_param(Cow::from("end_date"), &self.end_date);
            errors.add("code_id", validation_error);
            return Err(errors.into());
        }
        Ok(())
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
            .to_db_error(ErrorCode::QueryError, "Could not load organization for code")
    }

    pub fn find_for_event(
        event_id: Uuid,
        code_type: Option<CodeTypes>,
        conn: &PgConnection,
    ) -> Result<Vec<DisplayCodeAvailability>, DatabaseError> {
        let query = r#"
                SELECT
                    codes.id,
                    codes.name,
                    codes.event_id,
                    codes.code_type,
                    array[codes.redemption_code] as redemption_codes,
                    codes.max_uses,
                    codes.discount_in_cents,
                    codes.discount_as_percentage,
                    codes.start_date,
                    codes.end_date,
                    codes.max_tickets_per_user,
                    codes.created_at,
                    codes.updated_at,
                    ARRAY(select ticket_type_id FROM ticket_type_codes WHERE ticket_type_codes.code_id = codes.id) as ticket_type_ids,
                    codes.deleted_at
                FROM codes
                WHERE
                    codes.event_id = $1
                    AND ($2 IS NULL OR codes.code_type = $2)
                    AND codes.deleted_at IS NULL
                ORDER BY codes.name;"#;

        let display_codes: Vec<DisplayCode> = diesel::sql_query(query)
            .bind::<dUuid, _>(event_id)
            .bind::<Nullable<Text>, _>(code_type.map(|s| s.to_string()))
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Cannot find codes for event")?;

        let mut display_codes_availability = Vec::new();

        for dc in display_codes {
            let available = Code::availablity_by_code_id_max_uses(dc.id, dc.max_uses, conn)?;
            display_codes_availability.push(DisplayCodeAvailability {
                display_code: dc,
                available,
            });
        }

        Ok(display_codes_availability)
    }

    // Validate that for the Discount code type one, and only one, discount type is specified.
    pub fn single_discount_present_for_discount_type(
        code_type: CodeTypes,
        discount_in_cents: Option<i64>,
        discount_as_percentage: Option<i64>,
    ) -> Result<(), ValidationError> {
        if code_type == CodeTypes::Discount && discount_in_cents.is_none() && discount_as_percentage.is_none() {
            let mut validation_error = create_validation_error("required", "Discount required for Discount code type");
            validation_error.add_param(Cow::from("code_type"), &code_type);
            validation_error.add_param(Cow::from("discount_in_cents"), &discount_in_cents);
            validation_error.add_param(Cow::from("discount_as_percentage"), &discount_as_percentage);
            return Err(validation_error);
        }

        if code_type == CodeTypes::Discount && discount_in_cents.is_some() && discount_as_percentage.is_some() {
            let mut validation_error = create_validation_error(
                "only_single_discount_type_allowed",
                "Cannot apply more than one type of discount",
            );
            validation_error.add_param(Cow::from("code_type"), &code_type);
            validation_error.add_param(Cow::from("discount_in_cents"), &discount_in_cents);
            validation_error.add_param(Cow::from("discount_as_percentage"), &discount_as_percentage);
            return Err(validation_error);
        }

        Ok(())
    }

    fn validate_record(&self, update_attrs: &UpdateCodeAttributes, conn: &PgConnection) -> Result<(), DatabaseError> {
        let mut validation_errors = update_attrs.validate();

        validation_errors = validators::append_validation_error(
            validation_errors,
            "start_date",
            validators::start_date_valid(
                update_attrs.start_date.unwrap_or(self.start_date),
                update_attrs.end_date.unwrap_or(self.end_date),
            ),
        );

        validation_errors = validators::append_validation_error(
            validation_errors,
            "discount_in_cents",
            Code::single_discount_present_for_discount_type(
                self.code_type.clone(),
                update_attrs.discount_in_cents.unwrap_or(self.discount_in_cents),
                update_attrs
                    .discount_as_percentage
                    .unwrap_or(self.discount_as_percentage),
            ),
        );

        validation_errors = validators::append_validation_error(
            validation_errors,
            "redemption_code",
            redemption_code_unique_per_event_validation(
                Some(self.id),
                "codes".into(),
                update_attrs
                    .redemption_code
                    .clone()
                    .unwrap_or(self.redemption_code.clone()),
                self.event_id,
                conn,
            )?,
        );

        Ok(validation_errors?)
    }

    pub fn update(
        &self,
        update_attrs: UpdateCodeAttributes,
        current_user_id: Option<Uuid>,
        conn: &PgConnection,
    ) -> Result<Code, DatabaseError> {
        let mut update_attrs = update_attrs;

        if update_attrs.discount_in_cents.is_some() || update_attrs.discount_as_percentage.is_some() {
            update_attrs.discount_in_cents = Some(update_attrs.discount_in_cents.unwrap_or(None));
            update_attrs.discount_as_percentage = Some(update_attrs.discount_as_percentage.unwrap_or(None));
        }

        self.validate_record(&update_attrs, conn)?;

        let result = diesel::update(
            codes::table
                .filter(codes::id.eq(self.id))
                .filter(codes::updated_at.eq(self.updated_at)),
        )
        .set((update_attrs, codes::updated_at.eq(dsl::now)))
        .get_result(conn)
        .to_db_error(ErrorCode::UpdateError, "Could not update code")?;

        DomainEvent::create(
            DomainEventTypes::CodeUpdated,
            format!("Code  {} deleted", self.name),
            Tables::Codes,
            Some(self.id),
            current_user_id,
            Some(json!(&self)),
        )
        .commit(conn)?;

        Ok(result)
    }

    pub fn find(id: Uuid, conn: &PgConnection) -> Result<Code, DatabaseError> {
        codes::table
            .filter(codes::id.eq(id))
            .filter(codes::deleted_at.is_null())
            .first(conn)
            .to_db_error(ErrorCode::QueryError, "Could not retrieve code")
    }

    pub fn destroy(&self, current_user_id: Option<Uuid>, conn: &PgConnection) -> Result<usize, DatabaseError> {
        let result = diesel::update(codes::table.filter(codes::id.eq(self.id)))
            .set((codes::deleted_at.eq(dsl::now), codes::updated_at.eq(dsl::now)))
            .execute(conn)
            .to_db_error(ErrorCode::DeleteError, "Could not delete code")?;

        DomainEvent::create(
            DomainEventTypes::CodeDeleted,
            format!("Code  {} deleted", self.name),
            Tables::Codes,
            Some(self.id),
            current_user_id,
            Some(json!(&self)),
        )
        .commit(conn)?;

        Ok(result)
    }
}

#[derive(Deserialize, Insertable, Serialize, Validate)]
#[table_name = "codes"]
pub struct NewCode {
    pub name: String,
    pub event_id: Uuid,
    pub code_type: CodeTypes,
    pub redemption_code: String,
    pub max_uses: i64,
    pub discount_in_cents: Option<i64>,
    pub discount_as_percentage: Option<i64>,
    pub start_date: NaiveDateTime,
    pub end_date: NaiveDateTime,
    pub max_tickets_per_user: Option<i64>,
}

impl NewCode {
    pub fn commit(self, current_user_id: Option<Uuid>, conn: &PgConnection) -> Result<Code, DatabaseError> {
        self.validate_record(conn)?;

        let result: Code = diesel::insert_into(codes::table)
            .values(&self)
            .get_result(conn)
            .to_db_error(ErrorCode::InsertError, "Could not create code")?;

        DomainEvent::create(
            DomainEventTypes::CodeCreated,
            format!("Code  {} created", self.name),
            Tables::Codes,
            Some(result.id),
            current_user_id,
            Some(json!(&self)),
        )
        .commit(conn)?;

        Ok(result)
    }

    fn validate_record(&self, conn: &PgConnection) -> Result<(), DatabaseError> {
        let mut validation_errors = self.validate();

        validation_errors = validators::append_validation_error(
            validation_errors,
            "discounts",
            Code::single_discount_present_for_discount_type(
                self.code_type.clone(),
                self.discount_in_cents,
                self.discount_as_percentage,
            ),
        );
        validation_errors = validators::append_validation_error(
            validation_errors,
            "start_date",
            validators::start_date_valid(self.start_date, self.end_date),
        );
        validation_errors = validators::append_validation_error(
            validation_errors,
            "redemption_code",
            redemption_code_unique_per_event_validation(
                None,
                "codes".into(),
                self.redemption_code.clone(),
                self.event_id,
                conn,
            )?,
        );

        Ok(validation_errors?)
    }
}
