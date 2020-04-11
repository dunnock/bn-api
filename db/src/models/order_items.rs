use chrono::prelude::*;
use diesel;
use diesel::dsl::{self, select};
use diesel::prelude::*;
use diesel::sql_types::{Array, BigInt, Nullable, Text, Uuid as dUuid};
use itertools::Itertools;
use models::*;
use schema::{codes, events, order_items, ticket_instances, ticket_types};
use std::borrow::Cow;
use std::cmp;
use std::collections::HashMap;
use utils::errors::*;
use uuid::Uuid;
use validator::*;
use validators::{self, *};

sql_function!(fn order_items_quantity_in_increments(item_type: Text, quantity: BigInt, ticket_pricing_id: Nullable<dUuid>) -> Bool);
sql_function!(fn order_items_ticket_type_id_valid_for_access_code(ticket_type_id: dUuid, code_id: Nullable<dUuid>) -> Bool);

#[derive(Identifiable, Associations, Queryable, QueryableByName, AsChangeset)]
#[belongs_to(Order)]
#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
#[table_name = "order_items"]
pub struct OrderItem {
    pub id: Uuid,
    pub order_id: Uuid,
    pub item_type: OrderItemTypes,
    pub ticket_type_id: Option<Uuid>,
    pub event_id: Option<Uuid>,
    pub quantity: i64,
    pub unit_price_in_cents: i64,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub ticket_pricing_id: Option<Uuid>,
    pub fee_schedule_range_id: Option<Uuid>,
    pub parent_id: Option<Uuid>,
    pub hold_id: Option<Uuid>,
    pub code_id: Option<Uuid>,
    pub company_fee_in_cents: i64,
    pub client_fee_in_cents: i64,
    pub refunded_quantity: i64,
}

