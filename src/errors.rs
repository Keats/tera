use std::error::Error;
use std::fmt;


/// Library generic result type.
pub type TeraResult<T> = Result<T, TeraError>;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum TeraErrorType {
    /// Template doesn't exist
    TemplateNotFound,
    /// Field not found in context
    FieldNotFound,
    /// Tried to do math on something that isn't a number
    NotANumber,
    /// Tried to iterate on a non-array field
    NotAnArray,
    /// Tried to call a function that wasn't added to Tera
    FunctionNotFound,
    /// Function errored when called
    FunctionError,
    /// Function was missing args or they had the wrong type when called
    FunctionIncorrectArgs
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
        error: format!("Template `{}` not found", name),
        error_type: TeraErrorType::TemplateNotFound
    }
}

pub fn field_not_found(key: &str) -> TeraError {
    TeraError {
        error: format!("Field `{}` not found in context", key),
        error_type: TeraErrorType::FieldNotFound
    }
}


pub fn not_a_number(key: &str) -> TeraError {
    TeraError {
        error: format!("Field `{}` was used in a math operation but is not a number", key),
        error_type: TeraErrorType::NotANumber
    }
}


pub fn not_an_array(key: &str) -> TeraError {
    TeraError {
        error: format!("Field `{}` is not an array but was used as iterator in forloop", key),
        error_type: TeraErrorType::NotAnArray
    }
}

pub fn function_not_found(name: &str) -> TeraError {
    TeraError {
        error: format!("Function `{}` not found in Tera", name),
        error_type: TeraErrorType::FunctionNotFound
    }
}
