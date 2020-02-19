use chrono::prelude::*;
use chrono::Duration;
use dev::times;
use diesel;
use diesel::dsl::{exists, select};
use diesel::expression::dsl;
use diesel::expression::sql_literal::sql;
use diesel::pg::types::sql_types::Array;
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Bool, Integer, Nullable, Text, Timestamp, Uuid as dUuid};
use diesel::{sql_query, sql_types};
use itertools::Itertools;
use log::Level::{self, Debug};
use models::*;
use schema::{
    event_users, events, order_items, order_transfers, orders, organization_users, organizations, payments, refunds,
    transfers, users,
};
use serde_json;
use serde_json::Value;
use std::borrow::Cow;
use std::collections::HashMap;
use url::Url;
use utils::dates::*;
use utils::errors::*;
use utils::iterators::*;
use utils::regexes;
use uuid::Uuid;
use validator::*;
use validators::*;

pub const CART_EXPIRY_TIME_MINUTES: i64 = 15;
const ORDER_NUMBER_LENGTH: usize = 8;

#[derive(Associations, Clone, Debug, Deserialize, Identifiable, PartialEq, Queryable, QueryableByName, Serialize)]
#[table_name = "orders"]
#[belongs_to(User)]
pub struct Order {
    pub id: Uuid,
    pub user_id: Uuid,
    pub status: OrderStatus,
    pub order_type: OrderTypes,
    pub order_date: NaiveDateTime,
    pub expires_at: Option<NaiveDateTime>,
    pub version: i64,
    pub on_behalf_of_user_id: Option<Uuid>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub paid_at: Option<NaiveDateTime>,
    pub box_office_pricing: bool,
    pub checkout_url: Option<String>,
    pub checkout_url_expires: Option<NaiveDateTime>,
    pub create_user_agent: Option<String>,
    pub purchase_user_agent: Option<String>,
    pub external_payment_type: Option<ExternalPaymentType>,
    pub tracking_data: Option<serde_json::Value>,
    pub source: Option<String>,
    pub campaign: Option<String>,
    pub medium: Option<String>,
    pub term: Option<String>,
    pub content: Option<String>,
    pub platform: Option<String>,
    #[serde(skip_serializing)]
    pub settlement_id: Option<Uuid>,
    pub referrer: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RefundItemRequest {
    pub order_item_id: Uuid,
    pub ticket_instance_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, PartialEq, Queryable, QueryableByName, Serialize)]
pub struct OrderDetailsLineItem {
    #[sql_type = "Nullable<dUuid>"]
    pub ticket_instance_id: Option<Uuid>,
    #[sql_type = "dUuid"]
    pub order_item_id: Uuid,
    #[sql_type = "Text"]
    pub description: String,
    #[sql_type = "BigInt"]
    pub ticket_price_in_cents: i64,
    #[sql_type = "BigInt"]
    pub fees_price_in_cents: i64,
    #[sql_type = "BigInt"]
    pub total_price_in_cents: i64,
    #[sql_type = "Text"]
    pub status: String,
    #[sql_type = "Bool"]
    pub refundable: bool,
    #[sql_type = "Nullable<Text>"]
    pub attendee_email: Option<String>,
    #[sql_type = "Nullable<dUuid>"]
    pub attendee_id: Option<Uuid>,
    #[sql_type = "Nullable<Text>"]
    pub attendee_first_name: Option<String>,
    #[sql_type = "Nullable<Text>"]
    pub attendee_last_name: Option<String>,
    #[sql_type = "Nullable<dUuid>"]
    pub ticket_type_id: Option<Uuid>,
    #[sql_type = "Nullable<Text>"]
    pub ticket_type_name: Option<String>,
    #[sql_type = "Nullable<Text>"]
    pub code: Option<String>,
    #[sql_type = "Nullable<Text>"]
    pub code_type: Option<String>,
    #[sql_type = "Nullable<dUuid>"]
    pub pending_transfer_id: Option<Uuid>,
    #[sql_type = "Nullable<BigInt>"]
    pub discount_price_in_cents: Option<i64>,
}

#[derive(Debug)]
struct LimitCheck {
    ticket_type_id: Uuid,
    hold_id: Option<Uuid>,
    code_id: Option<Uuid>,
    limit_per_person: u32,
    redemption_code: Option<String>,
}

#[derive(Debug)]
struct MatchData<'a> {
    index: Option<usize>,
    hold_id: Option<Uuid>,
    hold: Option<Hold>,
    code_id: Option<Uuid>,
    code: Option<Code>,
    redemption_code: Option<String>,
    update_order_item: &'a UpdateOrderItem,
}

impl Order {
    pub fn retarget_abandoned_carts(conn: &PgConnection) -> Result<(Vec<Order>), DatabaseError> {
        let now = Utc::now().naive_utc();
        let beginning_of_current_hour =
            NaiveDate::from_ymd(now.year(), now.month(), now.day()).and_hms(now.hour(), 0, 0);
        let yesterday_same_hour = beginning_of_current_hour - Duration::days(1);

        let results: Vec<(Order, Uuid, Vec<Uuid>)> = orders::table
            .inner_join(order_items::table.on(orders::id.eq(order_items::order_id)))
            .inner_join(events::table.on(order_items::event_id.eq(events::id.nullable())))
            // Filter carts to only those created during that window yesterday
            .filter(orders::created_at.ge(yesterday_same_hour))
            .filter(orders::created_at.lt(yesterday_same_hour + Duration::hours(1)))
            // Limited to Draft to avoid any pending payment orders from being included
            .filter(orders::status.eq(OrderStatus::Draft))
            // Sanity check, these should have expired shortly after they were created
            .filter(orders::expires_at.lt(now))
            // Event level sanity checks
            .filter(orders::on_behalf_of_user_id.is_null())
            .filter(events::event_end.gt(now))
            .filter(events::status.eq(EventStatus::Published))
            .filter(events::cancelled_at.is_null())
            .filter(events::deleted_at.is_null())
            .filter(events::override_status.eq_any(vec![
                EventOverrideStatus::PurchaseTickets,
                EventOverrideStatus::TicketsAtTheDoor,
                EventOverrideStatus::Free
            ]).or(events::override_status.is_null()))
            .filter(order_items::item_type.eq(OrderItemTypes::Tickets))
            .distinct()
            .select((
                orders::all_columns,
                events::id,
                sql::<Array<dUuid>>("array_agg(distinct order_items.ticket_type_id)"),
            ))
            .group_by((orders::all_columns, events::id))
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Could not get abandoned carts")?;

        let mut order_event_id_map = HashMap::new();
        let mut ticket_type_ids: Vec<Uuid> = Vec::new();
        let mut abandoned_carts = Vec::new();
        for (order, event_id, mut tt_ids) in results {
            order_event_id_map.insert(order.id, event_id);
            ticket_type_ids.append(&mut tt_ids);
            abandoned_carts.push(order);
        }

        // Prefetching ticket types to avoid extra queries across orders from the same events
        ticket_type_ids.sort();
        ticket_type_ids.dedup();
        let ticket_types = TicketType::find_by_ids(&ticket_type_ids, conn)?;
        let mut ticket_types_map = HashMap::new();
        for ticket_type in ticket_types {
            ticket_types_map.insert(
                ticket_type.id,
                (ticket_type.clone(), ticket_type.valid_available_ticket_count(conn)?),
            );
        }

        let mut carts_to_retarget = Vec::new();
        for cart in abandoned_carts {
            if cart.valid_for_duplicating(Some(&ticket_types_map), conn)? {
                carts_to_retarget.push(cart.clone());
            }
        }

        // From the remaining carts, filter out users who have received a targeted email for this event in the past
        let mut user_ids: Vec<Uuid> = carts_to_retarget.iter().map(|c| c.user_id).collect();
        user_ids.sort();
        user_ids.dedup();
        #[derive(Debug, Queryable, QueryableByName)]
        struct R {
            #[sql_type = "dUuid"]
            user_id: Uuid,
            #[sql_type = "Array<dUuid>"]
            event_ids: Vec<Uuid>,
        }
        let query = r#"
            SELECT
                o.user_id,
                array_agg(distinct oi.event_id) as event_ids
            FROM
                orders o
            JOIN
                order_items oi
            ON
                oi.order_id = o.id
            JOIN
                domain_events de
            ON
                de.main_table = 'Orders'
            AND
                de.main_id = o.id
            WHERE
                de.event_type = 'OrderRetargetingEmailTriggered'
            AND
                oi.event_id is not null
            AND
                o.user_id = ANY($1)
            GROUP BY
                o.user_id;
        "#;
        let results: Vec<R> = diesel::sql_query(query)
            .bind::<Array<dUuid>, _>(user_ids)
            .get_results(conn)
            .to_db_error(
                ErrorCode::QueryError,
                "Could not check users to confirm they are eligible for retargeting",
            )?;

        let mut user_targeting_event_ids_mapping = HashMap::new();
        for result in results {
            user_targeting_event_ids_mapping.insert(result.user_id, result.event_ids);
        }
        carts_to_retarget.retain(|c| {
            if let Some(event_id) = order_event_id_map.get(&c.id) {
                if let Some(event_ids) = user_targeting_event_ids_mapping.get(&c.user_id) {
                    // Only allow a single event to send one of these once

                    if event_ids.contains(event_id) {
                        return false;
                    }
                }
                if User::is_attending_event(c.user_id, *event_id, conn)
                    .ok()
                    .unwrap_or(true)
                {
                    return false;
                }
            }
            true
        });

        // Trigger cart abandoned event
        for cart in &carts_to_retarget {
            DomainEvent::create(
                DomainEventTypes::OrderRetargetingEmailTriggered,
                "Order retargeting email triggered".to_string(),
                Tables::Orders,
                Some(cart.id),
                None,
                None,
            )
            .commit(conn)?;
        }
        Ok(carts_to_retarget)
    }

    pub fn valid_for_duplicating(
        &self,
        ticket_type_cache: Option<&HashMap<Uuid, (TicketType, u32)>>,
        conn: &PgConnection,
    ) -> Result<bool, DatabaseError> {
        let mut valid = true;
        let now = Utc::now().naive_utc();
        for item in self.items(conn)? {
            if item.item_type != OrderItemTypes::Tickets {
                continue;
            }
            if let Some(ticket_type_id) = item.ticket_type_id {
                let (ticket_type, available_quantity) = if let Some(ticket_types_map) = ticket_type_cache {
                    let (tt, q) = ticket_types_map.get(&ticket_type_id).ok_or_else(|| {
                        DatabaseError::business_process_error::<(TicketType, u32)>(
                            "Failed to load ticket type for order item",
                        )
                        .unwrap_err()
                    })?;
                    (tt.clone(), *q)
                } else {
                    let ticket_type = TicketType::find(ticket_type_id, conn)?;
                    let available_quantity = ticket_type.valid_available_ticket_count(conn)?;
                    (ticket_type, available_quantity)
                };

                if ticket_type.start_date.unwrap_or(times::infinity()) > now
                    || ticket_type.end_date(conn)? < now
                    || ticket_type.cancelled_at.is_some()
                    || ticket_type.deleted_at.is_some()
                {
                    valid = false;
                    break;
                }

                match ticket_type.status(false, conn)? {
                    // We check sold out below, other statuses lead to these not sending out
                    TicketTypeStatus::Published | TicketTypeStatus::SoldOut => (),
                    _ => {
                        valid = false;
                        break;
                    }
                }

                // Default to available quantity for non hold orders
                let mut available = available_quantity as i64;
                if let Some(code_id) = item.code_id {
                    let code = Code::find(code_id, conn)?;
                    let code_available = code.available(conn)? as i64;

                    if code_available < available {
                        available = code_available;
                    }
                } else if let Some(hold_id) = item.hold_id {
                    let hold = Hold::find(hold_id, conn)?;
                    available = hold.quantity(conn)?.1 as i64;
                }

                if available < item.quantity {
                    valid = false;
                    break;
                }
            }
        }
        Ok(valid)
    }

    pub fn duplicate_order(&self, conn: &PgConnection) -> Result<Order, DatabaseError> {
        if self.valid_for_duplicating(None, conn)? {
            let user = User::find(self.on_behalf_of_user_id.unwrap_or(self.user_id), conn)?;
            // Check if user has an unexpired cart
            if let Some(cart) = Order::find_cart_for_user(user.id, conn)? {
                if cart.items(conn)?.len() > 0 {
                    return DatabaseError::conflict_error("You already have tickets in your cart");
                }
            }

            let mut update_data = Vec::new();
            let redemption_code = self.redemption_code(conn)?;
            for item in self.items(conn)? {
                if item.item_type != OrderItemTypes::Tickets {
                    continue;
                }

                if let Some(ticket_type_id) = item.ticket_type_id {
                    update_data.push(UpdateOrderItem {
                        quantity: item.quantity as u32,
                        ticket_type_id,
                        redemption_code: redemption_code.clone(),
                    });
                }
            }

            let mut cart = Order::find_or_create_cart(&user, conn)?;
            cart.update_quantities(self.user_id, &update_data, false, true, conn)
                .map_err(|_err| {
                    DatabaseError::business_process_error::<()>("Order is invalid for duplication").unwrap_err()
                })?;
            return Ok(cart);
        } else {
            return DatabaseError::business_process_error("Order is invalid for duplication");
        }
    }

