/// Filters operating on numbers
use std::collections::HashMap;

#[cfg(feature = "builtins")]
use humansize::format_size;
use serde_json::value::{to_value, Value};

use crate::errors::{Error, Result};

/// Returns the absolute value of the argument.
pub fn abs(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    if value.as_u64().is_some() {
        Ok(value.clone())
    } else if let Some(num) = value.as_i64() {
        Ok(to_value(num.abs()).unwrap())
    } else if let Some(num) = value.as_f64() {
        Ok(to_value(num.abs()).unwrap())
    } else {
        Err(Error::msg("Filter `abs` was used on a value that isn't a number."))
    }
}

/// Returns a plural suffix if the value is not equal to ±1, or a singular
/// suffix otherwise. The plural suffix defaults to `s` and the singular suffix
/// defaults to the empty string (i.e nothing).
pub fn pluralize(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
    let num = try_get_value!("pluralize", "value", f64, value);

    let plural = match args.get("plural") {
        Some(val) => try_get_value!("pluralize", "plural", String, val),
        None => "s".to_string(),
    };

    let singular = match args.get("singular") {
        Some(val) => try_get_value!("pluralize", "singular", String, val),
        None => "".to_string(),
    };

    // English uses plural when it isn't one
    if (num.abs() - 1.).abs() > ::std::f64::EPSILON {
        Ok(to_value(plural).unwrap())
    } else {
        Ok(to_value(singular).unwrap())
    }
}

/// Returns a rounded number using the `method` arg and `precision` given.
/// `method` defaults to `common` which will round to the nearest number.
/// `ceil` and `floor` are also available as method.
/// `precision` defaults to `0`, meaning it will round to an integer
pub fn round(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
    let num = try_get_value!("round", "value", f64, value);
    let method = match args.get("method") {
        Some(val) => try_get_value!("round", "method", String, val),
        None => "common".to_string(),
    };
    let precision = match args.get("precision") {
        Some(val) => try_get_value!("round", "precision", i32, val),
        None => 0,
    };
    let multiplier = if precision == 0 { 1.0 } else { 10.0_f64.powi(precision) };

    match method.as_ref() {
        "common" => Ok(to_value((multiplier * num).round() / multiplier).unwrap()),
        "ceil" => Ok(to_value((multiplier * num).ceil() / multiplier).unwrap()),
        "floor" => Ok(to_value((multiplier * num).floor() / multiplier).unwrap()),
        _ => Err(Error::msg(format!(
            "Filter `round` received an incorrect value for arg `method`: got `{:?}`, \
             only common, ceil and floor are allowed",
            method
        ))),
    }
}

/// Returns a human-readable file size (i.e. '110 MB') from an integer
#[cfg(feature = "builtins")]
pub fn filesizeformat(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
    let num = try_get_value!("filesizeformat", "value", usize, value);
    let binary = match args.get("binary") {
        Some(binary) => try_get_value!("filesizeformat", "binary", bool, binary),
        None => false,
    };
    let format = if binary { humansize::BINARY } else { humansize::WINDOWS };
    Ok(to_value(format_size(num, format))
        .expect("json serializing should always be possible for a string"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::value::to_value;
    use std::collections::HashMap;

    #[test]
    fn test_abs_unsigend() {
        let result = abs(&to_value(1).unwrap(), &HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(1).unwrap());
    }

    #[test]
    fn test_abs_negative_integer() {
        let result = abs(&to_value(-1).unwrap(), &HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(1).unwrap());
    }

    #[test]
    fn test_abs_negative_float() {
        let result = abs(&to_value(-1.0).unwrap(), &HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(1.0).unwrap());
    }

    #[test]
    fn test_abs_non_number() {
        let result = abs(&to_value("nan").unwrap(), &HashMap::new());
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Filter `abs` was used on a value that isn't a number."
        );
    }

    #[test]
    fn test_pluralize_single() {
        let result = pluralize(&to_value(1).unwrap(), &HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("").unwrap());
    }

    #[test]
    fn test_pluralize_multiple() {
        let result = pluralize(&to_value(2).unwrap(), &HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("s").unwrap());
    }

    #[test]
    fn test_pluralize_zero() {
        let result = pluralize(&to_value(0).unwrap(), &HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("s").unwrap());
    }

    #[test]
    fn test_pluralize_multiple_custom_plural() {
        let mut args = HashMap::new();
        args.insert("plural".to_string(), to_value("es").unwrap());
        let result = pluralize(&to_value(2).unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("es").unwrap());
    }

    #[test]
    fn test_pluralize_multiple_custom_singular() {
        let mut args = HashMap::new();
        args.insert("singular".to_string(), to_value("y").unwrap());
        let result = pluralize(&to_value(1).unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("y").unwrap());
    }

    #[test]
    fn test_round_default() {
        let result = round(&to_value(2.1).unwrap(), &HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(2.0).unwrap());
    }

    #[test]
    fn test_round_default_precision() {
        let mut args = HashMap::new();
        args.insert("precision".to_string(), to_value(2).unwrap());
        let result = round(&to_value(3.15159265359).unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(3.15).unwrap());
    }

    #[test]
    fn test_round_ceil() {
        let mut args = HashMap::new();
        args.insert("method".to_string(), to_value("ceil").unwrap());
        let result = round(&to_value(2.1).unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(3.0).unwrap());
    }

    #[test]
    fn test_round_ceil_precision() {
        let mut args = HashMap::new();
        args.insert("method".to_string(), to_value("ceil").unwrap());
        args.insert("precision".to_string(), to_value(1).unwrap());
        let result = round(&to_value(2.11).unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(2.2).unwrap());
    }

    #[test]
    fn test_round_floor() {
        let mut args = HashMap::new();
        args.insert("method".to_string(), to_value("floor").unwrap());
        let result = round(&to_value(2.1).unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(2.0).unwrap());
    }

    #[test]
    fn test_round_floor_precision() {
        let mut args = HashMap::new();
        args.insert("method".to_string(), to_value("floor").unwrap());
        args.insert("precision".to_string(), to_value(1).unwrap());
        let result = round(&to_value(2.91).unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(2.9).unwrap());
    }

    #[cfg(feature = "builtins")]
    #[test]
    fn test_filesizeformat() {
        let args = HashMap::new();
        let result = filesizeformat(&to_value(123456789).unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("117.74 MB").unwrap());
    }

    #[cfg(feature = "builtins")]
    #[test]
    fn test_filesizeformat_binary() {
        let mut args = HashMap::new();
        args.insert("binary".to_string(), to_value(true).unwrap());
        let result = filesizeformat(&to_value(123456789).unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("117.74 MiB").unwrap());
    }
}
