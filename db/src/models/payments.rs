use chrono::Duration;
use chrono::NaiveDateTime;
use chrono::Utc;
use diesel;
use diesel::expression::dsl;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use models::*;
use schema::{orders, payments};
use serde_json;
use utils::errors::*;
use uuid::Uuid;

#[allow(dead_code)]
#[derive(Debug, Identifiable, PartialEq, Queryable)]
pub struct Payment {
    pub id: Uuid,
    pub order_id: Uuid,
    created_by: Option<Uuid>,
    pub status: PaymentStatus,
    pub payment_method: PaymentMethods,
    pub amount: i64,
    pub provider: PaymentProviders,
    pub external_reference: Option<String>,
    raw_data: Option<serde_json::Value>,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
    pub url_nonce: Option<String>,
    pub refund_id: Option<Uuid>,
}

impl Payment {
    pub(crate) fn create(
        order_id: Uuid,
        created_by: Option<Uuid>,
        status: PaymentStatus,
        payment_method: PaymentMethods,
        provider: PaymentProviders,
        external_reference: Option<String>,
        amount: i64,
        raw_data: Option<serde_json::Value>,
        url_nonce: Option<String>,
        refund_id: Option<Uuid>,
    ) -> NewPayment {
        NewPayment {
            order_id,
            created_by,
            status,
            payment_method,
            provider,
            external_reference,
            amount,
            raw_data,
            url_nonce,
            refund_id,
        }
    }