    pub fn create_next_retarget_abandoned_cart_domain_action(conn: &PgConnection) -> Result<(), DatabaseError> {
        let now = Utc::now().naive_utc();
        if let Some(upcoming_domain_action) =
            DomainAction::upcoming_domain_action(None, None, DomainActionTypes::RetargetAbandonedOrders, conn)?
        {
            if upcoming_domain_action.scheduled_at > now {
                return DatabaseError::business_process_error(
                    "Retarget abandoned cart domain action is already pending",
                );
            }
        }

        let beginning_of_current_hour =
            NaiveDate::from_ymd(now.year(), now.month(), now.day()).and_hms(now.hour(), 0, 0);
        let next_action_date = beginning_of_current_hour + Duration::hours(1);

        let mut action = DomainAction::create(
            None,
            DomainActionTypes::RetargetAbandonedOrders,
            None,
            json!({}),
            None,
            None,
        );
        action.schedule_at(next_action_date);
        action.commit(conn)?;

        Ok(())
    }

    pub fn redemption_code(&self, conn: &PgConnection) -> Result<Option<String>, DatabaseError> {
        for item in self.items(conn)? {
            if let Some(code_id) = item.code_id {
                return Ok(Some(Code::find(code_id, conn)?.redemption_code));
            }
            if let Some(hold_id) = item.hold_id {
                return Ok(Hold::find(hold_id, conn)?.redemption_code);
            }
        }

        Ok(None)
    }

    pub fn validate_record(&self, conn: &PgConnection) -> Result<(), DatabaseError> {
        let validation_errors = append_validation_error(
            Ok(()),
            "event_id",
            Order::order_contains_items_from_only_one_event(self.id, conn)?,
        );

        Ok(validation_errors?)
    }

    pub fn order_contains_items_from_only_one_event(
        id: Uuid,
        conn: &PgConnection,
    ) -> Result<Result<(), ValidationError>, DatabaseError> {
        let event_count = order_items::table
            .filter(order_items::order_id.eq(id))
            .filter(order_items::event_id.is_not_null())
            .select(sql::<BigInt>("count(distinct event_id) AS event_count"))
            .get_result::<i64>(conn)
            .to_db_error(ErrorCode::QueryError, "Could not get count of unique events in cart")?;

        if event_count > 1 {
            let mut validation_error =
                create_validation_error("cart_event_limit_reached", "You already have another event ticket in your cart. Please clear your cart first to purchase tickets to this event.");
            validation_error.add_param(Cow::from("order_id"), &id);
            return Ok(Err(validation_error.into()));
        }
        Ok(Ok(()))
    }

    pub fn destroy(&self, conn: &PgConnection) -> Result<usize, DatabaseError> {
        let cart_user: Option<User> = users::table
            .filter(users::last_cart_id.eq(self.id))
            .get_result(conn)
            .to_db_error(ErrorCode::QueryError, "Could not find user attached to this cart")
            .optional()?;

        if let Some(user) = cart_user {
            user.update_last_cart(None, conn)?;
        }

        DatabaseError::wrap(
            ErrorCode::DeleteError,
            "Failed to delete order record",
            diesel::delete(self).execute(conn),
        )
    }

    pub fn activity(&self, conn: &PgConnection) -> Result<Payload<ActivityItem>, DatabaseError> {
        let activity_items = ActivityItem::load_for_order(&self, conn)?;
        let payload = Payload::from_data(activity_items, 0, std::u32::MAX, None);
        Ok(payload)
    }

