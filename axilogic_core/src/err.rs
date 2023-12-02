use std::{error::Error, fmt};

#[derive(Debug)]
pub struct OperationError {
    details: String,
}

impl OperationError {
    pub fn new<T: Into<String>>(msg: T) -> OperationError {
        OperationError {
            details: msg.into(),
        }
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
