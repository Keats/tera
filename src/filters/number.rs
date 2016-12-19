/// Filters operating on numbers
use std::collections::HashMap;

use serde_json::value::{Value, to_value};
use humansize::{FileSize, file_size_opts};

use errors::Result;


/// Returns a suffix if the value is greater or equal than 2. Suffix defaults to `s`
pub fn pluralize(value: Value, args: HashMap<String, Value>) -> Result<Value> {
    let num = try_get_value!("pluralize", "value", f64, value);
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
pub fn round(value: Value, args: HashMap<String, Value>) -> Result<Value> {
    let num = try_get_value!("round", "value", f64, value);
    let method = match args.get("method") {
        Some(val) => try_get_value!("round", "method", String, val.clone()),
        None => "common".to_string(),
    };

    match method.as_ref() {
        "common" => Ok(to_value(num.round())),
        "ceil" => Ok(to_value(num.ceil())),
        "floor" => Ok(to_value(num.floor())),
        _ => bail!(
                "Filter `round` received an incorrect value for arg `method`: got `{:?}`, \
                only common, ceil and floor are allowed",
                method
            )
    }
}


/// Returns a human-readable file size (i.e. '110 MB') from an integer
pub fn filesizeformat(value: Value, _: HashMap<String, Value>) -> Result<Value> {
    let num = try_get_value!("filesizeformat", "value", i64, value);
    num
        .file_size(file_size_opts::CONVENTIONAL)
        .or_else(|_| Err(format!("Filter `filesizeformat` was called on a negative number: {}", num).into()))
        .map(to_value)
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

    #[test]
    fn test_filesizeformat() {
        let args = HashMap::new();
        let result = filesizeformat(to_value(123456789), args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("117.74 MB"));
    }
}
