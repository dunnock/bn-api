use crate::auth::TokenResponse;
use crate::communications::mailers;
use crate::db::Connection;
use crate::errors::*;
use crate::extractors::*;
use crate::helpers::application;
use crate::server::AppState;
use actix_web::{HttpResponse, web::Data};
use bigneon_db::models::concerns::users::password_resetable::*;
use bigneon_db::models::User;
use bigneon_db::utils::errors::Optional;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct CreatePasswordResetParameters {
    pub email: String,
}

#[derive(Deserialize)]
pub struct UpdatePasswordResetParameters {
    pub password_reset_token: Uuid,
    pub password: String,
}

pub async fn create(
    (state, connection, parameters): (Data<AppState>, Connection, Json<CreatePasswordResetParameters>),
) -> Result<HttpResponse, BigNeonError> {
    let request_pending_response = Ok(HttpResponse::Created().json(json!({
        "message": format!("Your request has been received; {} will receive an email shortly with a link to reset your password if it is an account on file.", parameters.email)
    })));

    let connection = connection.get();
    let email = parameters.email.trim().to_lowercase();
    let mut user = match User::find_by_email(&email, false, connection) {
        Ok(user) => user,
        Err(_) => return request_pending_response,
    };

    if !user.has_valid_password_reset_token() {
        user = user.create_password_reset_token(connection)?;
    }

    mailers::user::password_reset_email(&state.config, &user).queue(connection)?;

    request_pending_response
}

pub async fn update(
    (state, connection, parameters): (Data<AppState>, Connection, Json<UpdatePasswordResetParameters>),
) -> Result<HttpResponse, BigNeonError> {
    let user =
        User::consume_password_reset_token(&parameters.password_reset_token, &parameters.password, connection.get())
            .optional()?;

    match user {
        Some(user) => Ok(HttpResponse::Ok().json(&TokenResponse::create_from_user(
            &state.config.token_secret,
            &state.config.token_issuer,
            &state.config.jwt_expiry_time,
            &user,
        )?)),
        None => application::unprocessable("Password has already been reset."),
    }
}
