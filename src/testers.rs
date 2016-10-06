use errors::{TeraResult, TeraError};
use serde_json::value::{Value};
use context::ValueNumber;



pub type TesterFn = fn(
    value: Option<Value>,
    params: Vec<Value>
) -> TeraResult<bool>;


// Some helper functions to remove boilerplate with tester error handling
fn number_args_allowed(name: &str, max: usize, args_len: usize) -> TeraResult<()> {
    if args_len > max {
        return Err(TeraError::TestError(
            name.to_string(),
            format!("{} should not be called with parameters", name)
        ))
    }

    Ok(())
}

// Called to return an error when unwrapping the value and realising there's nothing
fn value_is_undefined(name: &str) -> TeraResult<bool> {
    Err(TeraError::TestError(
        name.to_string(),
        format!("{} was called on an undefined expression", name)
    ))
}

/// Returns true if `value` is defined. Otherwise, returns false.
pub fn defined(value: Option<Value>, params: Vec<Value>) -> TeraResult<bool> {
    try!(number_args_allowed("defined", 0, params.len()));

    Ok(value.is_some())
}

/// Returns true if `value` is undefined. Otherwise, returns false.
pub fn undefined(value: Option<Value>, params: Vec<Value>) -> TeraResult<bool> {
    try!(number_args_allowed("undefined", 0, params.len()));

    Ok(value.is_none())
}

/// Returns true if `value` is a string. Otherwise, returns false.
pub fn string(value: Option<Value>, params: Vec<Value>) -> TeraResult<bool> {
    try!(number_args_allowed("string", 0, params.len()));

    match value {
        Some(v) => match v {
            Value::String(_) => Ok(true),
            _ => Ok(false),
        },
        None => value_is_undefined("string")
    }
}

/// Returns true if `value` is a number. Otherwise, returns false.
pub fn number(value: Option<Value>, params: Vec<Value>) -> TeraResult<bool> {
    try!(number_args_allowed("number", 0, params.len()));

    match value {
        Some(v) => match v {
            Value::I64(_) | Value::F64(_) | Value::U64(_) => Ok(true),
            _ => Ok(false),
        },
        None => value_is_undefined("number")
    }
}

/// Returns true if `value` is an odd number. Otherwise, returns false.
pub fn odd(value: Option<Value>, params: Vec<Value>) -> TeraResult<bool> {
    try!(number_args_allowed("odd", 0, params.len()));

    match value {
        Some(v) => match v.to_number() {
            Ok(f) => Ok(f % 2.0 != 0.0),
            Err(_) => Err(TeraError::TestError(
                "odd".to_string(),
                "odd can only be called on numbers".to_string()
            ))
        },
        None => value_is_undefined("odd")
    }
}


/// Returns true if `value` is an even number. Otherwise, returns false.
pub fn even(value: Option<Value>, params: Vec<Value>) -> TeraResult<bool> {
    try!(number_args_allowed("even", 0, params.len()));

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
        None => value_is_undefined("even")
    }
}

#[cfg(test)]
mod tests {
    use super::{defined};

    use serde_json::value::{to_value};

    #[test]
    fn test_number_args_ok() {
        assert!(defined(None, vec![]).is_ok())
    }

    #[test]
    fn test_too_many_args() {
        assert!(defined(None, vec![to_value(1)]).is_err())
    }
}
