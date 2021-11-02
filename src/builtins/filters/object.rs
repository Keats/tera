/// Filters operating on numbers
use std::collections::HashMap;

use serde_json::value::Value;

use crate::errors::{Error, Result};

/// Returns a value by a `key` argument from a given object
pub fn get(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
    let default = args.get("default");
    let key = match args.get("key") {
        Some(val) => try_get_value!("get", "key", String, val),
        None => return Err(Error::msg("The `get` filter has to have an `key` argument")),
    };

    match value.as_object() {
        Some(o) => match o.get(&key) {
            Some(val) => Ok(val.clone()),
            // If the value is not present, allow for an optional default value
            None => match default {
                Some(def) => Ok(def.clone()),
                None => Err(Error::msg(format!(
                    "Filter `get` tried to get key `{}` but it wasn't found",
                    &key
                ))),
            },
        },
        None => Err(Error::msg("Filter `get` was used on a value that isn't an object")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::value::to_value;
    use std::collections::HashMap;

    #[test]
    fn test_get_filter_exists() {
        let mut obj = HashMap::new();
        obj.insert("1".to_string(), "first".to_string());
        obj.insert("2".to_string(), "second".to_string());

        let mut args = HashMap::new();
        args.insert("key".to_string(), to_value("1").unwrap());
        let result = get(&to_value(&obj).unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("first").unwrap());
    }

    #[test]
    fn test_get_filter_doesnt_exist() {
        let mut obj = HashMap::new();
        obj.insert("1".to_string(), "first".to_string());
        obj.insert("2".to_string(), "second".to_string());

        let mut args = HashMap::new();
        args.insert("key".to_string(), to_value("3").unwrap());
        let result = get(&to_value(&obj).unwrap(), &args);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_filter_with_default_exists() {
        let mut obj = HashMap::new();
        obj.insert("1".to_string(), "first".to_string());
        obj.insert("2".to_string(), "second".to_string());

        let mut args = HashMap::new();
        args.insert("key".to_string(), to_value("1").unwrap());
        args.insert("default".to_string(), to_value("default").unwrap());
        let result = get(&to_value(&obj).unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("first").unwrap());
    }

    #[test]
    fn test_get_filter_with_default_doesnt_exist() {
        let mut obj = HashMap::new();
        obj.insert("1".to_string(), "first".to_string());
        obj.insert("2".to_string(), "second".to_string());

        let mut args = HashMap::new();
        args.insert("key".to_string(), to_value("3").unwrap());
        args.insert("default".to_string(), to_value("default").unwrap());
        let result = get(&to_value(&obj).unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("default").unwrap());
    }
}
