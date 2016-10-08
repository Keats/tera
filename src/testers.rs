use errors::{TeraResult, TeraError};
use serde_json::value::{Value};
use context::ValueNumber;



pub type TesterFn = fn(&str, Option<Value>, Vec<Value>) -> TeraResult<bool>;


// Some helper functions to remove boilerplate with tester error handling
fn number_args_allowed(arg_name: &str, tester_name: &str, max: usize, args_len: usize) -> TeraResult<()> {
    if max == 0 && args_len > max {
        return Err(TeraError::TestError(
            tester_name.to_string(),
            format!(
                "{} was called on the variable {} with some arguments \
                but this test doesn't take arguments.",
                tester_name, arg_name
            )
        ))
    }

    if args_len > max {
        return Err(TeraError::TestError(
            tester_name.to_string(),
            format!(
                "{} was called on the variable {} with {} arguments, the max number is {}. ",
                tester_name, arg_name, args_len, max
            )
        ))
    }

    Ok(())
}

// Called to return an error when unwrapping the value and realising there's nothing
fn value_is_undefined(arg_name: &str, tester_name: &str) -> TeraResult<bool> {
    Err(TeraError::TestError(
        tester_name.to_string(),
        format!("{} was called on the variable {}, which is undefined", tester_name, arg_name)
    ))
}

/// Returns true if `value` is defined. Otherwise, returns false.
pub fn defined(name: &str, value: Option<Value>, params: Vec<Value>) -> TeraResult<bool> {
    try!(number_args_allowed(name, "defined", 0, params.len()));

    Ok(value.is_some())
}

/// Returns true if `value` is undefined. Otherwise, returns false.
pub fn undefined(name: &str, value: Option<Value>, params: Vec<Value>) -> TeraResult<bool> {
    try!(number_args_allowed(name, "undefined", 0, params.len()));

    Ok(value.is_none())
}

/// Returns true if `value` is a string. Otherwise, returns false.
pub fn string(name: &str, value: Option<Value>, params: Vec<Value>) -> TeraResult<bool> {
    try!(number_args_allowed(name, "string", 0, params.len()));

    match value {
        Some(Value::String(_)) => Ok(true),
        None => value_is_undefined(name, "string"),
        _ => Ok(false)
    }
}

/// Returns true if `value` is a number. Otherwise, returns false.
pub fn number(name: &str, value: Option<Value>, params: Vec<Value>) -> TeraResult<bool> {
    try!(number_args_allowed(name, "number", 0, params.len()));

    match value {
        Some(Value::I64(_)) | Some(Value::F64(_)) | Some(Value::U64(_)) => Ok(true),
        None => value_is_undefined(name, "number"),
        _ => Ok(false)
    }
}

/// Returns true if `value` is an odd number. Otherwise, returns false.
pub fn odd(name: &str, value: Option<Value>, params: Vec<Value>) -> TeraResult<bool> {
    try!(number_args_allowed(name, "odd", 0, params.len()));

    match value {
        Some(v) => match v.to_number() {
            Ok(f) => Ok(f % 2.0 != 0.0),
            Err(_) => Err(TeraError::TestError(
                "odd".to_string(),
                "odd can only be called on numbers".to_string()
            ))
        },
        None => value_is_undefined(name, "odd")
    }
}


/// Returns true if `value` is an even number. Otherwise, returns false.
pub fn even(name: &str, value: Option<Value>, params: Vec<Value>) -> TeraResult<bool> {
    try!(number_args_allowed(name, "even", 0, params.len()));

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
        None => value_is_undefined(name, "even")
    }
}

#[cfg(test)]
mod tests {
    use super::{defined};

    use serde_json::value::{to_value};

    #[test]
    fn test_number_args_ok() {
        assert!(defined("", None, vec![]).is_ok())
    }

    #[test]
    fn test_too_many_args() {
        assert!(defined("", None, vec![to_value(1)]).is_err())
    }
}
