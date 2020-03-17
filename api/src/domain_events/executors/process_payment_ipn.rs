use crate::config::Config;
use crate::db::Connection;
use crate::domain_events::executor_future::ExecutorFuture;
use crate::domain_events::routing::DomainActionExecutor;
use crate::errors::ApplicationError;
use crate::errors::BigNeonError;
use bigneon_db::prelude::*;
use futures::future::TryFutureExt;
use globee::GlobeeClient;
use globee::GlobeeIpnRequest;
use log::Level::{Debug, Error};
use uuid::Uuid;

#[derive(Clone)]
pub struct ProcessPaymentIPNExecutor {
    globee_api_key: String,
    globee_base_url: String,
    validate_ipn: bool,
    api_keys_encryption_key: String,
}

impl DomainActionExecutor for ProcessPaymentIPNExecutor {
    fn execute(&self, action: DomainAction, conn: Connection) -> ExecutorFuture {
        let fut = self.clone().perform_job(action.clone(), conn.clone())
            .inspect_err({
                let action = action.clone();
                move |e| jlog!(Error, "Payment IPN processor failed", {"action_id": action.id, "main_table_id":action.main_table_id,  "error": e.to_string()})
            });
        ExecutorFuture::new(action, conn, Box::pin(fut))
    }
}

impl ProcessPaymentIPNExecutor {
    pub fn new(config: &Config) -> ProcessPaymentIPNExecutor {
        ProcessPaymentIPNExecutor {
            globee_api_key: config.globee_api_key.clone(),
            globee_base_url: config.globee_base_url.clone(),
            validate_ipn: config.validate_ipns,
            api_keys_encryption_key: config.api_keys_encryption_key.clone(),
        }
    }

    pub async fn perform_job(self, action: DomainAction, conn: Connection) -> Result<(), BigNeonError> {
        let mut ipn: GlobeeIpnRequest = serde_json::from_value(action.payload.clone())?;
        if ipn.custom_payment_id.is_none() {
            return Err(
                ApplicationError::new("Invalid IPN, the custom_payment_id must be specified".to_string()).into(),
            );
        }

        let order_id = Uuid::parse_str(ipn.custom_payment_id.as_ref().ok_or(ApplicationError::new(
            "Globee response did not include a custom_payment_id".to_string(),
        ))?)?;

        let connection = conn.get();
        let mut order = Order::find(order_id, connection)?;
        let mut organizations = order.organizations(connection)?;
        if organizations.len() != 1 {
            return Err(ApplicationError::new(
                "Orders containing more than one organization are not supported".to_string(),
            )
            .into());
        };
        let mut organization = organizations.remove(0);
        organization.decrypt(&self.api_keys_encryption_key)?;
        let api_key = organization.globee_api_key.unwrap_or(self.globee_api_key.clone());

        let client = GlobeeClient::new(api_key.clone(), self.globee_base_url.clone());

        if self.validate_ipn {
            ipn = client.get_payment_request(&ipn.id).await?;
        }

        if ipn
            .custom_payment_id
            .as_ref()
            .map(|r| Uuid::parse_str(r).unwrap_or(Uuid::nil()))
            != Some(order_id)
        {
            return Err(ApplicationError::new("Invalid IPN, the custom_payment_id has changed".to_string()).into());
        }
        // Refetch the order in case it's been spoofed.

        let order_id = Uuid::parse_str(ipn.custom_payment_id.as_ref().ok_or(ApplicationError::new(
            "Globee response did not include a custom_payment_id".to_string(),
        ))?)?;

        order = Order::find(order_id, connection)?;

        // Lock the order to prevent other processes from adding/updating payments.
        // This is a fairly heavy way of locking, but it should prevent deadlocks
        // of processes trying to update orders and payments in different orders
        order.lock_version(connection)?;

        // If expired attempt to refresh cart
        if order.is_expired() && (order.status == OrderStatus::PendingPayment || order.status == OrderStatus::Draft) {
            match order.try_refresh_expired_cart(None, connection) {
                Ok(_) => jlog!(Debug, "IPN: refreshed expired cart", {"ipn_id": ipn.id, "order_id": order.id}),
                Err(_) => {
                    jlog!(Debug, "IPN: Attempted to refresh expired cart but failed", {"ipn_id": ipn.id, "order_id": order.id})
                }
            }
        }

        jlog!(Debug, "Found IPN", {"ipn_id": ipn.id, "order_id": order_id});

        let external_reference = format!("globee-{}", ipn.id);
        let status = match ipn.status.clone().unwrap_or("none".to_string()).to_lowercase().as_str() {
            "unpaid" => PaymentStatus::Unpaid,
            "paid" => PaymentStatus::PendingConfirmation,
            "overpaid" => PaymentStatus::PendingConfirmation,
            "underpaid" => PaymentStatus::PendingConfirmation,
            "paid_late" => PaymentStatus::PendingConfirmation,
            "confirmed" => PaymentStatus::Completed,
            "completed" => PaymentStatus::Completed,
            "refunded" => PaymentStatus::Refunded,
            "cancelled" => PaymentStatus::Cancelled,
            "draft" => PaymentStatus::Draft,
            _ => PaymentStatus::Unknown,
        };

        jlog!(Debug, &format!("IPN status:{}", status), {"ipn_id": ipn.id, "order_id": order_id, "status": status});

        let payment = match Payment::find_by_order(order_id, &external_reference, connection).optional()? {
            Some(p) => {
                jlog!(Debug, "IPN: Payment found,updating", {"ipn_id": ipn.id, "order_id": order_id, "status": status});
                p
            }
            None => {
                jlog!(Debug, "IPN: No payment found, creating new payment", {"ipn_id": ipn.id, "order_id": order_id, "status": status});

                order.add_provider_payment(
                    Some(external_reference.to_string()),
                    PaymentProviders::Globee,
                    None,
                    (ipn.payment_details.received_amount.unwrap_or(0f64) * 100f64) as i64,
                    status,
                    None,
                    action.payload.clone(),
                    connection,
                )?
            }
        };

        // Sometimes the IPN will come in before the user has been redirected back to the success page
        if order.status == OrderStatus::Draft {
            payment.mark_pending_ipn(None, connection)?;
        }

        if status == PaymentStatus::Completed {
            jlog!(Debug, &format!("IPN: Payment completed, updating amount received from {:?} to {:?}", payment.amount, ipn.payment_details.received_amount), {"ipn_id": ipn.id, "order_id": order_id, "status": status});

            payment.update_amount(
                None,
                (ipn.payment_details.received_amount.unwrap_or(0f64) * 100f64).round() as i64,
                connection,
            )?;
            jlog!(Debug, "IPN: Marking payment complete", {"ipn_id": ipn.id, "order_id": order_id, "status": status});

            payment.mark_complete(json!(ipn), None, connection)?;
        } else {
            jlog!(Debug, "IPN: Payment not yet completed, just recording for now", {"ipn_id": ipn.id, "order_id": order_id, "status": status});

            payment.add_ipn(status, json!(ipn), None, connection)?;
        }

        Ok(())
    }
}
