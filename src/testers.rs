use errors::{Result, ErrorKind};
use serde_json::value::{Value};
use context::ValueNumber;



pub type TesterFn = fn(&str, Option<Value>, Vec<Value>) -> Result<bool>;


// Some helper functions to remove boilerplate with tester error handling
fn number_args_allowed(arg_name: &str, tester_name: &str, max: usize, args_len: usize) -> Result<()> {
    if max == 0 && args_len > max {
        return Err(ErrorKind::TestError(
            tester_name.to_string(),
            format!(
                "{} was called on the variable {} with some arguments \
                but this test doesn't take arguments.",
                tester_name, arg_name
            )
        ).into())
    }

    if args_len > max {
        return Err(ErrorKind::TestError(
            tester_name.to_string(),
            format!(
                "{} was called on the variable {} with {} arguments, the max number is {}. ",
                tester_name, arg_name, args_len, max
            )
        ).into())
    }

    Ok(())
}

// Called to check if the Value is defined and return an Err if not
fn value_defined(arg_name: &str, tester_name: &str, value: &Option<Value>) -> Result<()> {
    if value.is_none() {
        return Err(ErrorKind::TestError(
            tester_name.to_string(),
            format!("{} was called on the variable {}, which is undefined", tester_name, arg_name)
        ).into());
    }

    Ok(())
}

/// Returns true if `value` is defined. Otherwise, returns false.
pub fn defined(name: &str, value: Option<Value>, params: Vec<Value>) -> Result<bool> {
    number_args_allowed(name, "defined", 0, params.len())?;

    Ok(value.is_some())
}

/// Returns true if `value` is undefined. Otherwise, returns false.
pub fn undefined(name: &str, value: Option<Value>, params: Vec<Value>) -> Result<bool> {
    number_args_allowed(name, "undefined", 0, params.len())?;

    Ok(value.is_none())
}

/// Returns true if `value` is a string. Otherwise, returns false.
pub fn string(name: &str, value: Option<Value>, params: Vec<Value>) -> Result<bool> {
    number_args_allowed(name, "string", 0, params.len())?;
    value_defined(name, "string", &value)?;

    match value {
        Some(Value::String(_)) => Ok(true),
        _ => Ok(false)
    }
}

/// Returns true if `value` is a number. Otherwise, returns false.
pub fn number(name: &str, value: Option<Value>, params: Vec<Value>) -> Result<bool> {
    number_args_allowed(name, "number", 0, params.len())?;
    value_defined(name, "number", &value)?;

    match value {
        Some(Value::I64(_)) | Some(Value::F64(_)) | Some(Value::U64(_)) => Ok(true),
        _ => Ok(false)
    }
}

/// Returns true if `value` is an odd number. Otherwise, returns false.
pub fn odd(name: &str, value: Option<Value>, params: Vec<Value>) -> Result<bool> {
    number_args_allowed(name, "odd", 0, params.len())?;
    value_defined(name, "odd", &value)?;

    match value.and_then(|v| v.to_number().ok()) {
        Some(f) => Ok(f % 2.0 != 0.0),
        _ => Err(ErrorKind::TestError(
            "odd".to_string(),
            "odd can only be called on numbers".to_string()
        ).into())
    }
}


/// Returns true if `value` is an even number. Otherwise, returns false.
pub fn even(name: &str, value: Option<Value>, params: Vec<Value>) -> Result<bool> {
    number_args_allowed(name, "even", 0, params.len())?;
    value_defined(name, "even", &value)?;

    let is_odd = odd(name, value, params)?;
    Ok(!is_odd)
}

#[cfg(test)]
mod tests {
    use super::{defined, string};

    use serde_json::value::{to_value};

    #[test]
    fn test_number_args_ok() {
        assert!(defined("", None, vec![]).is_ok())
    }

    #[test]
    fn test_too_many_args() {
        assert!(defined("", None, vec![to_value(1)]).is_err())
    }

    #[test]
    fn test_value_defined() {
        assert!(string("", None, vec![]).is_err())
    }
}
