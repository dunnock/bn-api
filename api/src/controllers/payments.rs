use crate::db::Connection;
use actix_web::HttpResponse;
use actix_web::web::{Path, Query, Data};
use bigneon_db::prelude::*;
use crate::errors::*;
use crate::extractors::OptionalUser;
use crate::helpers::application;
use log::Level::Debug;
use crate::server::AppState;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub struct QueryParams {
    pub success: bool,
}

// The nonce is in the path so that it is not cached.
#[derive(Serialize, Deserialize, Debug)]
pub struct PathParams {
    pub nonce: String,
    pub id: Uuid,
}

pub fn callback(
    (query, path, connection, state, user): (
        Query<QueryParams>,
        Path<PathParams>,
        Connection,
        Data<AppState>,
        OptionalUser,
    ),
) -> Result<HttpResponse, BigNeonError> {
    let conn = connection.get();
    let mut order = Order::find(path.id, conn)?;
    // Try to get a lock. IPNs might come in quickly, so try a few times
    for _ in 0..5 {
        match order.lock_version(conn) {
            Ok(_) => break,
            Err(err) => match err.error_code {
                ErrorCode::ConcurrencyError => {
                    // Get the latest order and try again...
                    order = Order::find(path.id, conn)?;
                }
                _ => return Err(err.into()),
            },
        }
    }

    let mut payments: Vec<Payment> = order
        .payments(conn)?
        .into_iter()
        .filter(|p| p.url_nonce.as_ref() == Some(&path.nonce))
        .collect();

    let payment = payments.pop();
    let payment = match payment {
        Some(p) => p,
        None => return application::not_found(),
    };

    // We specifically don't count this as a payment confirmation, that will be done via the IPN
    // Just redirect to page accordingly

    if query.success {
        if payment.status == PaymentStatus::Requested {
            // If expired attempt to refresh cart
            if order.is_expired() {
                match order.try_refresh_expired_cart(user.id(), conn) {
                    Ok(_) => jlog!(Debug, "Payments: refreshed expired cart", {"payment_id": payment.id, "order_id": order.id}),
                    Err(_) => jlog!(Debug, "Payments: Attempted to refresh expired cart but failed", {"payment_id": payment.id, "order_id": order.id}),
                }
            }
            payment.mark_pending_ipn(user.id(), conn)?;
        }
        application::redirect(&format!(
            "{}/tickets/{}/tickets/success?order_id={}",
            state.config.front_end_url,
            order.event_slug(conn)?,
            order.id
        ))
    } else {
        payment.mark_cancelled(
            json!({"path": &path.into_inner(), "query": &query.into_inner()}),
            None,
            conn,
        )?;
        // order.reset_to_draft(None, conn)?;
        application::redirect(&format!(
            "{}/tickets/{}/tickets/confirmation",
            state.config.front_end_url,
            order.event_slug(conn)?
        ))
    }
}
