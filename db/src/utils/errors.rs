use backtrace::Backtrace;
use diesel::result::ConnectionError;
use diesel::result::DatabaseErrorKind;
use diesel::result::Error as DieselError;
use diesel::result::QueryResult;
use log::Level;
use regex;
use serde::ser::{Serialize, SerializeStruct, Serializer};
use serde_json::Error as SerdeError;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::io::Error as IoError;
use tari_client::TariError;
use url;
use validator::{ValidationError, ValidationErrors};
use validators::create_validation_error;

#[derive(Clone, Debug, PartialEq)]
pub enum ErrorCode {
    InvalidInput,
    MissingInput,
    NoResults,
    QueryError,
    InsertError,
    UpdateError,
    DeleteError,
    DuplicateKeyError,
    ConnectionError,
    InternalError,
    AccessError,
    BusinessProcessError,
    AlreadyScheduledError,
    ConcurrencyError,
    ValidationError {
        errors: HashMap<&'static str, Vec<ValidationError>>,
    },
    ForeignKeyError,
    ParseError,
    Unknown,
    MultipleResultsWhenOneExpected,
}

pub fn get_error_message(code: &ErrorCode) -> (i32, String) {
    use self::ErrorCode::*;
    // In general, these errors try to match the HTTP status codes
    match code {
        // Input errors - 1000 range
        InvalidInput => (1000, "Invalid input".to_string()),
        MissingInput => (1100, "Missing input".to_string()),
        // No results - 2000 range. Query was successful, but the wrong amount of rows was returned
        NoResults => (2000, "No results".to_string()),
        MultipleResultsWhenOneExpected => (2100, "Multiple results when one was expected".to_string()),
        // Query errors - 3000 range. Something went wrong during the query
        QueryError => (3000, "Query Error".to_string()),
        InsertError => (3100, "Could not insert record".to_string()),
        UpdateError => (3200, "Could not update record".to_string()),
        DeleteError => (3300, "Could not delete record".to_string()),
        // TODO - This should probably move to the 2000 range
        DuplicateKeyError => (3400, "Duplicate key error".to_string()),
        ConnectionError => (4000, "Connection error".to_string()),
        // Internal server error - 5000, similar to the HTTP 500 errors
        InternalError => (5000, "Internal error".to_string()),
        // TODO - This should probably move to the 4000 range
        AccessError => (6000, "Access error".to_string()),
        // Logical/Business errors - 7000 range. These represent errors
        // that arise from an invalid setup in the database
        BusinessProcessError => (7000, "Business Process error".to_string()),
        ConcurrencyError => (7100, "Concurrency error".to_string()),
        ValidationError { errors: _ } => (7200, "Validation failed:".to_string()),
        ForeignKeyError => (
            7300,
            "Could not delete record because there are other entities referencing it".to_string(),
        ),
        ParseError => (7400, "Parse failed:".to_string()),
        AlreadyScheduledError => (7500, "Already Scheduled error".to_string()),
        // Try not to use this error
        Unknown => (10, "Unknown database error".to_string()),
    }
}

#[derive(Debug, PartialEq)]
pub struct EnumParseError {
    pub message: String,
    pub value: String,
    pub enum_type: String,
}

impl fmt::Display for EnumParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}\nType: {}\nValue: {}", self.message, self.enum_type, self.value)?;

        Ok(())
    }
}

impl Error for EnumParseError {
    fn description(&self) -> &str {
        &self.message
    }
}

#[derive(Debug, PartialEq)]
pub struct ParseError {
    pub message: String,
    pub input: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}\nInput: {}", self.message, self.input)?;

        Ok(())
    }
}

impl Error for ParseError {
    fn description(&self) -> &str {
        &self.message
    }
}

use jsonwebtoken;
impl From<jsonwebtoken::errors::Error> for DatabaseError {
    fn from(s: jsonwebtoken::errors::Error) -> DatabaseError {
        DatabaseError::new(
            ErrorCode::InternalError,
            Some(format!("JSON Web Token error: {}", s.description())),
        )
    }
}

#[derive(Debug, PartialEq)]
pub struct DatabaseError {
    pub code: i32,
    pub message: String,
    pub cause: Option<String>,
    pub error_code: ErrorCode,
}

impl fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)?;
        if let Some(ref cause) = self.cause {
            write!(f, "\nCaused by: {}", cause)?;
        }
        Ok(())
    }
}

impl Error for DatabaseError {
    fn description(&self) -> &str {
        &self.message
    }
}

impl Serialize for DatabaseError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // 3 is the number of fields in the struct.
        let mut state = serializer.serialize_struct("DatabaseError", 3)?;
        state.serialize_field("code", &self.code)?;
        state.serialize_field("message", &self.message)?;
        state.serialize_field("cause", &self.cause)?;
        state.end()
    }
}