    pub fn transfers(&self, conn: &PgConnection) -> Result<Vec<Transfer>, DatabaseError> {
        order_transfers::table
            .inner_join(transfers::table.on(transfers::id.eq(order_transfers::transfer_id)))
            .filter(order_transfers::order_id.eq(self.id))
            .select(transfers::all_columns)
            .then_order_by(transfers::created_at.desc())
            .load(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load transfer tickets")
    }

    pub(crate) fn destroy_item(&self, item_id: Uuid, conn: &PgConnection) -> Result<(), DatabaseError> {
        if self.status != OrderStatus::Draft {
            return DatabaseError::business_process_error(
                "Cannot delete an order item for an order that is not in draft",
            );
        }

        // delete children order items
        diesel::delete(order_items::table.filter(order_items::parent_id.eq(item_id)))
            .execute(conn)
            .map(|_| ())
            .to_db_error(ErrorCode::DeleteError, "Could not delete child order item")?;

        diesel::delete(order_items::table.filter(order_items::id.eq(item_id)))
            .execute(conn)
            .map(|_| ())
            .to_db_error(ErrorCode::DeleteError, "Could not delete order item")
    }

    pub fn main_event_id(&self, conn: &PgConnection) -> Result<Uuid, DatabaseError> {
        for item in self.items(conn)? {
            if let Some(event_id) = item.event_id {
                return Ok(event_id);
            }
        }

        DatabaseError::no_results("Could not find any event for this order")
    }

    pub fn event_slug(&self, conn: &PgConnection) -> Result<String, DatabaseError> {
        let items = self.items(conn)?;
        let mut event_ids: Vec<Uuid> = items.into_iter().filter_map(|i| i.event_id).collect();
        event_ids.sort();
        event_ids.dedup();

        if event_ids.length() > 1 {
            //Currently we only allow a single event per order, so this should never be more than 1
            jlog!(Level::Warn, "Found more than 1 event in an order, Orders::event_slug() is at least 1 place that needs to be updated to allow for more than a single event per cart.");
        }

        if event_ids.length() > 0 {
            let slug = Event::find(event_ids[0], conn)?.slug(conn)?;
            return Ok(slug);
        }

        DatabaseError::no_results("Could not find any event for this order")
    }

    pub fn resend_order_confirmation(&self, current_user_id: Uuid, conn: &PgConnection) -> Result<(), DatabaseError> {
        if self.status != OrderStatus::Paid {
            return DatabaseError::business_process_error("Cannot resend confirmation for unpaid order");
        }

        DomainEvent::create(
            DomainEventTypes::OrderResendConfirmationTriggered,
            "Resend order confirmation".to_string(),
            Tables::Orders,
            Some(self.id),
            Some(current_user_id),
            None,
        )
        .commit(conn)?;

        Ok(())
    }

    pub fn has_refunds(&self, conn: &PgConnection) -> Result<bool, DatabaseError> {
        select(exists(refunds::table.filter(refunds::order_id.eq(self.id))))
            .get_result(conn)
            .to_db_error(ErrorCode::QueryError, "Could not check if order has associated refunds")
    }

    pub fn refund(
        &mut self,
        refund_data: &[RefundItemRequest],
        user_id: Uuid,
        reason: Option<String>,
        manual_override: bool,
        conn: &PgConnection,
    ) -> Result<(Refund, i64), DatabaseError> {
        self.lock_version(conn)?;
        let mut total_to_be_refunded: i64 = 0;

        let refund = Refund::create(self.id, user_id, reason, manual_override).commit(conn)?;
        let previous_item_refund_counts: HashMap<Uuid, i64> =
            self.items(conn)?.iter().map(|i| (i.id, i.refunded_quantity)).collect();

        for refund_datum in refund_data {
            let mut order_item = OrderItem::find(refund_datum.order_item_id, conn)?;
            if order_item.item_type == OrderItemTypes::Discount {
                return DatabaseError::business_process_error("Discount order items can not be refunded");
            } else if order_item.order_id != self.id {
                return DatabaseError::business_process_error("Order item id does not belong to this order");
            }

            // Find the ticket instance if it was specified
            let ticket_instance = match refund_datum.ticket_instance_id {
                Some(id) => Some(TicketInstance::find(id, conn)?),
                None => None,
            };

            if order_item.item_type == OrderItemTypes::Tickets || order_item.item_type == OrderItemTypes::PerUnitFees {
                match ticket_instance {
                    None => {
                        return DatabaseError::business_process_error(
                            "Ticket id required when refunding ticket related order item",
                        );
                    }
                    Some(ref ticket_instance) => {
                        total_to_be_refunded +=
                            Order::refund_ticket_instance(&ticket_instance, &mut order_item, user_id, conn)?;
                    }
                }
            } else {
                total_to_be_refunded += order_item.refund_one_unit(true, conn)?;
            }
        }

        //        // If there are no more items for an event, refund the per event fees
        //        for mut fee_item in self.find_orphaned_per_event_fees(conn)? {
        //            total_to_be_refunded += fee_item.refund_one_unit(true, conn)?;
        //        }

        // Refund items automatically refund their dependencies so the difference in refund_quantity
        // is used to calculate the refund item data.
        let new_item_refund_counts: HashMap<Uuid, i64> =
            self.items(conn)?.iter().map(|i| (i.id, i.refunded_quantity)).collect();
        let mut calculated_refunded_value = 0;
        for (order_item_id, count) in new_item_refund_counts {
            let order_item = OrderItem::find(order_item_id, conn)?;
            if let Some(old_count) = previous_item_refund_counts.get(&order_item_id) {
                let difference = count - old_count;
                let amount = difference * order_item.unit_price_in_cents;
                calculated_refunded_value += amount;
                if difference > 0 {
                    RefundItem::create(refund.id, order_item.id, difference, amount).commit(conn)?;
                }
            }
        }

        if calculated_refunded_value != (total_to_be_refunded) {
            return DatabaseError::business_process_error(&format!(
                "Error processing refund, calculated refund does not match expected total, expected {}, calculated {}",
                total_to_be_refunded, calculated_refunded_value
            ));
        }

        Ok((refund, total_to_be_refunded))
    }

    fn refund_ticket_instance(
        ticket_instance: &TicketInstance,
        order_item: &mut OrderItem,
        user_id: Uuid,
        conn: &PgConnection,
    ) -> Result<i64, DatabaseError> {
        let mut refunded_ticket = RefundedTicket::find_or_create_by_ticket_instance(ticket_instance, conn)?;

        if refunded_ticket.ticket_refunded_at.is_some()
            || (refunded_ticket.fee_refunded_at.is_some() && order_item.item_type == OrderItemTypes::PerUnitFees)
        {
            return DatabaseError::business_process_error("Already refunded");
        }

        if ticket_instance.was_transferred(conn)? {
            return DatabaseError::business_process_error("Ticket was transferred so ineligible for refund");
        }
        let refund_fees = refunded_ticket.fee_refunded_at.is_none();

        if order_item.item_type == OrderItemTypes::PerUnitFees {
            refunded_ticket.mark_fee_only_refunded(conn)?;
        } else {
            refunded_ticket.mark_ticket_and_fee_refunded(conn)?;
        }

        // Release tickets if they are purchased (i.e. not yet redeemed)
        if ticket_instance.status == TicketInstanceStatus::Purchased && order_item.item_type == OrderItemTypes::Tickets
        {
            ticket_instance.release(TicketInstanceStatus::Purchased, user_id, conn)?;
        }

        order_item.refund_one_unit(refund_fees, conn)
    }

    //    fn find_orphaned_per_event_fees(
    //        &self,
    //        conn: &PgConnection,
    //    ) -> Result<Vec<OrderItem>, DatabaseError> {
    //        order_items::table
    //            .filter(order_items::refunded_quantity.ne(order_items::quantity))
    //            .filter(order_items::order_id.eq(self.id))
    //            .filter(
    //                order_items::item_type
    //                    .eq(OrderItemTypes::EventFees)
    //                    .or(order_items::item_type.eq(OrderItemTypes::CreditCardFees)),
    //            )
    //            .filter(sql("not exists(
    //                select oi2.id from order_items oi2
    //                where oi2.order_id = order_items.order_id
    //                and oi2.event_id = order_items.event_id
    //                and oi2.item_type <> 'EventFees'
    //                and oi2.item_type <> 'Discount'
    //                and oi2.item_type <> 'CreditCardFees'
    //                and oi2.refunded_quantity <> oi2.quantity
    //            )"))
    //            .select(order_items::all_columns)
    //            .load(conn)
    //            .to_db_error(
    //                ErrorCode::QueryError,
    //                "Could not check if order only contains event fees",
    //            )
    //    }

    pub fn details(
        &self,
        organization_ids: &Vec<Uuid>,
        user_id: Uuid,
        conn: &PgConnection,
    ) -> Result<Vec<OrderDetailsLineItem>, DatabaseError> {
        let query = include_str!("../queries/order_details.sql");
        diesel::sql_query(query)
            .bind::<dUuid, _>(self.id)
            .bind::<Array<dUuid>, _>(organization_ids)
            .bind::<dUuid, _>(user_id)
            .load(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load order details")
    }

    pub fn partially_visible_order(
        &self,
        organization_ids: &Vec<Uuid>,
        user_id: Uuid,
        conn: &PgConnection,
    ) -> Result<bool, DatabaseError> {
        select(exists(
            order_items::table
                .inner_join(events::table.on(order_items::event_id.eq(events::id.nullable())))
                .left_join(
                    organization_users::table.on(events::organization_id
                        .eq(organization_users::organization_id)
                        .and(organization_users::user_id.eq(user_id))),
                )
                .left_join(
                    event_users::table.on(event_users::event_id
                        .eq(events::id)
                        .and(event_users::user_id.eq(organization_users::user_id))),
                )
                .filter(events::organization_id.ne_all(organization_ids).or(sql("(
                        event_users.id IS NULL
                        AND (
                            'Promoter' = ANY(organization_users.role)
                            OR 'PromoterReadOnly' = ANY(organization_users.role)
                        )
                        )")))
                .filter(order_items::order_id.eq(self.id)),
        ))
        .get_result(conn)
        .to_db_error(ErrorCode::QueryError, "Could not check if order is partially visible")
    }

    pub fn organizations(&self, conn: &PgConnection) -> Result<Vec<Organization>, DatabaseError> {
        organizations::table
            .inner_join(events::table.on(events::organization_id.eq(organizations::id)))
            .inner_join(order_items::table.on(order_items::event_id.eq(events::id.nullable())))
            .filter(order_items::order_id.eq(self.id))
            .select(organizations::all_columns)
            .order_by(organizations::name.asc())
            .distinct()
            .load(conn)
            .to_db_error(ErrorCode::QueryError, "Error loading organizations")
    }

    pub fn is_expired(&self) -> bool {
        self.expires_at.is_some() && self.expires_at < Some(Utc::now().naive_utc())
    }

    pub fn try_refresh_expired_cart(
        &mut self,
        current_user_id: Option<Uuid>,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        jlog!(Level::Debug, "Attempting to refresh expired cart");
        self.lock_version(conn)?;
        let new_expires_at = Utc::now().naive_utc() + Duration::minutes(CART_EXPIRY_TIME_MINUTES);

        if self.status != OrderStatus::Draft && self.status != OrderStatus::PendingPayment {
            return DatabaseError::business_process_error(
                "Can't refresh expired cart unless the order is in draft or pending payment statuses",
            );
        } else if !self.is_expired() {
            return DatabaseError::business_process_error("Cart is not expired");
        }

        for item in self.items(conn)? {
            if item.item_type != OrderItemTypes::Tickets {
                continue;
            } else if item.ticket_type_id.is_none() {
                // Sanity check given unwrap below
                return DatabaseError::business_process_error("Ticket type required for order refresh");
            }

            // Sanity check: clear unexpired tickets (should affect 0; it inherits expires_at from order)
            let quantity = item.calculate_quantity(conn)?;
            TicketInstance::release_tickets(&item, quantity as u32, current_user_id, conn)?;

            // Reserve new tickets for each order item
            TicketInstance::reserve_tickets(
                &item,
                Some(new_expires_at),
                item.ticket_type_id.unwrap(),
                item.hold_id,
                item.quantity as u32,
                conn,
            )?;
        }

        // Update cart expiration
        self.set_expiry(None, Some(new_expires_at), true, conn)?;

        Ok(())
    }

    pub fn payments(&self, conn: &PgConnection) -> Result<Vec<Payment>, DatabaseError> {
        payments::table
            .filter(payments::order_id.eq(self.id))
            .order_by(payments::created_at)
            .load(conn)
            .to_db_error(ErrorCode::QueryError, "Error loading payments")
    }

    pub fn set_browser_data(
        &mut self,
        user_agent: Option<String>,
        purchase_completed: bool,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        self.lock_version(conn)?;
        self.updated_at = Utc::now().naive_utc();

        let platform: Option<Platforms> = if self.box_office_pricing {
            Some(Platforms::BoxOffice)
        } else if user_agent.is_some() {
            Platforms::from_user_agent(user_agent.as_ref().map(|ua| ua.as_str()).unwrap()).ok()
        } else {
            None
        };

        if purchase_completed {
            self.purchase_user_agent = user_agent;
        } else {
            self.create_user_agent = user_agent;
        }

        let affected_rows =
            diesel::update(orders::table.filter(orders::id.eq(self.id).and(orders::version.eq(self.version))))
                .set((
                    orders::purchase_user_agent.eq(self.purchase_user_agent.clone()),
                    orders::create_user_agent.eq(self.create_user_agent.clone()),
                    orders::platform.eq(platform.unwrap_or(Platforms::Web)),
                    orders::updated_at.eq(self.updated_at),
                ))
                .execute(conn)
                .to_db_error(ErrorCode::UpdateError, "Could not update user agent")?;
        if affected_rows != 1 {
            return DatabaseError::concurrency_error("Could not update user agent.");
        }

        Ok(())
    }

    pub fn set_tracking_data(
        &mut self,
        tracking_data: Option<serde_json::Value>,
        current_user_id: Option<Uuid>,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        self.updated_at = Utc::now().naive_utc();

        let mut source: Option<String> = None;
        let mut medium: Option<&str> = None;
        let mut campaign: Option<&str> = None;
        let mut term: Option<&str> = None;
        let mut content: Option<&str> = None;
        let mut referrer: Option<&str> = None;

        if let Some(td) = tracking_data.as_ref() {
            referrer = td.get("referrer").and_then(|r| r.as_str());
            let referrer_host = match referrer {
                Some(r) => match Url::parse(r) {
                    Ok(p) => p.host_str().map(|h| h.to_string()),
                    Err(_) => Some(r.to_string()),
                },
                None => None,
            };
            source = td
                .get("utm_source")
                .and_then(|s| s.as_str().map(|s| s.to_string()))
                .or(referrer_host)
                .or(td.get("fbclid").map(|_| "facebook.com".to_string()))
                .or(Some("direct".to_string()));
            medium = td
                .get("utm_medium")
                .and_then(|m| m.as_str())
                .or(referrer.map(|_| "referral"))
                .or(td.get("fbclid").map(|_| "referral"));
            campaign = td.get("utm_campaign").and_then(|c| c.as_str());
            term = td.get("utm_term").and_then(|t| t.as_str());
            content = td.get("utm_content").and_then(|c| c.as_str());
        }

        diesel::update(orders::table.filter(orders::id.eq(self.id)))
            .set((
                orders::tracking_data.eq(tracking_data.clone()),
                orders::referrer.eq(referrer),
                orders::source.eq(source),
                orders::medium.eq(medium),
                orders::campaign.eq(campaign),
                orders::term.eq(term),
                orders::content.eq(content),
                orders::updated_at.eq(self.updated_at),
            ))
            .execute(conn)
            .to_db_error(ErrorCode::UpdateError, "Could not update user agent")?;

        DomainEvent::create(
            DomainEventTypes::TrackingDataUpdated,
            "Tracking data updated".to_string(),
            Tables::Orders,
            Some(self.id),
            current_user_id,
            tracking_data.clone(),
        )
        .commit(conn)?;

        Ok(())
    }

    pub fn find_or_create_cart(user: &User, conn: &PgConnection) -> Result<Order, DatabaseError> {
        // Do a quick check to find the cart linked to the user.
        let cart = Order::find_cart_for_user(user.id, conn)?;

        if let Some(cart) = cart {
            return Ok(cart);
        }

        // Cart either does not exist, expired or was paid up.
        // A number of threads might reach here at the same time, so we
        // need to do a bit of concurrency checking.

        let query = r#"
            INSERT INTO Orders (user_id, status, expires_at, order_type)
            SELECT $1 as user_id, 'Draft' as status, null as expires_at, 'Cart' as order_type
            WHERE NOT EXISTS
            ( SELECT o.id FROM orders o
                WHERE o.user_id = $1
                AND o.status = 'Draft'
                AND o.order_type = 'Cart'
                AND o.expires_at > now())
            RETURNING id;
        "#;

        #[derive(QueryableByName)]
        struct R {
            #[sql_type = "Nullable<dUuid>"]
            id: Option<Uuid>,
        }

        let cart_id: Vec<R> = diesel::sql_query(query)
            .bind::<sql_types::Uuid, _>(user.id)
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Could not find or create cart")?;

        if cart_id.is_empty() || cart_id[0].id.is_none() || cart_id.len() > 1 {
            // Another thread has created a cart
            return DatabaseError::concurrency_error(&format!(
                "Possible race condition when creating a cart for a user. Number of carts returned: {}",
                cart_id.len()
            ));
        }

        let cart_id = cart_id[0].id;

        // This will also row lock the user row to detect that another thread has not
        // created another cart in the mean time
        user.update_last_cart(cart_id, conn)?;

        let order = Order::find(cart_id.unwrap(), conn)?;

        DomainEvent::create(
            DomainEventTypes::OrderCreated,
            "Order created".into(),
            Tables::Orders,
            Some(order.id),
            Some(user.id),
            Some(json!(order)),
        )
        .commit(conn)?;

        Ok(order)
    }

    pub fn find_cart_for_user(user_id: Uuid, conn: &PgConnection) -> Result<Option<Order>, DatabaseError> {
        users::table
            .inner_join(orders::table.on(users::last_cart_id.eq(orders::id.nullable())))
            .filter(users::id.eq(user_id))
            .filter(orders::user_id.eq(user_id))
            .filter(orders::status.eq(OrderStatus::Draft))
            .filter(orders::order_type.eq(OrderTypes::Cart))
            .filter(
                orders::expires_at
                    .is_null()
                    .or(orders::expires_at.ge(dsl::now.nullable())),
            )
            .select(orders::all_columns)
            .first(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load cart for user")
            .optional()
    }

    pub fn find(id: Uuid, conn: &PgConnection) -> Result<Order, DatabaseError> {
        orders::table
            .filter(orders::id.eq(id))
            .first(conn)
            .to_db_error(ErrorCode::QueryError, "Could not find order")
    }

    pub fn search(
        event_id: Option<Uuid>,
        organization_id: Option<Uuid>,
        general_query: Option<&str>,
        partial_order_no: Option<&str>,
        partial_ticket_no: Option<&str>,
        email: Option<&str>,
        phone: Option<&str>,
        name: Option<&str>,
        ticket_type_id: Option<Uuid>,
        partial_promo_code: Option<&str>,
        box_office_sales: bool,
        online_sales: bool,
        web_sales: bool,
        app_sales: bool,
        start_date: Option<NaiveDateTime>,
        end_date: Option<NaiveDateTime>,
        current_user_id: Uuid,
        paging: &PagingParameters,
        conn: &PgConnection,
    ) -> Result<(Vec<DisplayOrder>, i64), DatabaseError> {
        let mut query = sql_query(
            r#"
        SELECT *, COUNT(*) OVER () as total FROM (
        select distinct o.*
        from orders o
            inner join order_items oi on oi.order_id = o.id
            inner join events e on oi.event_id = e.id
            inner join ticket_types tt on oi.ticket_type_id= tt.id
            left join refunded_tickets rt on oi.id = rt.order_item_id
            left join ticket_instances ti on (oi.id = ti.order_item_id or rt.ticket_instance_id = ti.id)
            left join transfer_tickets trt on trt.ticket_instance_id = ti.id
            left join transfers trns on trt.transfer_id = trns.id
            left join users trnsu on trns.destination_user_id = trnsu.id
            left join holds h on oi.hold_id = h.id
            left join codes c on oi.code_id = c.id
            inner join users u on o.user_id = u.id
            left join users bu on o.on_behalf_of_user_id = bu.id

        where o.status <> 'Draft'
        "#,
        )
        .into_boxed();

        let mut bind_no = 0;

        if let Some(event_id) = event_id {
            bind_no = bind_no + 1;
            query = query
                .sql(format!(" and tt.event_id = ${} ", bind_no))
                .bind::<sql_types::Uuid, _>(event_id);
        }

        if let Some(organization_id) = organization_id {
            bind_no = bind_no + 1;
            query = query
                .sql(format!(" and e.organization_id = ${} ", bind_no))
                .bind::<sql_types::Uuid, _>(organization_id);
        }

        if let Some(general_query) = general_query {
            bind_no = bind_no + 1;
            query = query
                .sql(format!(" and (o.id::text ilike ${}  or ti.id::text ilike ${} or coalesce(bu.email, u.email) ilike ${} or trns.transfer_address ilike ${} or coalesce(bu.phone, u.phone) ilike ${} or coalesce(h.redemption_code, c.redemption_code) ilike ${}  or (coalesce(bu.first_name, u.first_name) || ' ' || coalesce(bu.last_name, u.last_name) ilike ${} or coalesce(bu.last_name, u.last_name) || ' ' || coalesce(bu.first_name, u.first_name) ilike ${} or trnsu.first_name || ' ' || trnsu.last_name ilike ${} or trnsu.last_name || ' ' || trnsu.first_name ilike ${}) )", bind_no, bind_no, bind_no, bind_no, bind_no, bind_no, bind_no, bind_no, bind_no, bind_no))
                .bind::<diesel::sql_types::Text, _>(format!("%{}%", general_query));
        }

        if let Some(partial_order_no) = partial_order_no {
            bind_no = bind_no + 1;
            query = query
                .sql(format!(" and o.id::text ilike ${} ", bind_no))
                .bind::<diesel::sql_types::Text, _>(format!("%{}%", partial_order_no));
        }

        if let Some(partial_ticket_no) = partial_ticket_no {
            bind_no = bind_no + 1;
            query = query
                .sql(format!(" and ti.id::text ilike ${} ", bind_no))
                .bind::<diesel::sql_types::Text, _>(format!("%{}%", partial_ticket_no));
        }

        if let Some(email) = email {
            bind_no = bind_no + 1;
            query = query
                .sql(format!(
                    " and (coalesce(bu.email, u.email) ilike ${} or trns.transfer_address ilike ${})",
                    bind_no, bind_no
                ))
                .bind::<sql_types::Text, _>(format!("%{}%", email));
        }
        if let Some(phone) = phone {
            bind_no = bind_no + 1;
            query = query
                .sql(format!(
                    " and (coalesce(bu.phone, u.phone) ilike ${} or trns.transfer_address ilike ${})",
                    bind_no, bind_no
                ))
                .bind::<sql_types::Text, _>(format!("%{}%", phone));
        }

        if let Some(name) = name {
            bind_no = bind_no + 2;
            let name = format!("%{}%", regexes::whitespace().replace_all(name, "%"));
            query = query.sql(format!(" and (coalesce(bu.first_name, u.first_name) || ' ' || coalesce(bu.last_name, u.last_name) ilike ${} or  coalesce(bu.last_name, u.last_name) || ' ' || coalesce(bu.first_name, u.first_name) ilike ${}) ", bind_no - 1, bind_no)).bind::<sql_types::Text, _>(name.clone()).bind::<sql_types::Text, _>(name);
            // TODO: Add order by edit distance
        }

        if let Some(ticket_type_id) = ticket_type_id {
            bind_no = bind_no + 1;
            query = query
                .sql(format!(" and tt.id = ${} ", bind_no))
                .bind::<sql_types::Uuid, _>(ticket_type_id);
        }

        if let Some(partial_promo_code) = partial_promo_code {
            bind_no = bind_no + 1;
            query = query
                .sql(format!(
                    " and coalesce(h.redemption_code, c.redemption_code) ilike ${} ",
                    bind_no
                ))
                .bind::<sql_types::Text, _>(format!("%{}%", partial_promo_code));
        }

        if let Some(date) = start_date {
            bind_no = bind_no + 1;
            query = query
                .sql(format!(" and o.order_date >= ${} ", bind_no))
                .bind::<sql_types::Timestamp, _>(date);
        }

        if let Some(date) = end_date {
            bind_no = bind_no + 1;
            query = query
                .sql(format!(" and o.order_date <= ${} ", bind_no))
                .bind::<sql_types::Timestamp, _>(date);
        }

        if !box_office_sales {
            query = query.sql(" and o.on_behalf_of_user_id is null ");
        }
        if !online_sales {
            query = query.sql(" and o.on_behalf_of_user_id is not null ");
        }

        if !web_sales {
            query = query.sql(" and o.platform != 'Web'");
        }

        if !app_sales {
            query = query.sql(" and o.platform != 'App'");
        }

        query = query.sql(" order by order_date desc");

        #[derive(QueryableByName)]
        struct R {
            #[sql_type = "dUuid"]
            id: Uuid,
            #[sql_type = "BigInt"]
            total: i64,
        }

        let limit = paging.limit.unwrap_or(100) as i64;
        query = query
            .sql(format!(") t LIMIT ${}  OFFSET ${}", bind_no + 1, bind_no + 2))
            .bind::<sql_types::BigInt, _>(limit)
            .bind::<sql_types::BigInt, _>(paging.page.unwrap_or(0) as i64 * limit);
        let order_data: Vec<R> = query
            .load(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load orders")?;

        Ok((
            Order::load_for_display(order_data.iter().map(|o| o.id).collect(), None, current_user_id, conn)?,
            order_data.get(0).map(|s| s.total).unwrap_or(0),
        ))
    }

    /// Sets the expiry time of an order. All tickets in the current order are also updated
    /// to reflect the new expiry
    pub fn set_expiry(
        &mut self,
        current_user_id: Option<Uuid>,
        expires_at: Option<NaiveDateTime>,
        force: bool,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        let old_expiry = self.expires_at;
        let expires_at = if expires_at.is_some() {
            expires_at.unwrap()
        } else {
            Utc::now().naive_utc() + Duration::minutes(CART_EXPIRY_TIME_MINUTES)
        };
        self.expires_at = Some(expires_at);
        self.updated_at = Utc::now().naive_utc();

        let affected_rows = diesel::update(
            orders::table.filter(
                orders::id.eq(self.id).and(orders::version.eq(self.version)).and(
                    sql("COALESCE(expires_at, '31 Dec 9999') > now()")
                        .or(dsl::sql("TRUE = ").bind::<diesel::sql_types::Bool, _>(force)),
                ),
            ),
        )
        .set((
            orders::expires_at.eq(self.expires_at),
            orders::updated_at.eq(self.updated_at),
        ))
        .execute(conn)
        .to_db_error(ErrorCode::UpdateError, "Could not update expiry time")?;
        if affected_rows != 1 {
            return DatabaseError::concurrency_error("Could not update expiry time.");
        }

        DomainEvent::create(
            DomainEventTypes::OrderUpdated,
            format!(
                "Order expiry time updated from {:?} to {:?}",
                &old_expiry.map(|e| e.to_string()).unwrap_or("null".into()),
                &expires_at.to_string()
            ),
            Tables::Orders,
            Some(self.id),
            current_user_id,
            Some(json!({
                "old_expires_at": &old_expiry,
                "new_expires_at": &expires_at
            })),
        )
        .commit(conn)?;

        // Extend the tickets expiry
        let order_items = OrderItem::find_for_order(self.id, conn)?;

        for item in &order_items {
            TicketInstance::update_reserved_time(item, expires_at, conn)?;
        }

        Ok(())
    }

    /// Removes the expiry time for an order. This can only be done when there are no
    /// tickets in the order, otherwise the tickets will remain reserved until the expiry
    pub fn remove_expiry(&mut self, current_user_id: Uuid, conn: &PgConnection) -> Result<(), DatabaseError> {
        if self.items(conn)?.len() > 0 {
            return DatabaseError::business_process_error(
                "Cannot clear the expiry of an order when there are items in it",
            );
        }
        let old_expiry = self.expires_at;
        self.updated_at = Utc::now().naive_utc();
        self.expires_at = None;
        let affected_rows = diesel::update(
            orders::table.filter(
                orders::id.eq(self.id).and(orders::version.eq(self.version)).and(
                    orders::expires_at
                        .is_null()
                        .or(orders::expires_at.gt(Some(Utc::now().naive_utc()))),
                ),
            ),
        )
        .set((
            orders::expires_at.eq(self.expires_at),
            orders::updated_at.eq(self.updated_at),
        ))
        .execute(conn)
        .to_db_error(ErrorCode::UpdateError, "Could not update expiry time")?;
        if affected_rows != 1 {
            return DatabaseError::concurrency_error("Could not update expiry time.");
        }
        DomainEvent::create(
            DomainEventTypes::OrderUpdated,
            format!(
                "Order expiry time removed was {:?}",
                &old_expiry.map(|e| e.to_string()).unwrap_or("null".into())
            ),
            Tables::Orders,
            Some(self.id),
            Some(current_user_id),
            Some(json!({
                "old_expires_at": &old_expiry,
                "new_expires_at": self.expires_at
            })),
        )
        .commit(conn)?;

        Ok(())
    }

    pub fn order_number(&self) -> String {
        Order::parse_order_number(self.id)
    }

    pub fn parse_order_number(id: Uuid) -> String {
        let id_string = id.to_string();
        id_string[id_string.len() - ORDER_NUMBER_LENGTH..].to_string()
    }

    pub fn clear_cart(&mut self, user_id: Uuid, conn: &PgConnection) -> Result<(), DatabaseError> {
        jlog!(Level::Debug, "Clearing cart");
        self.lock_version(conn)?;

        for current_line in self.items(conn)? {
            if current_line.item_type != OrderItemTypes::Tickets {
                continue;
            }
            // Use calculated quantity as reserved may have been taken in the meantime no longer pointing to this order item
            let quantity = current_line.calculate_quantity(conn)?;
            TicketInstance::release_tickets(&current_line, quantity as u32, Some(user_id), conn)?;
            self.destroy_item(current_line.id, conn)?;
        }
        Ok(())
    }

    pub fn update_quantities(
        &mut self,
        current_user_id: Uuid,
        items: &[UpdateOrderItem],
        box_office_pricing: bool,
        remove_others: bool,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        self.lock_version(conn)?;

        jlog!(Debug, "Update order quantities", {"items": items,"remove_others":remove_others, "user_id": current_user_id, "box_office_pricing":box_office_pricing });

        if box_office_pricing != self.box_office_pricing {
            self.clear_cart(current_user_id, conn)?;
            self.update_box_office_pricing(box_office_pricing, current_user_id, conn)?;
        }

        let current_items = self.items(conn)?;

        let mut check_ticket_limits: Vec<LimitCheck> = vec![];
        let mut mapped = vec![];
        for (index, item) in items.iter().enumerate() {
            let ticket_type = TicketType::find(item.ticket_type_id, conn)?;
            mapped.push(match &item.redemption_code {
                Some(r) => match Hold::find_by_redemption_code(r, Some(ticket_type.event_id), conn).optional()? {
                    Some(hold) => {
                        hold.confirm_hold_valid()?;
                        MatchData {
                            index: Some(index),
                            hold_id: Some(hold.id),
                            hold: Some(hold),
                            code_id: None,
                            code: None,
                            redemption_code: item.redemption_code.clone(),
                            update_order_item: item,
                        }
                    }
                    None => match Code::find_by_redemption_code_with_availability(r, Some(ticket_type.event_id), conn)
                        .optional()?
                    {
                        Some(code_availability) => {
                            code_availability.code.confirm_code_valid()?;
                            MatchData {
                                index: Some(index),
                                hold_id: None,
                                hold: None,
                                code_id: Some(code_availability.code.id),
                                code: Some(code_availability.code),
                                redemption_code: item.redemption_code.clone(),
                                update_order_item: item,
                            }
                        }
                        None => {
                            return DatabaseError::validation_error("redemption_code", "Redemption code is not valid");
                        }
                    },
                },
                None => MatchData {
                    index: Some(index),
                    hold_id: None,
                    hold: None,
                    code_id: None,
                    code: None,
                    redemption_code: None,
                    update_order_item: item,
                },
            });
        }

        for mut current_line in current_items {
            if current_line.item_type != OrderItemTypes::Tickets {
                continue;
            }

            let mut index_to_remove: Option<usize> = None;
            {
                let matching_result: Option<&MatchData> = mapped.iter().find(|match_data| {
                    match_data.index.is_some()
                        && Some(match_data.update_order_item.ticket_type_id) == current_line.ticket_type_id
                        && match_data.hold_id == current_line.hold_id
                        && match_data.code_id == current_line.code_id
                });

                if let Some(match_data) = matching_result {
                    jlog!(Level::Debug, "Found an existing cart item, replacing");
                    index_to_remove = match_data.index;
                    if current_line.quantity as u32 > match_data.update_order_item.quantity {
                        jlog!(Level::Debug, "Reducing quantity of cart item");
                        TicketInstance::release_tickets(
                            &current_line,
                            current_line.quantity as u32 - match_data.update_order_item.quantity,
                            Some(current_user_id),
                            conn,
                        )?;
                        current_line.quantity = match_data.update_order_item.quantity as i64;
                        current_line.update(conn)?;
                        if current_line.quantity == 0 {
                            jlog!(Level::Debug, "Cart item has 0 quantity, deleting it");
                            self.destroy_item(current_line.id, conn)?;
                        }
                    } else if (current_line.quantity as u32) < match_data.update_order_item.quantity {
                        jlog!(Level::Debug, "Increasing quantity of cart item");
                        // Ticket pricing might have changed since we added the previous item.
                        // In future we may want to use the ticket pricing at the time the order was created.

                        // TODO: Fetch the ticket type and pricing in one go.
                        let ticket_type_id = current_line.ticket_type_id.unwrap();
                        let ticket_pricing =
                            TicketPricing::get_current_ticket_pricing(ticket_type_id, box_office_pricing, false, conn)?;
                        let ticket_type = TicketType::find(ticket_type_id, conn)?;
                        check_ticket_limits.append(&mut Order::check_ticket_limits(&ticket_type, &match_data));

                        // TODO: Move this to an external processer
                        if Some(ticket_pricing.id) != current_line.ticket_pricing_id {
                            let price_in_cents = ticket_pricing.price_in_cents;

                            let order_item = NewTicketsOrderItem {
                                order_id: self.id,
                                item_type: OrderItemTypes::Tickets,
                                quantity: match_data.update_order_item.quantity as i64,
                                ticket_type_id: ticket_type.id,
                                ticket_pricing_id: ticket_pricing.id,
                                event_id: Some(ticket_type.event_id),
                                unit_price_in_cents: price_in_cents,
                                hold_id: match_data.hold_id,
                                code_id: match_data.code_id,
                            }
                            .commit(conn)?;
                            TicketInstance::reserve_tickets(
                                &order_item,
                                self.expires_at,
                                ticket_type_id,
                                match_data.hold_id,
                                match_data.update_order_item.quantity - current_line.quantity as u32,
                                conn,
                            )?;
                        } else {
                            TicketInstance::reserve_tickets(
                                &current_line,
                                self.expires_at,
                                ticket_type_id,
                                match_data.hold_id,
                                match_data.update_order_item.quantity - current_line.quantity as u32,
                                conn,
                            )?;
                            current_line.quantity = match_data.update_order_item.quantity as i64;
                            current_line.update(conn)?;
                        }
                    }
                } else if remove_others {
                    jlog!(Level::Debug, "Removing extra tickets because remove others was called.", { "order_item.id": current_line.id, "ticket_type_id": current_line.ticket_type_id});
                    jlog!(Level::Debug, "Reducing quantity of cart item");
                    TicketInstance::release_tickets(
                        &current_line,
                        current_line.quantity as u32,
                        Some(current_user_id),
                        conn,
                    )?;
                    self.destroy_item(current_line.id, conn)?;
                }
            }
            if let Some(index) = index_to_remove {
                mapped[index].index = None;
            }
        }

        // Set cart expiration time if not currently set (empty carts have no expiration)
        if self.expires_at.is_none() {
            self.set_expiry(Some(current_user_id), None, false, conn)?;
        }

        for match_data in mapped {
            if match_data.update_order_item.quantity == 0 || match_data.index.is_none() {
                continue;
            }

            jlog!(Level::Debug, "Adding new cart items");
            let ticket_pricing = TicketPricing::get_current_ticket_pricing(
                match_data.update_order_item.ticket_type_id,
                box_office_pricing,
                false,
                conn,
            )?;

            let ticket_type = TicketType::find(match_data.update_order_item.ticket_type_id, conn)?;
            check_ticket_limits.append(&mut Order::check_ticket_limits(&ticket_type, &match_data));

            let price_in_cents = ticket_pricing.price_in_cents;

            // TODO: Move this to an external processer
            let order_item = NewTicketsOrderItem {
                order_id: self.id,
                item_type: OrderItemTypes::Tickets,
                quantity: match_data.update_order_item.quantity as i64,
                ticket_type_id: ticket_type.id,
                ticket_pricing_id: ticket_pricing.id,
                event_id: Some(ticket_type.event_id),
                unit_price_in_cents: price_in_cents,
                hold_id: match_data.hold_id,
                code_id: match_data.code_id,
            }
            .commit(conn);

            TicketInstance::reserve_tickets(
                &order_item?,
                self.expires_at,
                match_data.update_order_item.ticket_type_id,
                match_data.hold_id,
                match_data.update_order_item.quantity,
                conn,
            )?;
        }

        // if the cart is empty at this point, it is effectively a new cart, remove expiration
        if self.items(conn)?.len() == 0 {
            self.remove_expiry(current_user_id, conn)?;
        }
        for limit_check in check_ticket_limits {
            let ordered_quantity = Order::quantity_for_user_for_ticket_type(
                self.user_id,
                limit_check.ticket_type_id,
                limit_check.hold_id,
                limit_check.code_id,
                &conn,
            )?;

            if limit_check.limit_per_person > 0 && ordered_quantity > limit_check.limit_per_person.into() {
                let mut error = ValidationError::new("limit_per_person_exceeded");
                error.message = Some(Cow::from(
                    if limit_check.hold_id.is_some() || limit_check.code_id.is_some() {
                        format!(
                            "Max of {} uses for code {} exceeded",
                            &limit_check.limit_per_person,
                            limit_check.redemption_code.unwrap_or("".into())
                        )
                    } else {
                        "You have exceeded the max tickets per customer limit.".into()
                    },
                ));
                error.add_param(Cow::from("limit_per_person"), &limit_check.limit_per_person);
                error.add_param(Cow::from("ticket_type_id"), &limit_check.ticket_type_id);
                if let Some(hold_id) = limit_check.hold_id {
                    error.add_param(Cow::from("hold_id"), &hold_id);
                }
                if let Some(code_id) = limit_check.code_id {
                    error.add_param(Cow::from("code_id"), &code_id);
                }
                error.add_param(Cow::from("attempted_quantity"), &ordered_quantity);
                let mut errors = ValidationErrors::new();
                errors.add("quantity", error);
                return Err(errors.into());
            }
        }
        self.update_fees_and_discounts(conn)?;
        self.validate_record(conn)?;
        // Beware there could be multiple orders that meet this condition
        for (ticket_type_id, remaining) in self.ticket_types(conn)? {
            if remaining == 0 {
                TicketType::find(ticket_type_id, conn)?.check_for_sold_out_triggers(Some(current_user_id), conn)?;
            }
        }

        Ok(())
    }

    fn check_ticket_limits(ticket_type: &TicketType, match_data: &MatchData) -> Vec<LimitCheck> {
        let mut check_ticket_limits: Vec<LimitCheck> = vec![];
        check_ticket_limits.push(LimitCheck {
            ticket_type_id: ticket_type.id,
            hold_id: None,
            code_id: None,
            limit_per_person: ticket_type.limit_per_person as u32,
            redemption_code: None,
        });
        if let Some(ref hold) = match_data.hold {
            check_ticket_limits.push(LimitCheck {
                ticket_type_id: ticket_type.id,
                hold_id: Some(hold.id),
                code_id: None,
                limit_per_person: hold.max_per_user.unwrap_or(0) as u32,
                redemption_code: match_data.redemption_code.clone(),
            });
        } else if let Some(ref code) = match_data.code {
            check_ticket_limits.push(LimitCheck {
                ticket_type_id: ticket_type.id,
                hold_id: None,
                code_id: Some(code.id),
                limit_per_person: code.max_tickets_per_user.unwrap_or(0) as u32,
                redemption_code: match_data.redemption_code.clone(),
            });
        }
        check_ticket_limits
    }

    pub fn has_items(&self, conn: &PgConnection) -> Result<bool, DatabaseError> {
        select(exists(order_items::table.filter(order_items::order_id.eq(self.id))))
            .get_result(conn)
            .to_db_error(ErrorCode::QueryError, "Could not check if order items exist")
    }

    pub fn update_fees_and_discounts(&self, conn: &PgConnection) -> Result<(), DatabaseError> {
        let items = self.items(conn)?;

        for o in items {
            o.update_discount(&self, conn)?;
            match o.item_type {
                OrderItemTypes::EventFees => self.destroy_item(o.id, conn)?,
                OrderItemTypes::CreditCardFees => self.destroy_item(o.id, conn)?,
                _ => {}
            }
        }

        // Box office purchased tickets do not have fees at this time
        if self.box_office_pricing {
            return Ok(());
        }

        let mut per_event_fees_included: HashMap<Uuid, bool> = HashMap::new();

        for ((event_id, hold_id), items) in self
            .items(conn)?
            .iter()
            .filter(|i| i.event_id.is_some())
            .sorted_by_key(|i| (i.event_id, i.hold_id))
            .into_iter()
            .group_by(|i| (i.event_id, i.hold_id))
            .into_iter()
        {
            if event_id.is_none() {
                continue;
            }
            if let Some(hold_id) = hold_id {
                let hold = Hold::find(hold_id, conn)?;
                if hold.hold_type == HoldTypes::Comp {
                    continue;
                }
            }

            let event_id = event_id.unwrap();
            let event = Event::find(event_id, conn)?;

            let mut all_zero_price = true;

            for o in items {
                match o.item_type {
                    OrderItemTypes::Tickets => {
                        let discount_item = o.find_discount_item(conn)?;

                        let unit_price_with_discount = match discount_item {
                            Some(di) => o.unit_price_in_cents + di.unit_price_in_cents,
                            None => o.unit_price_in_cents,
                        };

                        o.update_fees(&self, conn)?;
                        if unit_price_with_discount > 0 {
                            all_zero_price = false;
                        }
                    }
                    _ => {}
                }
            }

            //This must only be run once for an entire order
            //The issue was that if there was a hold that was not a comp as well as normal tickets
            //in the cart the EventFees would get duplicated
            if !all_zero_price && !per_event_fees_included.contains_key(&event_id) {
                let mut new_event_fee = NewFeesOrderItem {
                    order_id: self.id,
                    item_type: OrderItemTypes::EventFees,
                    event_id: Some(event.id),
                    unit_price_in_cents: 0,
                    fee_schedule_range_id: None,
                    company_fee_in_cents: 0,
                    client_fee_in_cents: 0,
                    quantity: 1,
                    parent_id: None,
                };

                // Credit card fees are set per organization
                // Event can override organization client and company fees
                let org = Organization::find(event.organization_id, conn)?;
                let company_fee_in_cents = event.company_fee_in_cents.unwrap_or(org.company_event_fee_in_cents);
                let client_fee_in_cents = event.client_fee_in_cents.unwrap_or(org.client_event_fee_in_cents);
                if (company_fee_in_cents + client_fee_in_cents) > 0 {
                    //we dont want to create 0 fee order item
                    new_event_fee.company_fee_in_cents = company_fee_in_cents;
                    new_event_fee.client_fee_in_cents = client_fee_in_cents;
                    new_event_fee.unit_price_in_cents = client_fee_in_cents + company_fee_in_cents;
                    new_event_fee.commit(conn)?;
                    per_event_fees_included.insert(event_id, true);
                }

                if org.cc_fee_percent > 0f32 {
                    let cc_fee = (self.calculate_total(conn)? as f32 * (org.cc_fee_percent / 100f32)).round() as i64;
                    NewFeesOrderItem {
                        order_id: self.id,
                        item_type: OrderItemTypes::CreditCardFees,
                        event_id: Some(event.id),
                        unit_price_in_cents: cc_fee,
                        fee_schedule_range_id: None,
                        company_fee_in_cents: cc_fee,
                        client_fee_in_cents: 0,
                        quantity: 1,
                        parent_id: None,
                    }
                    .commit(conn)?;
                }
            }
        }

        Ok(())
    }

    fn quantity_for_user_for_ticket_type(
        user_id: Uuid,
        ticket_type_id: Uuid,
        hold_id: Option<Uuid>,
        code_id: Option<Uuid>,
        conn: &PgConnection,
    ) -> Result<i64, DatabaseError> {
        use schema::*;

        let mut query = orders::table
            .inner_join(order_items::table.on(order_items::order_id.eq(orders::id)))
            .inner_join(ticket_instances::table.on(ticket_instances::order_item_id.eq(order_items::id.nullable())))
            .filter(
                orders::user_id
                    .eq(user_id)
                    .and(orders::on_behalf_of_user_id.is_null())
                    .or(orders::on_behalf_of_user_id.eq(user_id)),
            )
            .filter(order_items::ticket_type_id.eq(ticket_type_id))
            .filter(
                ticket_instances::status
                    .eq(TicketInstanceStatus::Purchased)
                    .or(ticket_instances::status
                        .eq(TicketInstanceStatus::Reserved)
                        .and(ticket_instances::reserved_until.gt(Utc::now().naive_utc()))),
            )
            .into_boxed();

        if let Some(hold_id) = hold_id {
            query = query.filter(order_items::hold_id.nullable().eq(hold_id));
        }

        if let Some(code_id) = code_id {
            query = query.filter(order_items::code_id.nullable().eq(code_id));
        }

        query
            .select(dsl::count(ticket_instances::id))
            .get_result(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load total")
    }

    pub fn quantity_for_user_for_event(
        user_id: Uuid,
        event_id: Uuid,
        conn: &PgConnection,
    ) -> Result<HashMap<Uuid, i32>, DatabaseError> {
        let mut ticket_type_totals: HashMap<Uuid, i32> = HashMap::new();

        let query = include_str!("../queries/quantity_of_tickets_per_user_per_event.sql");
        let order_items_for_user: Vec<ResultForTicketTypeTotal> = diesel::sql_query(query)
            .bind::<diesel::sql_types::Uuid, _>(user_id)
            .bind::<diesel::sql_types::Uuid, _>(event_id)
            .load::<ResultForTicketTypeTotal>(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load orders")?;

        for result_for_ticket in &order_items_for_user {
            ticket_type_totals.insert(
                result_for_ticket.ticket_type_id.unwrap(),
                result_for_ticket.total_quantity,
            );
        }

        Ok(ticket_type_totals)
    }

    pub fn find_for_user_for_display(user_id: Uuid, conn: &PgConnection) -> Result<Vec<DisplayOrder>, DatabaseError> {
        let order_ids: Vec<Uuid> = orders::table
            .filter(sql("COALESCE(orders.on_behalf_of_user_id, orders.user_id) = ").bind::<dUuid, _>(user_id))
            .filter(orders::status.ne(OrderStatus::Draft))
            .order_by(orders::order_date.desc())
            .select(orders::id)
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load orders")?;

        Order::load_for_display(order_ids, None, user_id, conn)
    }

    pub fn items(&self, conn: &PgConnection) -> Result<Vec<OrderItem>, DatabaseError> {
        OrderItem::find_for_order(self.id, conn)
    }

    pub fn tickets(
        &self,
        ticket_type_id: Option<Uuid>,
        conn: &PgConnection,
    ) -> Result<Vec<TicketInstance>, DatabaseError> {
        let mut items: Vec<OrderItem> = self
            .items(conn)?
            .into_iter()
            .filter(|i| i.item_type == OrderItemTypes::Tickets)
            .collect();

        if ticket_type_id.is_some() {
            items = items
                .into_iter()
                .filter(|i| i.ticket_type_id == ticket_type_id)
                .collect();
        }
        let mut result: Vec<TicketInstance> = vec![];
        for item in items {
            let mut instances = TicketInstance::find_for_order_item(item.id, conn)?;
            result.append(&mut instances);
        }

        Ok(result)
    }

    pub fn events(&self, conn: &PgConnection) -> Result<Vec<Event>, DatabaseError> {
        let mut unique_events: Vec<Uuid> = self.items(conn)?.iter().filter_map(|i| i.event_id).collect();
        unique_events.sort();
        unique_events.dedup();

        Event::find_by_ids(unique_events, conn)
    }

    /// Returns a list of ticket types found in this order as well as the number of
    /// remaining available tickets for that ticket type
    pub fn ticket_types(&self, conn: &PgConnection) -> Result<Vec<(Uuid, i64)>, DatabaseError> {
        let query = r#"
            SELECT remaining.id AS ticket_type_id, remaining.count
            FROM (SELECT tt.id,
                SUM(CASE
                   WHEN (ti.status = 'Available' OR (ti.status = 'Reserved' AND ti.reserved_until < now())) THEN 1
                   ELSE 0 END) AS count
                FROM ticket_types tt
                INNER JOIN assets a
                    INNER JOIN ticket_instances ti
                          ON a.id = ti.asset_id
                        ON tt.id = a.ticket_type_id
                GROUP BY tt.id
                ) AS remaining
                INNER JOIN order_items oi
                    ON remaining.id = oi.ticket_type_id
            WHERE oi.order_id = $1;
        "#;

        #[derive(QueryableByName)]
        struct R {
            #[sql_type = "dUuid"]
            ticket_type_id: Uuid,
            #[sql_type = "BigInt"]
            count: i64,
        };

        let results: Vec<R> = diesel::sql_query(query)
            .bind::<sql_types::Uuid, _>(self.id)
            .load(conn)
            .to_db_error(ErrorCode::QueryError, "Could not find ticket types for order")?;

        Ok(results.into_iter().map(|r| (r.ticket_type_id, r.count)).collect_vec())
    }

    pub fn purchase_metadata(&self, conn: &PgConnection) -> Result<Vec<(String, String)>, DatabaseError> {
        let query = r#"
            SELECT
                o.id as order_id,
                COALESCE(string_agg(distinct e.name, ', '), '') as event_names,
                COALESCE(string_agg(distinct e.event_start::date::character varying, ', '), '') as event_dates,
                COALESCE(string_agg(distinct v.name, ', '), '') as venue_names,
                o.user_id,
                CONCAT(u.first_name, ' ', u.last_name) as user_name,
                CAST(
                    SUM(COALESCE(oi.quantity, 0)) FILTER (WHERE oi.item_type = 'Tickets')
                AS BIGINT) as ticket_quantity,
                CAST(
                    SUM(
                        CASE WHEN oi.item_type = 'Tickets'
                        THEN
                            COALESCE(oi.unit_price_in_cents * (oi.quantity - oi.refunded_quantity), 0)
                        ELSE
                            0
                        END
                    )
                AS BIGINT) as face_value_in_cents,
                CAST(
                    SUM(
                        CASE WHEN fi.id IS NOT NULL
                        THEN (fi.quantity - fi.refunded_quantity)
                            * COALESCE(fi.unit_price_in_cents, 0)
                        ELSE (oi.quantity - oi.refunded_quantity)
                            * (COALESCE(oi.company_fee_in_cents, 0) + COALESCE(oi.client_fee_in_cents, 0))
                        END
                    )
                AS BIGINT) as fees_in_cents
            FROM orders o
            JOIN users u on u.id = COALESCE(o.on_behalf_of_user_id, o.user_id)
            LEFT JOIN order_items oi ON o.id = oi.order_id
            LEFT JOIN order_items fi on fi.parent_id = oi.id and fi.item_type = 'PerUnitFees'
            LEFT JOIN events e ON e.id = oi.event_id
            LEFT JOIN venues v ON v.id = e.venue_id
            WHERE o.id = $1
            AND (oi.item_type = 'Tickets' OR oi.item_type = 'EventFees')
            GROUP BY o.id, o.user_id, u.first_name, u.last_name
            ;
        "#;

        #[derive(QueryableByName)]
        struct R {
            #[sql_type = "dUuid"]
            order_id: Uuid,
            #[sql_type = "Text"]
            event_names: String,
            #[sql_type = "Text"]
            event_dates: String,
            #[sql_type = "Text"]
            venue_names: String,
            #[sql_type = "dUuid"]
            user_id: Uuid,
            #[sql_type = "Text"]
            user_name: String,
            #[sql_type = "BigInt"]
            ticket_quantity: i64,
            #[sql_type = "BigInt"]
            face_value_in_cents: i64,
            #[sql_type = "BigInt"]
            fees_in_cents: i64,
        }

        let order_metadata: R = diesel::sql_query(query)
            .bind::<sql_types::Uuid, _>(self.id)
            .get_result(conn)
            .to_db_error(ErrorCode::QueryError, "Could not find order metadata")?;

        Ok(vec![
            ("order_id".to_string(), order_metadata.order_id.to_string()),
            ("event_names".to_string(), order_metadata.event_names.clone()),
            ("event_dates".to_string(), order_metadata.event_dates.clone()),
            ("venue_names".to_string(), order_metadata.venue_names.clone()),
            ("user_id".to_string(), order_metadata.user_id.to_string()),
            ("user_name".to_string(), order_metadata.user_name.clone()),
            (
                "ticket_quantity".to_string(),
                order_metadata.ticket_quantity.to_string(),
            ),
            (
                "face_value_in_cents".to_string(),
                order_metadata.face_value_in_cents.to_string(),
            ),
            ("fees_in_cents".to_string(), order_metadata.fees_in_cents.to_string()),
        ])
    }

    pub fn for_display(
        &self,
        organization_ids: Option<Vec<Uuid>>,
        user_id: Uuid,
        conn: &PgConnection,
    ) -> Result<DisplayOrder, DatabaseError> {
        let mut results = Order::load_for_display(vec![self.id], organization_ids, user_id, conn)?;

        if results.len() != 1 {
            return DatabaseError::business_process_error("Unable to load display order");
        }

        Ok(results.remove(0))
    }

    pub fn load_for_display(
        order_ids: Vec<Uuid>,
        organization_ids: Option<Vec<Uuid>>,
        current_user_id: Uuid,
        conn: &PgConnection,
    ) -> Result<Vec<DisplayOrder>, DatabaseError> {
        let current_user = User::find(current_user_id, conn)?;

        #[derive(Queryable, QueryableByName)]
        struct R {
            #[sql_type = "dUuid"]
            id: Uuid,
            #[sql_type = "Text"]
            order_number: String,
            #[sql_type = "Text"]
            status: OrderStatus,
            #[sql_type = "Timestamp"]
            order_date: NaiveDateTime,
            #[sql_type = "Nullable<Timestamp>"]
            expires_at: Option<NaiveDateTime>,
            #[sql_type = "dUuid"]
            user_id: Uuid,
            #[sql_type = "Nullable<dUuid>"]
            on_behalf_of_user_id: Option<Uuid>,
            #[sql_type = "Nullable<Timestamp>"]
            paid_at: Option<NaiveDateTime>,
            #[sql_type = "Nullable<Text>"]
            platform: Option<String>,
            #[sql_type = "Nullable<Text>"]
            checkout_url: Option<String>,
            #[sql_type = "Nullable<Timestamp>"]
            checkout_url_expires: Option<NaiveDateTime>,
            #[sql_type = "Nullable<Array<Text>>"]
            payment_methods: Option<Vec<PaymentMethods>>,
            #[sql_type = "Nullable<Array<Text>>"]
            providers: Option<Vec<PaymentProviders>>,
            #[sql_type = "BigInt"]
            total_in_cents: i64,
            #[sql_type = "BigInt"]
            total_refunded_in_cents: i64,
            #[sql_type = "Nullable<Array<Text>>"]
            allowed_payment_providers: Option<Vec<String>>,
            #[sql_type = "Nullable<Array<dUuid>>"]
            organization_ids: Option<Vec<Uuid>>,
            #[sql_type = "Nullable<Array<dUuid>>"]
            event_ids: Option<Vec<Uuid>>,
        }

        let mut query = sql_query(
            r#"
            SELECT
                o.id,
                RIGHT(o.id::text, 8) as order_number,
                o.status,
                o.order_date,
                o.expires_at,
                o.user_id,
                o.on_behalf_of_user_id,
                o.paid_at,
                o.platform,
                o.checkout_url,
                o.checkout_url_expires,
                p.payment_methods,
                p.providers,
                CAST(COALESCE(SUM(oi.unit_price_in_cents * oi.quantity), 0) as BigInt) as total_in_cents,
                CAST(COALESCE(SUM(oi.unit_price_in_cents * oi.refunded_quantity), 0) as BigInt) as total_refunded_in_cents,
                ARRAY_AGG(DISTINCT SUBSTRING(orgs.allowed_payment_providers::text from 2 for char_length(orgs.allowed_payment_providers::text) - 2)) FILTER (WHERE orgs.allowed_payment_providers IS NOT NULL) as allowed_payment_providers,
                ARRAY_AGG(DISTINCT e.organization_id) FILTER (WHERE e.organization_id IS NOT NULL) as organization_ids,
                ARRAY_AGG(DISTINCT e.id) FILTER (WHERE e.id IS NOT NULL) as event_ids
            FROM orders o
            LEFT JOIN order_items oi ON oi.order_id = o.id
            LEFT JOIN events e ON oi.event_id = e.id
            LEFT JOIN organizations orgs ON e.organization_id = orgs.id
            LEFT JOIN (
                SELECT
                    p.order_id,
                    ARRAY_AGG(DISTINCT p.provider) FILTER (WHERE p.provider IS NOT NULL) as providers,
                    ARRAY_AGG(DISTINCT p.payment_method) FILTER (WHERE p.payment_method IS NOT NULL) as payment_methods
                FROM payments p
                WHERE p.status in ('Completed', 'Refunded')
                GROUP BY p.payment_method, p.order_id
            ) AS p on o.id = p.order_id
        "#,
        )
        .into_boxed();

        query = query.sql(" WHERE o.id = ANY($1) ").bind::<Array<dUuid>, _>(&order_ids);

        query = query.sql(
            "
            GROUP BY
                o.id,
                o.status,
                o.order_date,
                o.expires_at,
                o.user_id,
                o.on_behalf_of_user_id,
                o.paid_at,
                o.platform,
                o.expires_at,
                o.checkout_url_expires,
                o.checkout_url,
                p.payment_methods,
                p.providers
            ORDER BY o.order_date desc
        ",
        );
        let results: Vec<R> = query
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Unable to load order data for organization fan")?;

        let mut user_ids = Vec::new();
        for result in &results {
            if let Some(on_behalf_of_user_id) = result.on_behalf_of_user_id {
                user_ids.push(on_behalf_of_user_id);
            }
            user_ids.push(result.user_id);
        }
        user_ids.sort();
        user_ids.dedup();
        let users = User::find_by_ids(&user_ids, conn)?;
        let mut user_map: HashMap<Uuid, DisplayUser> = HashMap::new();
        for user in users {
            user_map.insert(user.id, user.for_display()?);
        }

        let mut event_ids = Vec::new();
        for result in &results {
            if let Some(ref ids) = result.event_ids {
                event_ids.append(&mut ids.clone());
            }
        }
        event_ids.sort();
        event_ids.dedup();
        let events = Event::find_by_ids(event_ids, conn)?;
        let mut event_map: HashMap<Uuid, Event> = HashMap::new();
        for event in events {
            event_map.insert(event.id, event);
        }

        let mut order_items = Order::items_for_display(
            results.iter().map(|r| r.id).collect(),
            organization_ids.clone(),
            current_user_id,
            conn,
        )?;

        let now = Utc::now().naive_utc();
        let mut display_orders: Vec<DisplayOrder> = Vec::new();
        for result in results {
            let seconds_until_expiry = result.expires_at.map(|expires_at| {
                if expires_at >= now {
                    let duration = expires_at.signed_duration_since(now);
                    duration.num_seconds() as u32
                } else {
                    0
                }
            });

            let items = if order_items.contains_key(&result.id) {
                order_items.remove(&result.id).ok_or_else(|| {
                    DatabaseError::new(
                        ErrorCode::BusinessProcessError,
                        Some("Order can't load items".to_string()),
                    )
                })?
            } else {
                Vec::new()
            };

            let mut limited_tickets_remaining: Vec<TicketsRemaining> = Vec::new();
            if let Some(event_ids) = result.event_ids {
                for event_id in event_ids {
                    let event = event_map.get(&event_id).ok_or_else(|| {
                        DatabaseError::new(
                            ErrorCode::BusinessProcessError,
                            Some("Order can't load event data".to_string()),
                        )
                    })?;

                    if let Some(ref organization_ids) = &organization_ids {
                        if !organization_ids.contains(&event.organization_id) {
                            continue;
                        }
                    }

                    let tickets_bought = Order::quantity_for_user_for_event(
                        result.on_behalf_of_user_id.unwrap_or(result.user_id),
                        event.id,
                        conn,
                    )?;
                    for (tt_id, num) in tickets_bought {
                        let limit = TicketType::find(tt_id, conn)?.limit_per_person;
                        if limit > 0 {
                            limited_tickets_remaining.push(TicketsRemaining {
                                ticket_type_id: tt_id,
                                tickets_remaining: limit - num,
                            });
                        }
                    }
                }
            }

            let mut available_payment_providers: Vec<Vec<PaymentProviders>> = Vec::new();
            if let Some(order_allowed_payment_providers) = result.allowed_payment_providers {
                for providers in order_allowed_payment_providers {
                    let mut allowed_payment_providers: Vec<PaymentProviders> = Vec::new();
                    for provider in providers.split(",") {
                        allowed_payment_providers.push(provider.trim().parse()?);
                    }
                    available_payment_providers.push(allowed_payment_providers);
                }
            }

            let allowed_payment_methods: Vec<AllowedPaymentMethod> = intersect_set(&available_payment_providers)
                .into_iter()
                .filter_map(|p| match p {
                    PaymentProviders::Stripe => Some(AllowedPaymentMethod {
                        method: "Card".to_string(),
                        provider: PaymentProviders::Stripe,
                        display_name: "Card".to_string(),
                    }),
                    PaymentProviders::Globee => Some(AllowedPaymentMethod {
                        method: "Provider".to_string(),
                        provider: PaymentProviders::Globee,
                        display_name: "Pay with crypto".to_string(),
                    }),
                    _ => None,
                })
                .collect();

            let order_organization_ids = result.organization_ids.clone().unwrap_or(Vec::new());
            let order_contains_other_tickets = !current_user.is_admin()
                && organization_ids.is_some()
                && order_organization_ids.len()
                    != intersection(&order_organization_ids, organization_ids.as_ref().unwrap()).len();

            let user = user_map
                .get(&result.user_id)
                .ok_or_else(|| {
                    DatabaseError::new(
                        ErrorCode::BusinessProcessError,
                        Some("Order can't load user".to_string()),
                    )
                })?
                .clone();
            let on_behalf_of_user = if let Some(on_behalf_of_user_id) = result.on_behalf_of_user_id {
                Some(
                    user_map
                        .get(&on_behalf_of_user_id)
                        .ok_or_else(|| {
                            DatabaseError::new(
                                ErrorCode::BusinessProcessError,
                                Some("Order can't load user".to_string()),
                            )
                        })?
                        .clone(),
                )
            } else {
                None
            };

            display_orders.push(DisplayOrder {
                id: result.id,
                status: result.status.clone(),
                date: result.order_date,
                expires_at: result.expires_at,
                valid_for_purchase: DisplayOrder::valid_for_purchase(result.status, &items),
                limited_tickets_remaining,
                total_in_cents: result.total_in_cents,
                total_refunded_in_cents: result.total_refunded_in_cents,
                seconds_until_expiry,
                user_id: result.user_id,
                user,
                order_number: result.order_number,
                paid_at: result.paid_at,
                checkout_url: if result
                    .checkout_url_expires
                    .unwrap_or(NaiveDateTime::from_timestamp(0, 0))
                    > Utc::now().naive_utc()
                {
                    result.checkout_url.clone()
                } else {
                    None
                },
                allowed_payment_methods,
                order_contains_other_tickets,
                platform: result.platform.clone(),
                is_box_office: result.on_behalf_of_user_id.is_some(),
                payment_method: result.payment_methods.map(|r| r.first().map(|p| *p)).unwrap_or(None),
                payment_provider: result.providers.map(|r| r.first().map(|p| *p)).unwrap_or(None),
                on_behalf_of_user_id: result.on_behalf_of_user_id,
                on_behalf_of_user,
                items,
            });
        }

        let mut iter = order_ids.iter();
        display_orders.sort_by_key(|order| iter.position(|&id| id == order.id));
        Ok(display_orders)
    }

    pub fn items_for_display(
        order_ids: Vec<Uuid>,
        organization_ids: Option<Vec<Uuid>>,
        user_id: Uuid,
        conn: &PgConnection,
    ) -> Result<HashMap<Uuid, Vec<DisplayOrderItem>>, DatabaseError> {
        OrderItem::find_for_display(order_ids, organization_ids, user_id, conn)
    }

    pub fn find_item(&self, cart_item_id: Uuid, conn: &PgConnection) -> Result<OrderItem, DatabaseError> {
        OrderItem::find_in_order(self.id, cart_item_id, conn)
    }

    pub fn find_item_by_type(
        &self,
        ticket_type_id: Uuid,
        item_type: OrderItemTypes,
        conn: &PgConnection,
    ) -> Result<OrderItem, DatabaseError> {
        let items = self.items(conn)?;
        let mut order_item: Vec<OrderItem> = items
            .into_iter()
            .filter(|i| i.ticket_type_id == Some(ticket_type_id) && i.item_type == item_type)
            .collect();

        match order_item.pop() {
            Some(o) => Ok(o),
            None => Err(DatabaseError::new(
                ErrorCode::NoResults,
                Some("Could not find item".to_string()),
            )),
        }
    }

    pub fn create_note(&self, note: String, user_id: Uuid, conn: &PgConnection) -> Result<Note, DatabaseError> {
        Note::create(Tables::Orders, self.id, user_id, note).commit(conn)
    }

    pub fn set_external_payment_type(
        &mut self,
        external_payment_type: ExternalPaymentType,
        current_user_id: Uuid,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        if self.status != OrderStatus::Draft {
            return DatabaseError::validation_error(
                "status",
                "Cannot change the order user unless the order is in draft status",
            );
        }

        self.lock_version(conn)?;
        self.external_payment_type = Some(external_payment_type);
        diesel::update(&*self)
            .set((
                orders::external_payment_type.eq(self.external_payment_type),
                orders::updated_at.eq(dsl::now),
            ))
            .execute(conn)
            .to_db_error(
                ErrorCode::UpdateError,
                "Could not change the external payment type for this order",
            )?;

        DomainEvent::create(
            DomainEventTypes::OrderUpdated,
            "External payment type information recorded on order".to_string(),
            Tables::Orders,
            Some(self.id),
            Some(current_user_id),
            Some(json!({ "external_payment_type": external_payment_type })),
        )
        .commit(conn)?;
        Ok(())
    }

    pub fn add_free_payment(
        &mut self,
        external_payment: bool,
        current_user_id: Uuid,
        conn: &PgConnection,
    ) -> Result<Payment, DatabaseError> {
        if external_payment {
            self.set_external_payment_type(ExternalPaymentType::Voucher, current_user_id, conn)?;
        }

        let payment = Payment::create(
            self.id,
            Some(current_user_id),
            PaymentStatus::Completed,
            PaymentMethods::Free,
            if external_payment {
                PaymentProviders::External
            } else {
                PaymentProviders::Free
            },
            Some("Free Checkout".to_string()),
            0,
            None,
            None,
            None,
        );
        self.add_payment(payment, Some(current_user_id), conn)
    }

    pub fn add_external_payment(
        &mut self,
        external_reference: Option<String>,
        external_payment_type: ExternalPaymentType,
        current_user_id: Uuid,
        amount: i64,
        conn: &PgConnection,
    ) -> Result<Payment, DatabaseError> {
        self.set_external_payment_type(external_payment_type, current_user_id, conn)?;

        let payment = Payment::create(
            self.id,
            Some(current_user_id),
            PaymentStatus::Completed,
            PaymentMethods::External,
            PaymentProviders::External,
            external_reference,
            amount,
            None,
            None,
            None,
        );
        self.add_payment(payment, Some(current_user_id), conn)
    }

    pub fn add_provider_payment(
        &mut self,
        external_reference: Option<String>,
        provider: PaymentProviders,
        current_user_id: Option<Uuid>,
        amount: i64,
        status: PaymentStatus,
        url_nonce: Option<String>,
        data: Value,
        conn: &PgConnection,
    ) -> Result<Payment, DatabaseError> {
        let payment = Payment::create(
            self.id,
            current_user_id,
            status,
            PaymentMethods::Provider,
            provider,
            external_reference,
            amount,
            Some(data),
            url_nonce,
            None,
        );

        self.add_payment(payment, current_user_id, conn)
    }

    pub fn add_credit_card_payment(
        &mut self,
        current_user_id: Uuid,
        amount: i64,
        provider: PaymentProviders,
        external_reference: String,
        status: PaymentStatus,
        provider_data: serde_json::Value,
        conn: &PgConnection,
    ) -> Result<Payment, DatabaseError> {
        let payment = Payment::create(
            self.id,
            Some(current_user_id),
            status,
            PaymentMethods::CreditCard,
            provider,
            Some(external_reference),
            amount,
            Some(provider_data),
            None,
            None,
        );

        self.add_payment(payment, Some(current_user_id), conn)
    }

    pub fn user(&self, conn: &PgConnection) -> Result<User, DatabaseError> {
        users::table
            .filter(users::id.eq(self.on_behalf_of_user_id.unwrap_or(self.user_id)))
            .first(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load user")
    }

    pub fn add_checkout_url(
        &mut self,
        current_user_id: Uuid,
        checkout_url: String,
        expires: NaiveDateTime,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        self.checkout_url = Some(checkout_url.clone());
        self.set_expiry(Some(current_user_id), Some(expires), false, conn)?;
        diesel::update(&*self)
            .set((
                orders::checkout_url.eq(&self.checkout_url),
                orders::checkout_url_expires.eq(expires),
                orders::updated_at.eq(dsl::now),
            ))
            .execute(conn)
            .to_db_error(ErrorCode::UpdateError, "Could not add checkout URL")?;

        DomainEvent::create(
            DomainEventTypes::OrderUpdated,
            format!("Order checkout URL added {:?}", &checkout_url),
            Tables::Orders,
            Some(self.id),
            Some(current_user_id),
            Some(json!({ "checkout_url": &checkout_url })),
        )
        .commit(conn)?;

        Ok(())
    }

    pub fn order_items_in_invalid_state(&self, conn: &PgConnection) -> Result<Vec<OrderItem>, DatabaseError> {
        let query = include_str!("../queries/order_items_in_invalid_state.sql");
        diesel::sql_query(query)
            .bind::<dUuid, _>(self.id)
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load invalid order items")
    }

    pub fn items_valid_for_purchase(&self, conn: &PgConnection) -> Result<bool, DatabaseError> {
        let invalid_items = self.order_items_in_invalid_state(conn)?;
        Ok(invalid_items.is_empty())
    }

    pub fn reset_to_draft(&mut self, current_user_id: Option<Uuid>, conn: &PgConnection) -> Result<(), DatabaseError> {
        match self.status {
            OrderStatus::Paid => {
                // still store the payment.
                DatabaseError::business_process_error("Cannot reset to draft, the order is already paid")
            }
            OrderStatus::Cancelled => {
                DatabaseError::business_process_error("Cannot reset this order because it has been cancelled")
            }

            OrderStatus::Draft => Ok(()),
            OrderStatus::PendingPayment => self.update_status(current_user_id, OrderStatus::Draft, conn),
        }
    }

    fn add_payment(
        &mut self,
        payment: NewPayment,
        current_user_id: Option<Uuid>,
        conn: &PgConnection,
    ) -> Result<Payment, DatabaseError> {
        // Confirm codes are still valid
        for item in self.items(conn)? {
            item.confirm_code_valid(conn)?;
        }

        let p = payment.commit(current_user_id, conn)?;
        if p.status != PaymentStatus::Requested {
            self.clear_user_cart(conn)?;
        }

        self.complete_if_fully_paid(current_user_id, conn)?;
        Ok(p)
    }

    pub(crate) fn complete_if_fully_paid(
        &mut self,
        current_user_id: Option<Uuid>,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        if self.status == OrderStatus::Paid {
            return Ok(());
        }

        let total_paid = self.total_paid(conn)?;
        let total_required = self.calculate_total(conn)?;
        if total_paid >= total_required {
            self.update_status(current_user_id, OrderStatus::Paid, conn)?;
            //Mark tickets as Purchased
            let order_items = OrderItem::find_for_order(self.id, conn)?;
            for item in order_items
                .iter()
                .filter(|oi| oi.item_type == OrderItemTypes::Tickets)
                .collect_vec()
            {
                TicketInstance::mark_as_purchased(item, self.on_behalf_of_user_id.unwrap_or(self.user_id), conn)?;
            }

            let ticket_ids = TicketInstance::find_ids_for_order(self.id, conn)?;
            let domain_event = DomainEvent::create(
                DomainEventTypes::OrderCompleted,
                "Order completed".into(),
                Tables::Orders,
                Some(self.id),
                current_user_id,
                Some(json!({ "ticket_ids": ticket_ids })),
            )
            .commit(conn)?;

            let mut action = DomainAction::create(
                Some(domain_event.id),
                DomainActionTypes::SendPurchaseCompletedCommunication,
                None,
                json!({"order_id": self.id, "user_id": current_user_id}),
                Some(Tables::Orders),
                Some(self.id),
            );
            action.expires_at = action.scheduled_at.into_builder().add_days(3).finish();
            action.commit(conn)?;

            self.user(&conn)?.update_genre_info(conn)?;
            Ok(())
        } else {
            jlog!(Debug, "Order was checked for completion but was short", {"required_amount": total_required, "total_paid": total_paid, "order_id": self.id});
            Ok(())
        }
    }

    fn clear_user_cart(&mut self, conn: &PgConnection) -> Result<(), DatabaseError> {
        let cart_user: Option<User> = users::table
            .filter(users::last_cart_id.eq(self.id))
            .get_result(conn)
            .to_db_error(ErrorCode::QueryError, "Could not find user attached to this cart")
            .optional()?;
        if let Some(user) = cart_user {
            user.update_last_cart(None, conn)?;
        }
        Ok(())
    }

    pub fn total_paid(&self, conn: &PgConnection) -> Result<i64, DatabaseError> {
        #[derive(QueryableByName)]
        struct ResultForSum {
            #[sql_type = "Nullable<BigInt>"]
            s: Option<i64>,
        };
        let query = diesel::sql_query(
            "SELECT CAST(SUM(amount) as BigInt) as s FROM payments WHERE order_id = $1 AND status='Completed';",
        )
        .bind::<diesel::sql_types::Uuid, _>(self.id);

        let sum: ResultForSum = query
            .get_result(conn)
            .to_db_error(ErrorCode::QueryError, "Could not get total payments for order")?;
        Ok(sum.s.unwrap_or(0))
    }

    pub fn clear_invalid_items(&mut self, user_id: Uuid, conn: &PgConnection) -> Result<(), DatabaseError> {
        if self.status != OrderStatus::Draft {
            return DatabaseError::validation_error(
                "status",
                "Cannot clear invalid items unless the order is in draft status",
            );
        }

        self.lock_version(conn)?;

        let order_items = self.order_items_in_invalid_state(conn)?;
        for item in order_items {
            // Use calculated quantity as reserved may have been taken in the meantime
            let quantity = item.calculate_quantity(conn)?;
            TicketInstance::release_tickets(&item, quantity as u32, Some(user_id), conn)?;
            self.destroy_item(item.id, conn)?;
        }

        Ok(())
    }

    fn update_box_office_pricing(
        &mut self,
        box_office_pricing: bool,
        current_user_id: Uuid,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        if self.status != OrderStatus::Draft {
            return DatabaseError::validation_error(
                "status",
                "Cannot change the box office pricing unless the order is in draft status",
            );
        }

        let old_box_office_pricing = self.box_office_pricing;
        self.box_office_pricing = box_office_pricing;
        jlog!(Debug, "Changing order to use box office pricing", { "order_id": self.id});
        diesel::update(&*self)
            .set((
                orders::box_office_pricing.eq(&self.box_office_pricing),
                orders::updated_at.eq(dsl::now),
            ))
            .execute(conn)
            .to_db_error(ErrorCode::UpdateError, "Could not update order")?;

        DomainEvent::create(
            DomainEventTypes::OrderUpdated,
            format!(
                "Order box office pricing updated from {:?} to {:?}",
                old_box_office_pricing, box_office_pricing
            ),
            Tables::Orders,
            Some(self.id),
            Some(current_user_id),
            Some(json!({
                "old_box_office_pricing": old_box_office_pricing,
                "new_box_office_pricing": box_office_pricing
            })),
        )
        .commit(conn)?;
        Ok(())
    }

    pub(crate) fn update_status(
        &mut self,
        current_user_id: Option<Uuid>,
        status: OrderStatus,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        let old_status = self.status;
        self.status = status;

        if status == OrderStatus::Paid {
            self.paid_at = Some(Utc::now().naive_utc());
            diesel::update(&*self)
                .set((
                    orders::paid_at.eq(self.paid_at),
                    orders::status.eq(&self.status),
                    orders::updated_at.eq(dsl::now),
                ))
                .execute(conn)
                .to_db_error(ErrorCode::UpdateError, "Could not mark order paid")?;
        } else {
            diesel::update(&*self)
                .set((orders::status.eq(&self.status), orders::updated_at.eq(dsl::now)))
                .execute(conn)
                .to_db_error(ErrorCode::UpdateError, "Could not update order status")?;
        }

        DomainEvent::create(
            DomainEventTypes::OrderStatusUpdated,
            format!("Order status changed from {} to {}", &old_status, &status),
            Tables::Orders,
            Some(self.id),
            current_user_id,
            Some(json!({
                "old_status": &old_status,
                "new_status": &status
            })),
        )
        .commit(conn)?;

        Ok(())
    }

    pub fn calculate_total(&self, conn: &PgConnection) -> Result<i64, DatabaseError> {
        Ok(self.calculate_total_and_refunded_total(conn)?.0)
    }

    pub fn calculate_total_and_refunded_total(&self, conn: &PgConnection) -> Result<(i64, i64), DatabaseError> {
        let order_items = self.items(conn)?;
        let mut total = 0;
        let mut refunded_total = 0;

        for item in &order_items {
            total += item.unit_price_in_cents * item.quantity;
            refunded_total += item.unit_price_in_cents * item.refunded_quantity;
        }

        Ok((total, refunded_total))
    }

    /// Updates the lock version in the database and forces a Concurrency error if
    /// another process has updated it. It will also keep a row lock on the order
    /// preventing multiple processes from updating data until it has finished
    pub fn lock_version(&mut self, conn: &PgConnection) -> Result<(), DatabaseError> {
        let rows_affected = diesel::update(
            orders::table
                .filter(orders::id.eq(self.id))
                .filter(orders::version.eq(self.version)),
        )
        .set((orders::version.eq(self.version + 1), orders::updated_at.eq(dsl::now)))
        .execute(conn)
        .to_db_error(ErrorCode::UpdateError, "Could not lock order")?;
        if rows_affected == 0 {
            return DatabaseError::concurrency_error("Could not lock order, another process has updated it");
        }
        self.version = self.version + 1;
        Ok(())
    }

    pub fn set_behalf_of_user(
        &mut self,
        user: User,
        current_user_id: Uuid,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        if self.status != OrderStatus::Draft {
            return DatabaseError::validation_error(
                "status",
                "Cannot change the order user unless the order is in draft status",
            );
        }

        self.lock_version(conn)?;

        let old_id = self.on_behalf_of_user_id;
        self.on_behalf_of_user_id = Some(user.id);
        diesel::update(&*self)
            .set((
                orders::on_behalf_of_user_id.eq(user.id),
                orders::updated_at.eq(dsl::now),
            ))
            .execute(conn)
            .to_db_error(
                ErrorCode::UpdateError,
                "Could not change the behalf of user for this order",
            )?;

        DomainEvent::create(
            DomainEventTypes::OrderBehalfOfUserChanged,
            "Behalf of user on order was changed".to_string(),
            Tables::Orders,
            Some(self.id),
            Some(current_user_id),
            Some(json!({
            "old_user" : old_id, "new_user": user.id
            })),
        )
        .commit(conn)?;
        Ok(())
    }
}

#[derive(QueryableByName, Deserialize, Serialize, Debug)]
pub struct ResultForTicketTypeTotal {
    #[sql_type = "Nullable<dUuid>"]
    ticket_type_id: Option<Uuid>,
    #[sql_type = "Integer"]
    total_quantity: i32,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct TicketsRemaining {
    pub ticket_type_id: Uuid,
    pub tickets_remaining: i32,
}

#[derive(Deserialize, Serialize)]
pub struct DisplayOrder {
    pub id: Uuid,
    pub date: NaiveDateTime,
    pub expires_at: Option<NaiveDateTime>,
    pub seconds_until_expiry: Option<u32>,
    pub status: OrderStatus,
    pub items: Vec<DisplayOrderItem>,
    pub limited_tickets_remaining: Vec<TicketsRemaining>,
    pub total_in_cents: i64,
    pub total_refunded_in_cents: i64,
    pub user_id: Uuid,
    pub user: DisplayUser,
    pub order_number: String,
    pub paid_at: Option<NaiveDateTime>,
    pub checkout_url: Option<String>,
    pub allowed_payment_methods: Vec<AllowedPaymentMethod>,
    pub order_contains_other_tickets: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_for_purchase: Option<bool>,
    pub platform: Option<String>,
    pub is_box_office: bool,
    pub payment_method: Option<PaymentMethods>,
    pub payment_provider: Option<PaymentProviders>,
    pub on_behalf_of_user: Option<DisplayUser>,
    pub on_behalf_of_user_id: Option<Uuid>,
}

impl DisplayOrder {
    pub fn valid_for_purchase(status: OrderStatus, items: &Vec<DisplayOrderItem>) -> Option<bool> {
        if status != OrderStatus::Draft {
            return None;
        }

        Some(
            !items
                .iter()
                .find(|i| i.cart_item_status.is_some() && i.cart_item_status != Some(CartItemStatus::Valid))
                .is_some(),
        )
    }
}

#[derive(Serialize, Deserialize)]
pub struct AllowedPaymentMethod {
    method: String,
    provider: PaymentProviders,
    display_name: String,
}

#[derive(Deserialize, Serialize, PartialEq, Debug)]
pub struct UpdateOrderItem {
    pub ticket_type_id: Uuid,
    pub quantity: u32,
    pub redemption_code: Option<String>,
}

#[test]
fn parse_order_number() {
    let id = Uuid::parse_str("01234567-1234-1234-1234-1234567890ab").unwrap();
    assert_eq!("567890ab".to_string(), Order::parse_order_number(id));
}
