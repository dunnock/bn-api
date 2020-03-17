use crate::errors::BigNeonError;
use bigneon_db::prelude::*;
use diesel;
use diesel::prelude::ConnectionError;
use r2d2;
use std::error;
use std::fmt;
use std::io;

#[derive(Debug)]
pub enum DomainActionError {
    Simple(String),
    CausedBy(Box<dyn error::Error + Send>),
}

impl From<DatabaseError> for DomainActionError {
    fn from(de: DatabaseError) -> Self {
        DomainActionError::CausedBy(Box::new(de))
    }
}

impl From<EnumParseError> for DomainActionError {
    fn from(e: EnumParseError) -> Self {
        let db_error: DatabaseError = e.into();
        db_error.into()
    }
}

impl From<ConnectionError> for DomainActionError {
    fn from(e: ConnectionError) -> Self {
        let db_error: DatabaseError = e.into();
        db_error.into()
    }
}

impl From<io::Error> for DomainActionError {
    fn from(e: io::Error) -> Self {
        DomainActionError::CausedBy(Box::new(e))
    }
}
impl From<diesel::result::Error> for DomainActionError {
    fn from(e: diesel::result::Error) -> Self {
        DomainActionError::CausedBy(Box::new(e))
    }
}
impl From<r2d2::Error> for DomainActionError {
    fn from(e: r2d2::Error) -> Self {
        DomainActionError::CausedBy(Box::new(e))
    }
}

impl fmt::Display for DomainActionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DomainActionError::Simple(s) => f.write_str(s),
            DomainActionError::CausedBy(c) => f.write_str(&c.to_string()),
        }
    }
}

impl From<BigNeonError> for DomainActionError {
    fn from(source: BigNeonError) -> Self {
        DomainActionError::CausedBy(Box::new(source))
    }
}
