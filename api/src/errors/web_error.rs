use actix_web::{http::StatusCode, HttpResponse};
use bigneon_db::utils::errors::ErrorCode::ValidationError;
use bigneon_db::utils::errors::*;
use branch_rs::BranchError;
use customer_io::CustomerIoError;
use diesel::result::Error as DieselError;
use crate::errors::*;
use facebook::prelude::FacebookError;
use globee::GlobeeError;
use crate::jwt::errors::{Error as JwtError, ErrorKind as JwtErrorKind};
use crate::payments::PaymentProcessorError;
use r2d2;
use redis::RedisError;
use reqwest::header::ToStrError as ReqwestToStrError;
use reqwest::Error as ReqwestError;
use serde_json::Error as SerdeError;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::Debug;
use std::string::ToString;
use stripe::StripeError;
use tari_client::TariError;
use uuid::ParseError as UuidParseError;

pub trait ConvertToWebError: Debug + Error + ToString {
    fn to_response(&self) -> HttpResponse;
}

fn internal_error(message: &str) -> HttpResponse {
    status_code_and_message(StatusCode::INTERNAL_SERVER_ERROR, message)
}

fn unauthorized(message: &str) -> HttpResponse {
    status_code_and_message(StatusCode::UNAUTHORIZED, message)
}

fn forbidden(message: &str) -> HttpResponse {
    status_code_and_message(StatusCode::FORBIDDEN, message)
}

fn unprocessable(message: &str) -> HttpResponse {
    status_code_and_message(StatusCode::UNPROCESSABLE_ENTITY, message)
}

fn not_found() -> HttpResponse {
    status_code_and_message(StatusCode::NOT_FOUND, "Not found")
}

fn status_code_and_message(code: StatusCode, message: &str) -> HttpResponse {
    HttpResponse::new(code)
        .into_builder()
        .json(json!({"error": message.to_string()}))
}

impl ConvertToWebError for dyn Error {
    fn to_response(&self) -> HttpResponse {
        error!("General error: {}", self);
        internal_error("Internal error")
    }
}

impl ConvertToWebError for CustomerIoError {
    fn to_response(&self) -> HttpResponse {
        internal_error("Internal error")
    }
}

impl ConvertToWebError for FacebookError {
    fn to_response(&self) -> HttpResponse {
        internal_error("Internal error")
    }
}

impl ConvertToWebError for DieselError {
    fn to_response(&self) -> HttpResponse {
        error!("Diesel error: {}", self);
        internal_error("Internal error")
    }
}

impl ConvertToWebError for r2d2::Error {
    fn to_response(&self) -> HttpResponse {
        error!("R2D2 error: {}", self);
        internal_error("Internal error")
    }
}
impl ConvertToWebError for GlobeeError {
    fn to_response(&self) -> HttpResponse {
        error!("Globee error: {}", self);
        internal_error("Internal error")
    }
}

impl ConvertToWebError for BranchError {
    fn to_response(&self) -> HttpResponse {
        error!("Branch error: {}", self);
        internal_error("Internal error")
    }
}

impl ConvertToWebError for NotFoundError {
    fn to_response(&self) -> HttpResponse {
        not_found()
    }
}

impl ConvertToWebError for RedisError {
    fn to_response(&self) -> HttpResponse {
        error!("Redis error: {}", self);
        internal_error("Internal error")
    }
}

impl ConvertToWebError for JwtError {
    fn to_response(&self) -> HttpResponse {
        match self.kind().clone() {
            JwtErrorKind::ExpiredSignature => info!("JWT error: {}", self),
            _ => warn!("JWT error: {}", self),
        }
        unauthorized("Invalid token")
    }
}

impl ConvertToWebError for UuidParseError {
    fn to_response(&self) -> HttpResponse {
        error!("UUID parse error: {}", self);
        internal_error("Internal error")
    }
}

impl ConvertToWebError for ReqwestError {
    fn to_response(&self) -> HttpResponse {
        error!("Reqwest error: {}", self);
        internal_error("Internal error")
    }
}

impl ConvertToWebError for ReqwestToStrError {
    fn to_response(&self) -> HttpResponse {
        error!("Reqwest error: {}", self);
        internal_error("Internal error")
    }
}

