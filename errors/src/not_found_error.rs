use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub struct NotFoundError {}

impl fmt::Display for NotFoundError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "Not found")
    }
}
impl Error for NotFoundError {}
