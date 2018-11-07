use std::fmt;
use std::error::{Error as StdError};
use std::convert::Into;
use serde_json;

/// The kind of an error.
#[derive(Debug)]
pub enum ErrorKind {
    /// Generic error
    Msg(String),
    /// An error happened while serializing JSON
    Json(serde_json::Error),
}

/// The Error type
#[derive(Debug)]
pub struct Error {
    /// Kind of error
    pub kind: ErrorKind,
    cause: Option<Box<dyn StdError>>,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.kind {
            ErrorKind::Msg(ref message) => write!(f, "{}", message),
            ErrorKind::Json(ref e) => write!(f, "{}", e),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.cause.as_ref().map(|c| &**c)
    }
}

impl Error {
    /// Creates generic error
    pub fn msg(value: impl ToString) -> Self {
        Self {
            kind: ErrorKind::Msg(value.to_string()),
            cause: None,
        }
    }

    /// Creates generic error with a cause
    pub fn chain(value: impl ToString, cause: impl Into<Box<dyn StdError>>) -> Self {
        Self {
            kind: ErrorKind::Msg(value.to_string()),
            cause: Some(cause.into()),
        }
    }

    /// Creates JSON error
    pub fn json(value: serde_json::Error) -> Self {
        Self {
            kind: ErrorKind::Json(value),
            cause: None,
        }
    }
}

/// Convenient wrapper around std::Result.
pub type Result<T> = ::std::result::Result<T, Error>;
