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

type Result<T> = std::result::Result<T, OperationError>;

pub trait ISA {
    type Term: Clone;
    fn print(&self) -> Result<()>;
    fn push(&mut self, n: isize) -> Result<()>;
    fn swap(&mut self) -> Result<()>;
    fn pop(&mut self) -> Result<()>;
    fn clear(&mut self) -> Result<()>;
    fn symbol(&mut self) -> Result<()>;
    fn forall(&mut self) -> Result<()>;
    fn apply(&mut self) -> Result<()>;
    fn express(&mut self) -> Result<()>;
    fn assume(&mut self) -> Result<()>;
    fn abs(&mut self) -> Result<()>; // abstract is a keyword in rust
    fn trust(&mut self) -> Result<()>;
    fn trust_all(&mut self) -> Result<()>;
    fn export(&mut self) -> Result<(Self::Term, bool)>;
    fn concept(&mut self) -> Result<(Self::Term, bool)>;
    fn refer(&mut self, term: Self::Term, truthy: bool) -> Result<()>;
    fn unbind(&mut self) -> Result<()>;
}