impl DatabaseError {
    pub fn new(error_code: ErrorCode, cause: Option<String>) -> DatabaseError {
        let (code, message) = get_error_message(&error_code);

        DatabaseError {
            code,
            message,
            cause,
            error_code,
        }
    }

    /// Wraps the error from a Result into a DatabaseError
    pub fn wrap<T>(error_code: ErrorCode, message: &str, res: Result<T, DieselError>) -> Result<T, DatabaseError> {
        match res {
            Ok(val) => Ok(val),
            Err(e) => match e {
                DieselError::NotFound => Err(DatabaseError::new(
                    ErrorCode::NoResults,
                    Some(format!("{}, {}", message, e.to_string())),
                )),
                DieselError::DatabaseError(kind, _) => {
                    let current_backtrace = Backtrace::new();

                    jlog!(
                        Level::Debug,
                        &format!("PG error {}", message),
                        {
                            "error": e.to_string(),
                            "backtrace": format!("{:?}",current_backtrace)
                    });

                    match kind {
                        DatabaseErrorKind::UniqueViolation => {
                            Err(DatabaseError::new(ErrorCode::DuplicateKeyError, None))
                        }
                        DatabaseErrorKind::ForeignKeyViolation => Err(DatabaseError::new(
                            ErrorCode::ForeignKeyError,
                            Some(format!("{} {}", message, e.to_string())),
                        )),
                        _ => Err(DatabaseError::new(
                            error_code,
                            Some(format!("{}, {}", message, e.to_string())),
                        )),
                    }
                }
                _ => {
                    let current_backtrace = Backtrace::new();
                    jlog!(
                        Level::Debug,
                        &format!("PG error {}", message),
                        {
                            "error": e.to_string(),
                            "backtrace": format!("{:?}",current_backtrace)
                    });

                    Err(DatabaseError::new(
                        error_code,
                        Some(format!("{}, {}", message, e.to_string())),
                    ))
                }
            },
        }
    }

    pub fn business_process_error<T>(message: &str) -> Result<T, DatabaseError> {
        Err(DatabaseError::new(
            ErrorCode::BusinessProcessError,
            Some(message.to_string()),
        ))
    }

    pub fn already_scheduled_error<T>(message: &str) -> Result<T, DatabaseError> {
        Err(DatabaseError::new(
            ErrorCode::AlreadyScheduledError,
            Some(message.to_string()),
        ))
    }

    pub fn conflict_error<T>(message: &str) -> Result<T, DatabaseError> {
        Err(DatabaseError::new(
            ErrorCode::DuplicateKeyError,
            Some(message.to_string()),
        ))
    }

    pub fn validation_error<T>(field: &'static str, message: &'static str) -> Result<T, DatabaseError> {
        let mut v = ValidationErrors::new();
        v.add(field, create_validation_error(message, message));
        Err(DatabaseError::new(
            ErrorCode::ValidationError {
                errors: v.field_errors(),
            },
            None,
        ))
    }

    pub fn concurrency_error<T>(message: &str) -> Result<T, DatabaseError> {
        Err(DatabaseError::new(
            ErrorCode::ConcurrencyError,
            Some(message.to_string()),
        ))
    }

    pub fn no_results<T>(message: &str) -> Result<T, DatabaseError> {
        Err(DatabaseError::new(ErrorCode::NoResults, Some(message.to_string())))
    }
}

impl From<ConnectionError> for DatabaseError {
    fn from(e: ConnectionError) -> Self {
        DatabaseError::new(ErrorCode::ConnectionError, Some(e.to_string()))
    }
}

impl From<diesel::result::Error> for DatabaseError {
    fn from(e: diesel::result::Error) -> Self {
        DatabaseError::new(ErrorCode::InternalError, Some(e.to_string()))
    }
}

impl From<EnumParseError> for DatabaseError {
    fn from(e: EnumParseError) -> Self {
        DatabaseError::new(ErrorCode::ParseError, Some(e.to_string()))
    }
}

impl From<ParseError> for DatabaseError {
    fn from(e: ParseError) -> Self {
        DatabaseError::new(ErrorCode::ParseError, Some(e.to_string()))
    }
}

impl From<url::ParseError> for DatabaseError {
    fn from(e: url::ParseError) -> Self {
        DatabaseError::new(ErrorCode::ParseError, Some(e.to_string()))
    }
}

impl From<SerdeError> for DatabaseError {
    fn from(e: SerdeError) -> Self {
        DatabaseError::new(ErrorCode::InternalError, Some(e.to_string()))
    }
}