impl ConvertToWebError for PaymentProcessorError {
    fn to_response(&self) -> HttpResponse {
        if let Some(ref validation_response) = self.validation_response {
            let mut fields = HashMap::new();
            #[derive(Serialize)]
            struct R {
                code: String,
                message: String,
            }

            fields.insert(
                "token",
                vec![R {
                    code: "processing-error".into(),
                    message: format!("Unable to process payment, {}", validation_response),
                }],
            );

            HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY)
                .into_builder()
                .json(json!({"error": "Validation error", "fields": fields}))
        } else {
            error!("Payment Processor error: {}", self);
            internal_error("Internal error")
        }
    }
}

impl ConvertToWebError for EnumParseError {
    fn to_response(&self) -> HttpResponse {
        error!("Enum parse error: {}", self);
        internal_error("Internal error")
    }
}

impl ConvertToWebError for ParseError {
    fn to_response(&self) -> HttpResponse {
        error!("Parse error: {}", self);
        internal_error("Internal error")
    }
}

impl ConvertToWebError for StripeError {
    fn to_response(&self) -> HttpResponse {
        error!("Stripe error: {}", self);
        internal_error("Internal error")
    }
}

impl ConvertToWebError for ApplicationError {
    fn to_response(&self) -> HttpResponse {
        warn!("Application error: {}", self);

        match self.error_type {
            ApplicationErrorType::Internal => internal_error("Internal error"),
            ApplicationErrorType::Unprocessable => unprocessable(&self.reason),
            ApplicationErrorType::BadRequest => status_code_and_message(StatusCode::BAD_REQUEST, &self.reason),
            ApplicationErrorType::ServerConfigError => internal_error(&self.reason),
        }
    }
}

impl ConvertToWebError for SerdeError {
    fn to_response(&self) -> HttpResponse {
        error!("Serde error: {}", self);
        internal_error("Internal error")
    }
}

impl ConvertToWebError for TariError {
    fn to_response(&self) -> HttpResponse {
        error!("Tari error: {}", self);
        internal_error("There was an error during the ticket transfer process")
    }
}

impl ConvertToWebError for chrono::ParseError {
    fn to_response(&self) -> HttpResponse {
        status_code_and_message(StatusCode::BAD_REQUEST, "Invalid input")
    }
}

impl ConvertToWebError for url::ParseError {
    fn to_response(&self) -> HttpResponse {
        status_code_and_message(StatusCode::BAD_REQUEST, "Invalid URL")
    }
}

impl ConvertToWebError for AuthError {
    fn to_response(&self) -> HttpResponse {
        warn!("AuthError error: {}", self.reason);

        match self.error_type {
            AuthErrorType::Forbidden => forbidden(&self.reason),
            AuthErrorType::Unauthorized => unauthorized(&self.reason),
        }
    }
}

impl ConvertToWebError for DatabaseError {
    fn to_response(&self) -> HttpResponse {
        match self.code {
            1000 => status_code_and_message(StatusCode::BAD_REQUEST, "Invalid input"),
            1100 => status_code_and_message(StatusCode::BAD_REQUEST, "Missing input"),
            2000 => status_code_and_message(StatusCode::NOT_FOUND, "No results"),
            3000 => internal_error("Query error"),
            3100 => internal_error("Could not insert record"),
            3200 => internal_error("Could not update record"),
            3300 => internal_error("Could not delete record"),
            3400 => status_code_and_message(
                StatusCode::CONFLICT,
                &self.cause.clone().unwrap_or("Duplicate record exists".to_string()),
            ),
            4000 => internal_error("Connection error"),
            5000 => internal_error("Internal error"),
            7000 => {
                let cause = self.cause.clone().unwrap_or("Unknown Cause".to_string());
                status_code_and_message(StatusCode::UNPROCESSABLE_ENTITY, cause.as_str())
            }
            7200 => match &self.error_code {
                ValidationError { errors } => HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY)
                    .into_builder()
                    .json(json!({"error": "Validation error".to_string(), "fields": errors})),
                _ => HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY)
                    .into_builder()
                    .json(json!({"error": "Validation error".to_string()})),
            },
            7300 => internal_error("Internal error"),
            _ => internal_error("Unknown error"),
        }
    }
}
