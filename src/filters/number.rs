/// Filters operating on numbers
use std::collections::HashMap;

use serde_json::value::{Value, to_value};

use errors::{TeraResult, TeraError};


/// Returns a suffix if the value is greater or equal than 2. Suffix defaults to `s`
pub fn pluralize(value: Value, args: HashMap<String, Value>) -> TeraResult<Value> {
    let num = try_get_value!("pluralize", "value", f32, value);
    let suffix = match args.get("suffix") {
        Some(val) => try_get_value!("pluralize", "suffix", String, val.clone()),
        None => "s".to_string(),
    };

    if num >= 2.0 {
        Ok(to_value(&suffix))
    } else {
        Ok(to_value(&""))
    }
}

/// Returns a rounded number using the `method` arg given. `method` defaults to `common` which
/// will round to the nearest number.
/// `ceil` and `floor` are also available as method.
pub fn round(value: Value, args: HashMap<String, Value>) -> TeraResult<Value> {
    let num = try_get_value!("round", "value", f32, value);
    let method = match args.get("method") {
        Some(val) => try_get_value!("round", "method", String, val.clone()),
        None => "common".to_string(),
    };

    match method.as_ref() {
        "common" => Ok(to_value(num.round())),
        "ceil" => Ok(to_value(num.ceil())),
        "floor" => Ok(to_value(num.floor())),
        _ => Err(
            TeraError::Internal(
                format!("round filter received an incorrect value for `method` argument: {}", method)
            )
        )
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use serde_json::value::to_value;
    use super::*;

    #[test]
    fn test_pluralize_single() {
        let result = pluralize(to_value(1), HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(""));
    }

    #[test]
    fn test_pluralize_multiple() {
        let result = pluralize(to_value(2), HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("s"));
    }

    #[test]
    fn test_pluralize_multiple_custom_suffix() {
        let mut args = HashMap::new();
        args.insert("suffix".to_string(), to_value("es"));
        let result = pluralize(to_value(2), args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("es"));
    }

    #[test]
    fn test_round_default() {
        let result = round(to_value(2.1), HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(2.0));
    }

    #[test]
    fn test_round_ceil() {
        let mut args = HashMap::new();
        args.insert("method".to_string(), to_value("ceil"));
        let result = round(to_value(2.1), args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(3.0));
    }

    #[test]
    fn test_round_floor() {
        let mut args = HashMap::new();
        args.insert("method".to_string(), to_value("floor"));
        let result = round(to_value(2.1), args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(2.0));
    }
}
