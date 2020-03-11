use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum AuthErrorType {
    Forbidden,
    Unauthorized,
}

#[derive(Debug)]
pub struct AuthError {
    pub reason: String,
    pub error_type: AuthErrorType,
}

impl AuthError {
    pub fn new(error_type: AuthErrorType, reason: String) -> AuthError {
        AuthError { reason, error_type }
    }
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self.reason)
    }
}
impl Error for AuthError {}
