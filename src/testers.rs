use errors::{TeraResult, TeraError};
use serde_json::value::{Value};
use context::ValueNumber;

use std::collections::LinkedList;


pub type TesterFn = fn(
    value: Option<Value>,
    params: LinkedList<Value>
) -> TeraResult<bool>;


/// Returns true if `value` is defined in the given context. Otherwise, returns
/// false.
pub fn defined(value: Option<Value>, params: LinkedList<Value>) -> TeraResult<bool> {
    if params.len() != 0 {
        return Err(TeraError::TestError(
            "defined".to_string(),
            "defined should not be called with parameters".to_string()
        ))
    }

    Ok(value.is_some())
}

/// Returns true if `value` is an odd number. Otherwise, returns false.
pub fn odd(value: Option<Value>, params: LinkedList<Value>) -> TeraResult<bool> {
    if params.len() != 0 {
        return Err(TeraError::TestError(
            "odd".to_string(),
            "odd should not be called with parameters".to_string()
        ))
    }

    match value {
        Some(v) => {
          return match v.to_number() {
            Ok(f) => Ok(f % 2.0 != 0.0),
            Err(_) => Err(TeraError::TestError(
                "odd".to_string(),
                "odd can only be called on numbers".to_string()
            ))
          };
        },
        None => Err(TeraError::TestError(
            "odd".to_string(),
            "odd was called on an undefined expression".to_string()
        ))
    }
}


/// Returns true if `value` is defined in an even number. Otherwise, returns false.
pub fn even(value: Option<Value>, params: LinkedList<Value>) -> TeraResult<bool> {
    if params.len() != 0 {
        return Err(TeraError::TestError(
            "even".to_string(),
            "even should not be called with parameters".to_string()
        ))
    }

    match value {
        Some(v) => {
          return match v.to_number() {
            Ok(f) => Ok(f % 2.0 == 0.0),
            Err(_) => Err(TeraError::TestError(
                "even".to_string(),
                "even can only be called on numbers".to_string()
            ))
          };
        },
        None => Err(TeraError::TestError(
            "even".to_string(),
            "even was called on an undefined expression".to_string()
        ))
    }
}

