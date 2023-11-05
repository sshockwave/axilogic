use std::{fmt, error::Error};

#[derive(Debug)]
pub struct OperationError {
    details: String
}

impl OperationError {
    pub fn new(msg: &str) -> OperationError {
        OperationError{details: msg.to_string()}
    }
}

impl fmt::Display for OperationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "OperationError: {}", self.details)
    }
}

impl Error for OperationError {
    fn description(&self) -> &str {
        &self.details
    }
}

pub type Result<T> = std::result::Result<T, OperationError>;