impl OrderItem {
    pub fn event(&self, conn: &PgConnection) -> Result<Event, DatabaseError> {
        events::table
            .filter(events::id.nullable().eq(self.event_id))
            .first::<EventData>(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load event for order item")
            .map(Event::from)
    }

    pub fn find_fee_item(&self, conn: &PgConnection) -> Result<Option<OrderItem>, DatabaseError> {
        order_items::table
            .filter(order_items::parent_id.eq(self.id))
            .filter(order_items::item_type.eq(OrderItemTypes::PerUnitFees))
            .first(conn)
            .optional()
            .to_db_error(ErrorCode::QueryError, "Could not retrieve order item fees")
    }

    pub fn find_discount_item(&self, conn: &PgConnection) -> Result<Option<OrderItem>, DatabaseError> {
        order_items::table
            .filter(order_items::parent_id.eq(self.id))
            .filter(order_items::item_type.eq(OrderItemTypes::Discount))
            .first(conn)
            .optional()
            .to_db_error(ErrorCode::QueryError, "Could not retrieve order item discount")
    }

    pub fn description(&self, conn: &PgConnection) -> Result<String, DatabaseError> {
        use models::OrderItemTypes::*;
        let res = match self.item_type {
            PerUnitFees => "Ticket Fees".to_string(),
            EventFees => {
                let ticket_type = self.ticket_type(conn)?;
                match ticket_type {
                    Some(t) => format!("Event Fees - {}", t.event(conn)?.name),
                    None => "Event Fees".to_string(),
                }
            }
            Discount => "Discount".to_string(),
            CreditCardFees => "Credit Card Fees".to_string(),
            _ => {
                let ticket_type = self.ticket_type(conn)?;
                match ticket_type {
                    Some(t) => format!("{} - {}", t.event(conn)?.name, t.name),
                    None => "Other".to_string(),
                }
            }
        };
        Ok(res)
    }

    pub(crate) fn refund_one_unit(&mut self, refund_fees: bool, conn: &PgConnection) -> Result<i64, DatabaseError> {
        if self.order(conn)?.status != OrderStatus::Paid {
            return DatabaseError::business_process_error("Order item must have associated paid order to refund unit");
        }

        if self.refunded_quantity == self.quantity {
            return DatabaseError::business_process_error(
                "Order item refund failed as requested refund quantity exceeds remaining quantity",
            );
        }

        self.refunded_quantity += 1;

        // Check if any discounts exist for this order_item
        let discount = self.find_discount_item(conn)?;
        let discount_amount;
        if let Some(mut oi) = discount {
            discount_amount = oi.refund_one_unit(true, conn)?
        } else {
            discount_amount = 0;
        }

        let mut refund_amount_in_cents = self.unit_price_in_cents + discount_amount;
        // Refund fees if ticket is being refunded
        if refund_fees && self.item_type == OrderItemTypes::Tickets {
            let fee_item = self.find_fee_item(conn)?;
            if let Some(mut fee_item) = fee_item {
                refund_amount_in_cents += fee_item.refund_one_unit(true, conn)?;
            }
        }

        diesel::update(order_items::table.filter(order_items::id.eq(self.id)))
            .set((
                order_items::updated_at.eq(dsl::now),
                order_items::refunded_quantity.eq(self.refunded_quantity),
            ))
            .execute(conn)
            .to_db_error(ErrorCode::UpdateError, "Could not refund ticket instance")?;
        Ok(refund_amount_in_cents)
    }

    pub fn code(&self, conn: &PgConnection) -> Result<Option<Code>, DatabaseError> {
        match self.code_id {
            Some(code_id) => codes::table
                .filter(codes::id.eq(code_id))
                .first(conn)
                .optional()
                .to_db_error(ErrorCode::QueryError, "Could not retrieve code"),
            _ => return Ok(None),
        }
    }

    pub fn confirm_code_valid(&self, conn: &PgConnection) -> Result<(), DatabaseError> {
        if let Some(code) = self.code(conn)? {
            code.confirm_code_valid()?;
        }

        Ok(())
    }

    pub(crate) fn update_discount(&self, order: &Order, conn: &PgConnection) -> Result<(), DatabaseError> {
        if self.item_type == OrderItemTypes::PerUnitFees
            || self.item_type == OrderItemTypes::EventFees
            || self.item_type == OrderItemTypes::Discount
            || self.item_type == OrderItemTypes::CreditCardFees
        {
            return Ok(());
        }

        let discount_item = self.find_discount_item(conn)?;

        if let Some(code_id) = self.code_id {
            let code = Code::find(code_id, conn)?;
            let mut discount = 0;
            if let Some(discount_percent) = code.discount_as_percentage {
                discount = cmp::min(
                    ((self.unit_price_in_cents as f32) * (discount_percent as f32) / 100.0f32) as i64,
                    self.unit_price_in_cents,
                );
            } else if let Some(discount_in_cents) = code.discount_in_cents {
                discount = cmp::min(discount_in_cents, self.unit_price_in_cents);
            }
            if discount > 0 {
                if let Some(mut di) = discount_item {
                    di.quantity = self.quantity;
                    di.unit_price_in_cents = -discount;
                    di.update(conn)?;
                } else {
                    NewDiscountOrderItem {
                        order_id: self.order_id,
                        item_type: OrderItemTypes::Discount,
                        event_id: self.event_id,
                        quantity: self.quantity,
                        unit_price_in_cents: -discount,
                        company_fee_in_cents: 0,
                        client_fee_in_cents: 0,
                        parent_id: Some(self.id),
                    }
                    .commit(conn)?;
                }
            } else {
                if let Some(di) = discount_item {
                    order.destroy_item(di.id, conn)?;
                }
            }
        } else if let Some(hold_id) = self.hold_id {
            let h = Hold::find(hold_id, conn)?;

            let hold_type = h.hold_type;
            let discount = match hold_type {
                HoldTypes::Discount => cmp::min(h.discount_in_cents.unwrap_or(0), self.unit_price_in_cents),
                HoldTypes::Comp => self.unit_price_in_cents,
            };
            if let Some(mut di) = discount_item {
                di.quantity = self.quantity;
                di.unit_price_in_cents = -discount;
                di.update(conn)?;
            } else {
                NewDiscountOrderItem {
                    order_id: self.order_id,
                    item_type: OrderItemTypes::Discount,
                    event_id: self.event_id,
                    quantity: self.quantity,
                    unit_price_in_cents: -discount,
                    company_fee_in_cents: 0,
                    client_fee_in_cents: 0,
                    parent_id: Some(self.id),
                }
                .commit(conn)?;
            }
        } else if let Some(di) = discount_item {
            order.destroy_item(di.id, conn)?;
        }

        Ok(())
    }

    pub(crate) fn update_fees(&self, order: &Order, conn: &PgConnection) -> Result<(), DatabaseError> {
        if self.item_type != OrderItemTypes::Tickets {
            return Ok(());
        }

        let fee_item = self.find_fee_item(conn)?;

        let ticket_type = match self.ticket_type_id {
            Some(ticket_type_id) => TicketType::find(ticket_type_id, conn)?,
            None => {
                return DatabaseError::no_results("Order item does not have a valid ticket type");
            }
        };

        let fee_schedule_ranges = ticket_type.fee_schedule(conn)?.ranges(conn)?;

        let discount_item = self.find_discount_item(conn)?;

        let unit_price_with_discount = match discount_item {
            Some(di) => self.unit_price_in_cents + di.unit_price_in_cents,
            None => self.unit_price_in_cents,
        };

        if fee_schedule_ranges.len() > 0 && unit_price_with_discount >= fee_schedule_ranges[0].min_price_in_cents {
            let fee_schedule_range = ticket_type
                .fee_schedule(conn)?
                .get_range(unit_price_with_discount, conn)?;

            // If the hold is a comp, then there are no fees.
            if let Some(hold_id) = self.hold_id {
                let hold = Hold::find(hold_id, conn)?;
                if hold.hold_type == HoldTypes::Comp {
                    if let Some(fee_item) = fee_item {
                        order.destroy_item(fee_item.id, conn)?;
                    }
                    return Ok(());
                }
            }

            match fee_item {
                Some(mut fee_item) => {
                    fee_item.quantity = self.quantity;
                    fee_item.unit_price_in_cents =
                        fee_schedule_range.fee_in_cents + ticket_type.additional_fee_in_cents;
                    fee_item.company_fee_in_cents = fee_schedule_range.company_fee_in_cents;
                    fee_item.client_fee_in_cents =
                        fee_schedule_range.client_fee_in_cents + ticket_type.additional_fee_in_cents;
                    fee_item.update(conn)
                }
                None => {
                    NewFeesOrderItem {
                        order_id: self.order_id,
                        item_type: OrderItemTypes::PerUnitFees,
                        event_id: self.event_id,
                        unit_price_in_cents: fee_schedule_range.fee_in_cents + ticket_type.additional_fee_in_cents,
                        fee_schedule_range_id: Some(fee_schedule_range.id),
                        company_fee_in_cents: fee_schedule_range.company_fee_in_cents,
                        client_fee_in_cents: fee_schedule_range.client_fee_in_cents
                            + ticket_type.additional_fee_in_cents,
                        quantity: self.quantity,
                        parent_id: Some(self.id),
                    }
                    .commit(conn)?;

                    Ok(())
                }
            }
        } else {
            if let Some(fee_item) = fee_item {
                order.destroy_item(fee_item.id, conn)?;
            }
            Ok(())
        }
    }

    pub(crate) fn update(&self, conn: &PgConnection) -> Result<(), DatabaseError> {
        self.validate_record(conn)?;
        diesel::update(self)
            .set((
                order_items::quantity.eq(self.quantity),
                order_items::unit_price_in_cents.eq(self.unit_price_in_cents),
                order_items::updated_at.eq(dsl::now),
            ))
            .execute(conn)
            .map(|_| ())
            .to_db_error(ErrorCode::UpdateError, "Could not update order item")
    }

    pub fn order(&self, conn: &PgConnection) -> Result<Order, DatabaseError> {
        Order::find(self.order_id, conn)
    }

    pub fn calculate_quantity(&self, conn: &PgConnection) -> Result<i64, DatabaseError> {
        ticket_instances::table
            .filter(ticket_instances::order_item_id.eq(self.id))
            //.filter(ticket_instances::reserved_until.ge(dsl::now.nullable()))
            .select(dsl::count(ticket_instances::id))
            .first(conn)
            .to_db_error(ErrorCode::QueryError, "Could calculate order item quantity")
    }

    pub fn validate_record(&self, conn: &PgConnection) -> Result<(), DatabaseError> {
        let validation_errors = validators::append_validation_error(
            Ok(()),
            "quantity",
            OrderItem::quantity_valid_increment(
                false,
                self.item_type.clone(),
                self.quantity,
                self.ticket_pricing_id,
                conn,
            )?,
        );
        let validation_errors = validators::append_validation_error(
            validation_errors,
            "quantity",
            OrderItem::code_id_max_uses_valid(self.order_id, self.code_id, conn)?,
        );
        Ok(validation_errors?)
    }

    pub fn ticket_type(&self, conn: &PgConnection) -> Result<Option<TicketType>, DatabaseError> {
        Ok(match self.ticket_type_id {
            Some(ticket_type_id) => ticket_types::table
                .filter(ticket_types::id.eq(ticket_type_id))
                .first(conn)
                .optional()
                .to_db_error(ErrorCode::QueryError, "Unable to load ticket type")?,
            None => None,
        })
    }

    fn ticket_type_id_valid_for_access_code(
        ticket_type_id: Uuid,
        code_id: Option<Uuid>,
        conn: &PgConnection,
    ) -> Result<Result<(), ValidationError>, DatabaseError> {
        let result = select(order_items_ticket_type_id_valid_for_access_code(
            ticket_type_id,
            code_id,
        ))
        .get_result::<bool>(conn)
        .to_db_error(
            ErrorCode::InsertError,
            "Could not confirm if ticket type valid without access code",
        )?;
        if !result {
            let mut validation_error = create_validation_error(
                "ticket_type_requires_access_code",
                "Ticket type requires access code for purchase",
            );
            validation_error.add_param(Cow::from("ticket_type_id"), &ticket_type_id);
            validation_error.add_param(Cow::from("code_id"), &code_id);

            return Ok(Err(validation_error));
        }
        Ok(Ok(()))
    }

    fn code_id_max_uses_valid(
        order_id: Uuid,
        code_id: Option<Uuid>,
        conn: &PgConnection,
    ) -> Result<Result<(), ValidationError>, DatabaseError> {
        match code_id {
            None => return Ok(Ok(())),
            Some(code_id) => {
                let code = Code::find(code_id, conn)?;
                let uses = Code::find_number_of_uses(code_id, Some(order_id), conn)?;
                if code.max_uses > 0 && code.max_uses < uses + 1 {
                    let mut validation_error =
                        create_validation_error("max_uses_reached", "Redemption code maximum uses limit exceeded");
                    validation_error.add_param(Cow::from("order_id"), &order_id);
                    validation_error.add_param(Cow::from("code_id"), &code_id);
                    return Ok(Err(validation_error));
                }
                Ok(Ok(()))
            }
        }
    }

    fn quantity_valid_increment(
        new_record: bool,
        item_type: OrderItemTypes,
        quantity: i64,
        ticket_pricing_id: Option<Uuid>,
        conn: &PgConnection,
    ) -> Result<Result<(), ValidationError>, DatabaseError> {
        if item_type != OrderItemTypes::Tickets {
            return Ok(Ok(()));
        }
        let result = select(order_items_quantity_in_increments(
            item_type,
            quantity,
            ticket_pricing_id,
        ))
        .get_result::<bool>(conn)
        .to_db_error(
            if new_record {
                ErrorCode::InsertError
            } else {
                ErrorCode::UpdateError
            },
            "Could not confirm quantity increment valid",
        )?;
        if !result {
            let mut validation_error = create_validation_error(
                "quantity_invalid_increment",
                "Order item quantity invalid for ticket pricing increment",
            );
            validation_error.add_param(Cow::from("ticket_pricing_id"), &ticket_pricing_id);
            validation_error.add_param(Cow::from("quantity"), &quantity);
            return Ok(Err(validation_error));
        }
        Ok(Ok(()))
    }

    pub(crate) fn find_for_display(
        order_ids: Vec<Uuid>,
        organization_ids: Option<Vec<Uuid>>,
        user_id: Uuid,
        conn: &PgConnection,
    ) -> Result<HashMap<Uuid, Vec<DisplayOrderItem>>, DatabaseError> {
        #[derive(Queryable, QueryableByName)]
        struct R {
            #[sql_type = "dUuid"]
            id: Uuid,
            #[sql_type = "Nullable<dUuid>"]
            parent_id: Option<Uuid>,
            #[sql_type = "Nullable<dUuid>"]
            ticket_type_id: Option<Uuid>,
            #[sql_type = "Nullable<dUuid>"]
            ticket_pricing_id: Option<Uuid>,
            #[sql_type = "BigInt"]
            quantity: i64,
            #[sql_type = "BigInt"]
            refunded_quantity: i64,
            #[sql_type = "BigInt"]
            unit_price_in_cents: i64,
            #[sql_type = "Text"]
            item_type: OrderItemTypes,
            #[sql_type = "Text"]
            description: String,
            #[sql_type = "Nullable<Text>"]
            redemption_code: Option<String>,
            #[sql_type = "Nullable<Text>"]
            cart_item_status: Option<CartItemStatus>,
            #[sql_type = "dUuid"]
            event_id: Uuid,
            #[sql_type = "dUuid"]
            order_id: Uuid,
        }

        let results: Vec<R> = diesel::sql_query(
            r#"
        SELECT
           oi.id,
           oi.parent_id,
           tt.id                      AS ticket_type_id,
           tp.id                      AS ticket_pricing_id,
           oi.quantity,
           oi.refunded_quantity,
           oi.unit_price_in_cents,
           oi.item_type,
           CASE
             WHEN item_type = 'PerUnitFees' THEN 'Ticket Fees'
             WHEN item_type = 'EventFees' THEN 'Event Fees - ' || e.name
             WHEN item_type = 'Discount' THEN 'Discount'
             WHEN item_type = 'CreditCardFees' THEN 'Credit Card Fees'
             ELSE e.name || ' - ' || tt.name
           END AS description,
           COALESCE(h.redemption_code, c.redemption_code) as redemption_code,
           CASE
             -- Null prevents serialization
             WHEN o.status <> 'Draft' THEN null
             WHEN item_type <> 'Tickets' THEN 'Valid'
             WHEN ti.status = 'Nullified' THEN 'TicketNullified'
             WHEN oit.count <> oi.quantity OR ti.reserved_until < now() THEN 'TicketNotReserved'
             WHEN c.id IS NOT NULL AND c.end_date < now() THEN 'CodeExpired'
             WHEN h.id IS NOT NULL AND h.end_at < now() THEN 'HoldExpired'
             ELSE 'Valid'
           END AS cart_item_status,
           e.id AS event_id,
           oi.order_id
        FROM order_items oi
           JOIN orders o ON oi.order_id = o.id
           LEFT JOIN ticket_pricing tp ON tp.id = oi.ticket_pricing_id
           LEFT JOIN events e ON oi.event_id = e.id
           LEFT JOIN users u on u.id = $3
           LEFT JOIN organization_users ou ON ou.organization_id = e.organization_id and ou.user_id = $3
           LEFT JOIN ticket_types tt ON tp.ticket_type_id = tt.id
           LEFT JOIN holds h ON oi.hold_id = h.id
           LEFT JOIN event_users ep ON u.id = ep.user_id and ep.event_id = e.id
           LEFT JOIN ticket_instances ti ON ti.id = (
               SELECT ti.id
               FROM ticket_instances ti
               WHERE ti.order_item_id = oi.id
               -- Only join on one ticket instance, order by Nullified, reserved_until
               -- First issue with the order item will result in the cart item status affecting the order item
               ORDER BY ti.status <> 'Nullified', ti.reserved_until
               LIMIT 1
           )
           LEFT JOIN codes c ON oi.code_id = c.id
           LEFT JOIN (
               SELECT count(ti.id) as count, oi.id
               FROM order_items oi
               LEFT JOIN ticket_instances ti ON oi.id = ti.order_item_id
               WHERE oi.order_id = ANY($1)
               GROUP BY oi.id
           ) oit on oit.id = oi.id
        WHERE oi.order_id = ANY($1)
        -- Filter by organization id if list provided otherwise do not filter
        AND (
            e.id is NULL
            OR e.organization_id = ANY($2)
            OR $2 IS NULL
        )
        AND (
            'Admin' = ANY(u.role)
            OR 'Super' = ANY(u.role)
            OR o.user_id = $3
            OR o.on_behalf_of_user_id = $3
            OR (NOT 'Promoter' = ANY(ou.role))
            OR ep.id is not null
        )
        ORDER BY oi.order_id, tt.name, oi.item_type DESC
        "#,
        )
        .bind::<Array<dUuid>, _>(order_ids)
        .bind::<Nullable<Array<dUuid>>, _>(organization_ids)
        .bind::<dUuid, _>(user_id)
        .load(conn)
        .to_db_error(ErrorCode::QueryError, "Could not load order items")?;

        let mut order_items: HashMap<Uuid, Vec<DisplayOrderItem>> = HashMap::new();
        for (order_id, items) in &results.into_iter().group_by(|oi| oi.order_id) {
            let mut display_items = Vec::new();
            for item in items {
                display_items.push(DisplayOrderItem {
                    id: item.id,
                    parent_id: item.parent_id,
                    ticket_type_id: item.ticket_type_id,
                    ticket_pricing_id: item.ticket_pricing_id,
                    quantity: item.quantity,
                    refunded_quantity: item.refunded_quantity,
                    unit_price_in_cents: item.unit_price_in_cents,
                    item_type: item.item_type,
                    description: item.description,
                    redemption_code: item.redemption_code,
                    cart_item_status: item.cart_item_status,
                    event_id: item.event_id,
                });
            }
            order_items.insert(order_id, display_items);
        }

        Ok(order_items)
    }

    pub fn find_for_order(order_id: Uuid, conn: &PgConnection) -> Result<Vec<OrderItem>, DatabaseError> {
        order_items::table
            .left_join(ticket_types::table)
            .filter(order_items::order_id.eq(order_id))
            .order_by(order_items::event_id.asc())
            .then_order_by(ticket_types::rank.asc())
            .then_order_by(ticket_types::name.asc())
            .then_order_by(order_items::item_type.asc())
            .select(order_items::all_columns)
            .load(conn)
            .to_db_error(ErrorCode::QueryError, "Could not find order items for order")
    }

    pub fn find(order_item_id: Uuid, conn: &PgConnection) -> Result<OrderItem, DatabaseError> {
        order_items::table
            .filter(order_items::id.eq(order_item_id))
            .first(conn)
            .to_db_error(ErrorCode::QueryError, "Could not retrieve order item")
    }

    pub fn find_in_order(order_id: Uuid, order_item_id: Uuid, conn: &PgConnection) -> Result<OrderItem, DatabaseError> {
        order_items::table
            .filter(order_items::order_id.eq(order_id))
            .filter(order_items::id.eq(order_item_id))
            .filter(order_items::item_type.eq(OrderItemTypes::Tickets))
            .first(conn)
            .to_db_error(ErrorCode::QueryError, "Could not retrieve order item")
    }
}

#[derive(Insertable, Serialize, Deserialize, PartialEq, Debug)]
#[table_name = "order_items"]
pub(crate) struct NewTicketsOrderItem {
    pub order_id: Uuid,
    pub item_type: OrderItemTypes,
    pub event_id: Option<Uuid>,
    pub quantity: i64,
    pub unit_price_in_cents: i64,
    pub ticket_type_id: Uuid,
    pub ticket_pricing_id: Uuid,
    pub hold_id: Option<Uuid>,
    pub code_id: Option<Uuid>,
}

impl NewTicketsOrderItem {
    pub(crate) fn commit(self, conn: &PgConnection) -> Result<OrderItem, DatabaseError> {
        self.validate_record(conn)?;
        diesel::insert_into(order_items::table)
            .values(self)
            .get_result(conn)
            .to_db_error(ErrorCode::InsertError, "Could not create order item")
    }

