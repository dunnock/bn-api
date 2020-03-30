use crate::errors::AuthError;
use crate::errors::*;
use crate::jwt::errors::Error as JwtError;
use crate::payments::PaymentProcessorError;
use actix_web::http::{header::ToStrError, StatusCode};
use actix_web::{error::ResponseError, HttpResponse};
use branch_rs::BranchError;
use chrono;
use customer_io::CustomerIoError;
use db::utils::errors::*;
use diesel::result::Error as DieselError;
use facebook::prelude::FacebookError;
use globee::GlobeeError;
use redis::RedisError;
use reqwest;
use serde_json::Error as SerdeError;
use sharetribe_flex::ShareTribeError;
use std::error::Error;
use std::fmt;
use tari_client::TariError;
use twilio::TwilioError;
use url;
use uuid::ParseError as UuidParseError;

#[derive(Debug)]
pub struct ApiError(Box<dyn ConvertToWebError + Send + Sync>);

macro_rules! error_conversion {
    ($e: ty) => {
        impl From<$e> for ApiError {
            fn from(e: $e) -> Self {
                ApiError(Box::new(e))
            }
        }
    };
}

error_conversion!(ApplicationError);
error_conversion!(AuthError);
error_conversion!(CustomerIoError);
error_conversion!(DatabaseError);
error_conversion!(r2d2::Error);
error_conversion!(DieselError);
error_conversion!(EnumParseError);
error_conversion!(JwtError);
error_conversion!(NotFoundError);
error_conversion!(ParseError);
error_conversion!(PaymentProcessorError);
error_conversion!(RedisError);
error_conversion!(SerdeError);
error_conversion!(TariError);
error_conversion!(UuidParseError);
error_conversion!(GlobeeError);
error_conversion!(BranchError);
error_conversion!(FacebookError);
error_conversion!(chrono::ParseError);
error_conversion!(std::io::Error);
error_conversion!(sitemap::Error);
error_conversion!(reqwest::Error);
error_conversion!(url::ParseError);
error_conversion!(ToStrError);
error_conversion!(ShareTribeError);

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&*self.0.to_string())
    }
}

impl Error for ApiError {
    #[allow(deprecated)]
    fn description(&self) -> &str {
        self.0.description()
    }
}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        self.0.status_code()
    }
    fn error_response(&self) -> HttpResponse {
        self.0.to_response()
    }
}

impl ApiError {
    pub fn new(inner: Box<dyn ConvertToWebError + Send + Sync>) -> ApiError {
        ApiError(inner)
    }

    pub fn into_inner(&self) -> &dyn ConvertToWebError {
        self.0.as_ref()
    }
}

impl ConvertToWebError for sitemap::Error {
    fn to_response(&self) -> HttpResponse {
        error!("Sitemap generator error: {}", self);
        HttpResponse::InternalServerError().json(json!({"error": self.to_string()}))
    }
}

impl ConvertToWebError for std::io::Error {
    fn to_response(&self) -> HttpResponse {
        error!("IO Error: {}", self);
        HttpResponse::InternalServerError().json(json!({"error": self.to_string()}))
    }
}

impl From<TwilioError> for ApiError {
    fn from(e: TwilioError) -> Self {
        ApiError::new(Box::new(e))
    }
}

impl ConvertToWebError for TwilioError {
    fn to_response(&self) -> HttpResponse {
        error!("Twilio error: {}", self);
        HttpResponse::InternalServerError().json(json!({"error": self.to_string()}))
    }
}
