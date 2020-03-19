use crate::auth::user::User;
use crate::database::Connection;
use crate::errors::*;
use actix_web::HttpResponse;
use db::models::ForDisplay;

pub async fn index((connection, auth_user): (Connection, User)) -> Result<HttpResponse, ApiError> {
    let connection = connection.get();
    let payment_methods = &auth_user.user.payment_methods(connection).for_display()?;
    Ok(HttpResponse::Ok().json(payment_methods))
}
