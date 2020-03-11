use crate::db::Connection;
use crate::errors::BigNeonError;
use crate::extractors::Json;
use actix_web::HttpResponse;
use bigneon_db::prelude::*;
use bigneon_db::utils::dates::IntoDateBuilder;
use globee::GlobeeIpnRequest;
use log::Level::Debug;
use uuid::Uuid;

pub async fn globee((data, conn): (Json<GlobeeIpnRequest>, Connection)) -> Result<HttpResponse, BigNeonError> {
    let data = data.into_inner();
    jlog!(Debug, "Globee IPN received", { "data": &data });
    let order_id = match data.custom_payment_id {
        Some(ref id) => Some(id.parse::<Uuid>()?),
        None => None,
    };
    // At this point, just log the action so that we can retry it locally.
    // The order associated with this payment may expire before the IPN is received, but we must
    // still record that it happened. If the order has already expired or paid, the payment will be
    // recorded, so that it can be used as store credit in future.
    let mut action = DomainAction::create(
        None,
        DomainActionTypes::PaymentProviderIPN,
        None,
        json!(data),
        Some(Tables::Orders),
        order_id,
    );
    action.expires_at = action.scheduled_at.into_builder().add_days(30).finish();
    action.max_attempt_count = 5;
    action.commit(conn.get())?;

    Ok(HttpResponse::Ok().finish())
}