    pub fn find_all_with_orders_paginated_by_provider(
        provider: PaymentProviders,
        page: i64,
        limit: i64,
        conn: &PgConnection,
    ) -> Result<Vec<(Payment, Order)>, DatabaseError> {
        payments::table
            .inner_join(orders::table.on(orders::id.eq(payments::order_id)))
            .filter(payments::provider.eq(provider))
            .filter(payments::status.eq(PaymentStatus::Completed))
            .limit(limit)
            .offset(page * limit)
            .select((payments::all_columns, orders::all_columns))
            .load(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load payments")
    }

    pub fn find(id: Uuid, conn: &PgConnection) -> Result<Payment, DatabaseError> {
        payments::table
            .filter(payments::id.eq(id))
            .first(conn)
            .to_db_error(ErrorCode::QueryError, "Could not find payment")
    }

    pub fn find_by_order(
        order_id: Uuid,
        external_reference: &str,
        conn: &PgConnection,
    ) -> Result<Payment, DatabaseError> {
        payments::table
            .filter(
                payments::order_id
                    .eq(order_id)
                    .and(payments::external_reference.eq(external_reference)),
            )
            .load(conn)
            .to_db_error(ErrorCode::QueryError, "Could not find payment for order")
            .expect_single()
    }

    pub fn log_refund(
        &self,
        current_user_id: Uuid,
        refund: &Refund,
        refund_amount: i64,
        refund_data: Option<serde_json::Value>,
        conn: &PgConnection,
    ) -> Result<Payment, DatabaseError> {
        let refund_payment = Payment::create(
            self.order_id,
            self.created_by,
            PaymentStatus::Refunded,
            self.payment_method,
            self.provider.clone(),
            self.external_reference.clone(),
            -refund_amount,
            refund_data.clone(),
            None,
            Some(refund.id),
        )
        .commit(Some(current_user_id), conn)?;

        DomainEvent::create(
            DomainEventTypes::PaymentRefund,
            "Payment was refunded".to_string(),
            Tables::Payments,
            Some(self.id),
            Some(current_user_id),
            refund_data,
        )
        .commit(conn)?;
        Ok(refund_payment)
    }

    pub fn add_ipn(
        &self,
        new_status: PaymentStatus,
        raw_data: serde_json::Value,
        current_user_id: Option<Uuid>,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        self.update_status(new_status, current_user_id, conn)?;
        DomainEvent::create(
            DomainEventTypes::PaymentProviderIPN,
            "Payment IPN received".to_string(),
            Tables::Payments,
            Some(self.id),
            current_user_id,
            Some(raw_data),
        )
        .commit(conn)?;

        Ok(())
    }

    fn update_status(
        &self,
        status: PaymentStatus,
        current_user_id: Option<Uuid>,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        diesel::update(payments::table.filter(payments::id.eq(self.id)))
            .set((payments::status.eq(status), payments::updated_at.eq(dsl::now)))
            .execute(conn)
            .to_db_error(ErrorCode::UpdateError, "Could not change the status of payment")?;

        DomainEvent::create(
            DomainEventTypes::PaymentUpdated,
            format!("Payment status updated to {}", status),
            Tables::Payments,
            Some(self.id),
            current_user_id,
            Some(json!({ "new_status": status })),
        )
        .commit(conn)?;

        Ok(())
    }

    pub fn update_amount(
        &self,
        current_user_id: Option<Uuid>,
        amount: i64,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        let old_amount = self.amount;

        diesel::update(self)
            .set((payments::amount.eq(&amount), payments::updated_at.eq(dsl::now)))
            .execute(conn)
            .to_db_error(ErrorCode::UpdateError, "Could not update amount on payment")?;
        DomainEvent::create(
            DomainEventTypes::PaymentUpdated,
            "Payment Amount was updated".to_string(),
            Tables::Payments,
            Some(self.id),
            current_user_id,
            Some(json!({
            "old_amount": old_amount, "new_amount": amount
            })),
        )
        .commit(conn)?;

        Ok(())
    }

    pub fn mark_complete(
        &self,
        raw_data: serde_json::Value,
        current_user_id: Option<Uuid>,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        if self.status != PaymentStatus::Completed {
            self.update_status(PaymentStatus::Completed, current_user_id, conn)?;

            DomainEvent::create(
                DomainEventTypes::PaymentCompleted,
                "Payment was completed".to_string(),
                Tables::Payments,
                Some(self.id),
                current_user_id,
                Some(raw_data),
            )
            .commit(conn)?;
        }
        self.order(conn)?.complete_if_fully_paid(current_user_id, conn)?;

        Ok(())
    }
    pub fn mark_pending_ipn(&self, current_user_id: Option<Uuid>, conn: &PgConnection) -> Result<(), DatabaseError> {
        if self.status != PaymentStatus::Completed {
            self.update_status(PaymentStatus::PendingIpn, current_user_id, conn)?;
            let mut order = self.order(conn)?;
            order.update_status(current_user_id, OrderStatus::PendingPayment, conn)?;
            order.set_expiry(
                current_user_id,
                Some(Utc::now().naive_utc() + Duration::minutes(120)),
                false,
                conn,
            )?;
        }
        Ok(())
    }

    pub fn mark_cancelled(
        &self,
        raw_data: serde_json::Value,
        current_user_id: Option<Uuid>,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        use models::enums::PaymentStatus::*;
        match self.status {
            Completed | Authorized | Refunded | PendingConfirmation => DatabaseError::business_process_error(
                "Could not mark payment as cancelled because it is in a status that doesn't allow cancelling",
            ),
            Requested | Unpaid | Draft | Unknown | PendingIpn => {
                DomainEvent::create(
                    DomainEventTypes::PaymentCancelled,
                    "Payment was cancelled".to_string(),
                    Tables::Payments,
                    Some(self.id),
                    current_user_id,
                    Some(raw_data),
                )
                .commit(conn)?;
                self.update_status(Cancelled, current_user_id, conn)
            }
            Cancelled => Ok(()),
        }
    }

    fn order(&self, conn: &PgConnection) -> Result<Order, DatabaseError> {
        use schema::*;
        orders::table
            .find(self.order_id)
            .get_result(conn)
            .to_db_error(ErrorCode::QueryError, "Could not retrieve order")
    }
}

#[derive(Insertable)]
#[table_name = "payments"]
pub struct NewPayment {
    order_id: Uuid,
    created_by: Option<Uuid>,
    pub status: PaymentStatus,
    payment_method: PaymentMethods,
    external_reference: Option<String>,
    amount: i64,
    provider: PaymentProviders,
    raw_data: Option<serde_json::Value>,
    url_nonce: Option<String>,
    refund_id: Option<Uuid>,
}

impl NewPayment {
    pub(crate) fn commit(self, current_user_id: Option<Uuid>, conn: &PgConnection) -> Result<Payment, DatabaseError> {
        let res: Payment = diesel::insert_into(payments::table)
            .values(&self)
            .get_result(conn)
            .to_db_error(ErrorCode::InsertError, "Could not create payment")?;

        DomainEvent::create(
            DomainEventTypes::PaymentCreated,
            "Payment created".to_string(),
            Tables::Payments,
            Some(res.id),
            current_user_id,
            self.raw_data,
        )
        .commit(conn)?;
        Ok(res)
    }
}