    pub fn validate_record(&self, conn: &PgConnection) -> Result<(), DatabaseError> {
        let mut validation_errors = validators::append_validation_error(
            Ok(()),
            "quantity",
            OrderItem::quantity_valid_increment(
                true,
                self.item_type.clone(),
                self.quantity,
                Some(self.ticket_pricing_id),
                conn,
            )?,
        );
        validation_errors = validators::append_validation_error(
            validation_errors,
            "quantity",
            OrderItem::code_id_max_uses_valid(self.order_id, self.code_id, conn)?,
        );
        validation_errors = validators::append_validation_error(
            validation_errors,
            "code_id",
            OrderItem::ticket_type_id_valid_for_access_code(self.ticket_type_id, self.code_id, conn)?,
        );
        Ok(validation_errors?)
    }
}

#[derive(Insertable, Serialize, Deserialize, PartialEq, Debug)]
#[table_name = "order_items"]
pub(crate) struct NewFeesOrderItem {
    pub order_id: Uuid,
    pub item_type: OrderItemTypes,
    pub event_id: Option<Uuid>,
    pub quantity: i64,
    pub fee_schedule_range_id: Option<Uuid>,
    pub unit_price_in_cents: i64,
    pub company_fee_in_cents: i64,
    pub client_fee_in_cents: i64,
    pub parent_id: Option<Uuid>,
}

impl NewFeesOrderItem {
    pub(crate) fn commit(self, conn: &PgConnection) -> Result<OrderItem, DatabaseError> {
        diesel::insert_into(order_items::table)
            .values(self)
            .get_result(conn)
            .to_db_error(ErrorCode::InsertError, "Could not create order item")
    }
}

#[derive(Insertable, Serialize, Deserialize, PartialEq, Debug)]
#[table_name = "order_items"]
pub(crate) struct NewDiscountOrderItem {
    pub order_id: Uuid,
    pub item_type: OrderItemTypes,
    pub event_id: Option<Uuid>,
    pub quantity: i64,
    pub unit_price_in_cents: i64,
    pub company_fee_in_cents: i64,
    pub client_fee_in_cents: i64,
    pub parent_id: Option<Uuid>,
}

impl NewDiscountOrderItem {
    pub(crate) fn commit(self, conn: &PgConnection) -> Result<OrderItem, DatabaseError> {
        diesel::insert_into(order_items::table)
            .values(self)
            .get_result(conn)
            .to_db_error(ErrorCode::InsertError, "Could not create order item")
    }
}

#[derive(Deserialize, Queryable, QueryableByName, Serialize)]
pub struct DisplayOrderItem {
    #[sql_type = "dUuid"]
    pub id: Uuid,
    #[sql_type = "Nullable<dUuid>"]
    pub parent_id: Option<Uuid>,
    #[sql_type = "Nullable<dUuid>"]
    pub ticket_type_id: Option<Uuid>,
    #[sql_type = "Nullable<dUuid>"]
    pub ticket_pricing_id: Option<Uuid>,
    #[sql_type = "BigInt"]
    pub quantity: i64,
    #[sql_type = "BigInt"]
    pub refunded_quantity: i64,
    #[sql_type = "BigInt"]
    pub unit_price_in_cents: i64,
    #[sql_type = "Text"]
    pub item_type: OrderItemTypes,
    #[sql_type = "Text"]
    pub description: String,
    #[sql_type = "Nullable<Text>"]
    pub redemption_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(skip_deserializing)]
    #[sql_type = "Nullable<Text>"]
    pub cart_item_status: Option<CartItemStatus>,
    #[sql_type = "dUuid"]
    pub event_id: Uuid,
}
