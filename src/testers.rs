use errors::Result;
use serde_json::value::{Value};
use context::ValueNumber;



pub type TesterFn = fn(Option<Value>, Vec<Value>) -> Result<bool>;


// Some helper functions to remove boilerplate with tester error handling
fn number_args_allowed(tester_name: &str, max: usize, args_len: usize) -> Result<()> {
    if max == 0 && args_len > max {
        bail!(
            "Tester `{}` was called with some args but this test doesn't take args",
            tester_name
        );
    }

    if args_len > max {
        bail!(
            "Tester `{}` was called with {} args, the max number is {}",
            tester_name, args_len, max
        );
    }

    Ok(())
}

// Called to check if the Value is defined and return an Err if not
fn value_defined(tester_name: &str, value: &Option<Value>) -> Result<()> {
    if value.is_none() {
        bail!("Tester `{}` was called on an undefined variable", tester_name);
    }

    Ok(())
}

/// Returns true if `value` is defined. Otherwise, returns false.
pub fn defined(value: Option<Value>, params: Vec<Value>) -> Result<bool> {
    number_args_allowed("defined", 0, params.len())?;

    Ok(value.is_some())
}

/// Returns true if `value` is undefined. Otherwise, returns false.
pub fn undefined(value: Option<Value>, params: Vec<Value>) -> Result<bool> {
    number_args_allowed("undefined", 0, params.len())?;

    Ok(value.is_none())
}

/// Returns true if `value` is a string. Otherwise, returns false.
pub fn string(value: Option<Value>, params: Vec<Value>) -> Result<bool> {
    number_args_allowed("string", 0, params.len())?;
    value_defined("string", &value)?;

    match value {
        Some(Value::String(_)) => Ok(true),
        _ => Ok(false)
    }
}

/// Returns true if `value` is a number. Otherwise, returns false.
pub fn number(value: Option<Value>, params: Vec<Value>) -> Result<bool> {
    number_args_allowed("number", 0, params.len())?;
    value_defined("number", &value)?;

    match value {
        Some(Value::I64(_)) | Some(Value::F64(_)) | Some(Value::U64(_)) => Ok(true),
        _ => Ok(false)
    }
}

/// Returns true if `value` is an odd number. Otherwise, returns false.
pub fn odd(value: Option<Value>, params: Vec<Value>) -> Result<bool> {
    number_args_allowed("odd", 0, params.len())?;
    value_defined("odd", &value)?;

    match value.and_then(|v| v.to_number().ok()) {
        Some(f) => Ok(f % 2.0 != 0.0),
        _ => bail!("Tester `odd` was called on a variable that isn't a number")
    }
}


/// Returns true if `value` is an even number. Otherwise, returns false.
pub fn even(value: Option<Value>, params: Vec<Value>) -> Result<bool> {
    number_args_allowed("even", 0, params.len())?;
    value_defined("even", &value)?;

    let is_odd = odd(value, params)?;
    Ok(!is_odd)
}


/// Returns true if `value` is divisible by the first param. Otherwise, returns false.
pub fn divisible_by(value: Option<Value>, params: Vec<Value>) -> Result<bool> {
    number_args_allowed("divisibleby", 1, params.len())?;
    value_defined("divisibleby", &value)?;

    match value.and_then(|v| v.to_number().ok()) {
        Some(val) => match params.first().and_then(|v| v.to_number().ok()) {
            Some(p) => Ok(val % p == 0.0),
            None => bail!("Tester `divisibleby` was called with a parameter that isn't a number")
        },
        None => bail!("Tester `divisibleby` was called on a variable that isn't a number")
    }
}


/// Returns true if `value` can be iterated over in Tera (ie is an array/tuple).
/// Otherwise, returns false.
pub fn iterable(value: Option<Value>, params: Vec<Value>) -> Result<bool> {
    number_args_allowed("iterable", 0, params.len())?;
    value_defined("iterable", &value)?;

    Ok(value.unwrap().is_array())
}


#[cfg(test)]
mod tests {
    use super::{defined, string, divisible_by, iterable};

    use serde_json::value::{to_value};

    #[test]
    fn test_number_args_ok() {
        assert!(defined(None, vec![]).is_ok())
    }

    #[test]
    fn test_too_many_args() {
        assert!(defined(None, vec![to_value(1)]).is_err())
    }

    #[test]
    fn test_value_defined() {
        assert!(string(None, vec![]).is_err())
    }

    #[test]
    fn test_divisible_by() {
        let tests = vec![
            (1.0, 2.0, false),
            (4.0, 2.0, true),
            (4.0, 2.1, false),
            (10.0, 2.0, true),
            (10.0, 0.0, false),
        ];

        for (val, divisor, expected) in tests {
            assert_eq!(
                divisible_by(Some(to_value(val)), vec![to_value(divisor)]).unwrap(),
                expected
            );
        }
    }

    #[test]
    fn test_iterable() {
        assert_eq!(iterable(Some(to_value(vec!["1"])), vec![]).unwrap(), true);
        assert_eq!(iterable(Some(to_value(1)), vec![]).unwrap(), false);
        assert_eq!(iterable(Some(to_value("hello")), vec![]).unwrap(), false);
    }
}
