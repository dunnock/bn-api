use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub struct PaymentProcessorError {
    pub description: String,
    pub cause: Option<Box<dyn Error>>,
    pub validation_response: Option<String>,
}

unsafe impl Send for PaymentProcessorError {}
unsafe impl Sync for PaymentProcessorError {}

impl Error for PaymentProcessorError {
    fn description(&self) -> &str {
        &self.description
    }
}

impl fmt::Display for PaymentProcessorError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match &self.cause {
            Some(c) => write!(f, "{} caused by: {}", self.description, c.description()),
            None => write!(f, "{}", self.description),
        }
    }
}