impl From<TariError> for DatabaseError {
    fn from(e: TariError) -> Self {
        DatabaseError::new(ErrorCode::InternalError, Some(e.to_string()))
    }
}

impl From<IoError> for DatabaseError {
    fn from(e: IoError) -> Self {
        DatabaseError::new(ErrorCode::InternalError, Some(e.to_string()))
    }
}

impl From<regex::Error> for DatabaseError {
    fn from(e: regex::Error) -> Self {
        DatabaseError::new(ErrorCode::InternalError, Some(e.to_string()))
    }
}

impl From<ring::error::Unspecified> for DatabaseError {
    fn from(_e: ring::error::Unspecified) -> Self {
        DatabaseError::new(ErrorCode::InternalError, Some("Encryption error".to_string()))
    }
}

impl From<ValidationErrors> for DatabaseError {
    fn from(e: ValidationErrors) -> Self {
        let message = e.to_string();
        DatabaseError::new(
            ErrorCode::ValidationError {
                errors: e.field_errors(),
            },
            Some(message),
        )
    }
}

pub trait ConvertToDatabaseError<U> {
    fn to_db_error(self, code: ErrorCode, message: &'static str) -> Result<U, DatabaseError>;
}

impl<U> ConvertToDatabaseError<U> for QueryResult<U> {
    fn to_db_error(self, code: ErrorCode, message: &'static str) -> Result<U, DatabaseError> {
        DatabaseError::wrap(code, message, self)
    }
}

pub trait OptionalToDatabaseError<U> {
    fn error_if_none(self, message: &str) -> Result<U, DatabaseError>;
}

impl<U> OptionalToDatabaseError<U> for Result<Option<U>, DatabaseError> {
    fn error_if_none(self, message: &str) -> Result<U, DatabaseError> {
        match self {
            Ok(i) => match i {
                Some(j) => Ok(j),
                None => Err(DatabaseError::new(
                    ErrorCode::NoResults,
                    Some(format!("No results returned when results were expected:{}", message)),
                )),
            },
            Err(e) => Err(e),
        }
    }
}

pub trait Optional<U> {
    fn optional(self) -> Result<Option<U>, DatabaseError>;
}

impl<U> Optional<U> for Result<U, DatabaseError> {
    fn optional(self) -> Result<Option<U>, DatabaseError> {
        match self {
            Ok(u) => Ok(Some(u)),
            Err(e) => match e.code {
                2000 => Ok(None),
                _ => Err(e),
            },
        }
    }
}

pub trait SingleResult<T> {
    fn expect_single(self) -> Result<T, DatabaseError>;
}

impl<T> SingleResult<T> for Result<Vec<T>, DatabaseError> {
    fn expect_single(self) -> Result<T, DatabaseError> {
        match self {
            Err(e) => Err(e),
            Ok(mut t) => match t.len() {
                0 => DatabaseError::no_results("No results"),
                1 => Ok(t.remove(0)),
                _ => Err(DatabaseError::new(ErrorCode::MultipleResultsWhenOneExpected, None)),
            },
        }
    }
}

#[test]
fn error_with_unknown_code() {
    let err = DatabaseError::new(ErrorCode::Unknown, None);
    assert_eq!(err.description(), err.message);
    assert_eq!(err.code, 10);
    assert!(err.cause.is_none());
    assert_eq!(format!("{}", err), "[10] Unknown database error");
}

#[test]
fn error_with_known_code() {
    let err = DatabaseError::new(ErrorCode::InvalidInput, None);
    assert_eq!(err.description(), "Invalid input");
    assert_eq!(err.code, 1000);
    assert!(err.cause.is_none());
    assert_eq!(format!("{}", err), "[1000] Invalid input");
}

#[test]
fn unknown_error_with_cause() {
    let cause = DatabaseError::new(ErrorCode::Unknown, None);
    let err = DatabaseError::new(ErrorCode::InvalidInput, Some(cause.description().to_string()));
    assert_eq!(err.description(), "Invalid input");
    assert_eq!(err.code, 1000);
    assert!(err.cause.is_some());
    assert_eq!(
        format!("{}", err),
        "\
[1000] Invalid input
Caused by: Unknown database error"
    );
}

#[test]
fn nested_causes() {
    let cause1 = DatabaseError::new(ErrorCode::Unknown, None);
    let cause2 = DatabaseError::new(ErrorCode::NoResults, Some(format!("{}", cause1)));
    let err = DatabaseError::new(ErrorCode::InvalidInput, Some(format!("{}", cause2)));
    assert_eq!(err.code, 1000);
    assert!(err.cause.is_some());
    assert_eq!(
        format!("{}", err),
        "\
[1000] Invalid input
Caused by: [2000] No results
Caused by: [10] Unknown database error"
    );
}
