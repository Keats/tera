use std::error::Error;
use std::fmt;


/// Library generic result type.
pub type TeraResult<T> = Result<T, TeraError>;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum TeraErrorType {
    /// Template doesn't exist
    TemplateNotFound,
}

/// Our actual error
#[derive(Debug)]
pub struct TeraError {
    /// The error message
    pub error: String,
    /// The error type
    pub error_type: TeraErrorType
}

impl Error for TeraError {
    fn description(&self) -> &str {
        &*self.error
    }

    fn cause(&self) -> Option<&Error> {
        match self.error_type {
            _ => None,
        }
    }
}

impl fmt::Display for TeraError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", self.error)
    }
}


pub fn template_not_found(name: &str) -> TeraError {
    TeraError {
        error: format!("Template {} not found", name),
        error_type: TeraErrorType::TemplateNotFound
    }
}
