use crate::errors::*;
use crate::jwt::errors::{Error as JwtError, ErrorKind as JwtErrorKind};
use crate::payments::PaymentProcessorError;
use actix_web::{http::StatusCode, HttpResponse};
use branch_rs::BranchError;
use customer_io::CustomerIoError;
use db::utils::errors::ErrorCode::ValidationError;
use db::utils::errors::*;
use diesel::result::Error as DieselError;
use facebook::prelude::FacebookError;
use globee::GlobeeError;
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
    fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }
    fn to_response(&self) -> HttpResponse;
}

fn internal_error(message: &str) -> HttpResponse {
    status_code_and_message(StatusCode::INTERNAL_SERVER_ERROR, message)
}

fn unauthorized(message: &str) -> HttpResponse {
    status_code_and_message(StatusCode::UNAUTHORIZED, message)
}

fn not_found() -> HttpResponse {
    status_code_and_message(StatusCode::NOT_FOUND, "Not found")
}

fn status_code_and_message(code: StatusCode, message: &str) -> HttpResponse {
    HttpResponse::build(code).json(json!({"error": message.to_string()}))
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
    fn status_code(&self) -> StatusCode {
        StatusCode::NOT_FOUND
    }
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
    fn status_code(&self) -> StatusCode {
        StatusCode::UNAUTHORIZED
    }
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
    fn status_code(&self) -> StatusCode {
        match self.validation_response {
            Some(_) => StatusCode::UNPROCESSABLE_ENTITY,
            None => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
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

            HttpResponse::UnprocessableEntity().json(json!({"error": "Validation error", "fields": fields}))
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
    fn status_code(&self) -> StatusCode {
        match self.error_type {
            ApplicationErrorType::Internal => StatusCode::INTERNAL_SERVER_ERROR,
            ApplicationErrorType::Unprocessable => StatusCode::UNPROCESSABLE_ENTITY,
            ApplicationErrorType::BadRequest => StatusCode::BAD_REQUEST,
            ApplicationErrorType::ServerConfigError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
    fn to_response(&self) -> HttpResponse {
        warn!("Application error: {}", self);

        let message = match self.error_type {
            ApplicationErrorType::Internal => "Internal error",
            _ => &self.reason,
        };
        status_code_and_message(self.status_code(), message)
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
    fn status_code(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }
    fn to_response(&self) -> HttpResponse {
        status_code_and_message(StatusCode::BAD_REQUEST, "Invalid input")
    }
}

impl ConvertToWebError for url::ParseError {
    fn status_code(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }
    fn to_response(&self) -> HttpResponse {
        status_code_and_message(StatusCode::BAD_REQUEST, "Invalid URL")
    }
}

impl ConvertToWebError for AuthError {
    fn status_code(&self) -> StatusCode {
        match self.error_type {
            AuthErrorType::Forbidden => StatusCode::FORBIDDEN,
            AuthErrorType::Unauthorized => StatusCode::UNAUTHORIZED,
        }
    }
    fn to_response(&self) -> HttpResponse {
        warn!("AuthError error: {}", self.reason);

        status_code_and_message(self.status_code(), &self.reason)
    }
}

impl ConvertToWebError for DatabaseError {
    fn status_code(&self) -> StatusCode {
        match self.code {
            1000 | 1100 => StatusCode::BAD_REQUEST,
            2000 => StatusCode::NOT_FOUND,
            3400 => StatusCode::CONFLICT,
            7000 | 7200 => StatusCode::UNPROCESSABLE_ENTITY,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
    fn to_response(&self) -> HttpResponse {
        let message = match self.code {
            1000 => "Invalid input",
            1100 => "Missing input",
            2000 => "No results",
            3000 => "Query error",
            3100 => "Could not insert record",
            3200 => "Could not update record",
            3300 => "Could not delete record",
            3400 => self
                .cause
                .as_ref()
                .map(|s| s.as_str())
                .unwrap_or("Duplicate record exists"),
            4000 => "Connection error",
            7000 => self.cause.as_ref().map(|s| s.as_str()).unwrap_or("Unknown Cause"),
            7200 => match &self.error_code {
                ValidationError { errors } => {
                    return HttpResponse::UnprocessableEntity()
                        .json(json!({"error": "Validation error".to_string(), "fields": errors}))
                }
                _ => "Validation error",
            },
            5000 | 7300 => "Internal error",
            _ => "Unknown error",
        };
        status_code_and_message(self.status_code(), message)
    }
}
