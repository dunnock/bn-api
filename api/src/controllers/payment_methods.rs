use actix_web::HttpResponse;
use crate::auth::user::User;
use bigneon_db::models::ForDisplay;
use crate::db::Connection;
use crate::errors::*;

pub fn index((connection, auth_user): (Connection, User)) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let payment_methods = &auth_user.user.payment_methods(connection).for_display()?;
    Ok(HttpResponse::Ok().json(payment_methods))
}
