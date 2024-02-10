/// Filters operating on objects
use std::collections::HashMap;

use serde_json::value::Value;
use serde_json::value::to_value;

use crate::errors::{Error, Result};

/// Returns a value by a `key` argument from a given object
pub fn get(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
    let default = args.get("default");
    let key = match args.get("key") {
        Some(val) => try_get_value!("get", "key", String, val),
        None => return Err(Error::msg("The `get` filter has to have a `key` argument")),
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

/// Merge two objects, the second object is indicated by the `other` argument.
/// The second object's values will overwrite the first's in the event of a key conflict.
pub fn merge(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
    let left = match value.as_object() {
        Some(val) => val,
        None => return Err(Error::msg("Filter `merge` was used on a value that isn't an object")),
    };
    match args.get("other") {
        Some(val) => match val.as_object() {
            Some(right) => {
                let mut result = left.clone();
                result.extend(right.clone());
                // We've already confirmed both sides were HashMaps, the result is a HashMap -
                // - so unwrap
                Ok(to_value(result).unwrap())
            },
            None => Err(Error::msg("The `other` argument for the `get` filter must be an object"))
        },
        None =>  Err(Error::msg("The `merge` filter has to have an `other` argument")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    #[test]
    fn test_merge_filter() {
        let mut obj_1 = HashMap::new();
        obj_1.insert("1".to_string(), "first".to_string());
        obj_1.insert("2".to_string(), "second".to_string());

        let mut obj_2 = HashMap::new();
        obj_2.insert("2".to_string(), "SECOND".to_string());
        obj_2.insert("3".to_string(), "third".to_string());

        let mut args = HashMap::new();
        args.insert("other".to_string(), to_value(obj_2).unwrap());

        let result = merge(&to_value(&obj_1).unwrap(), &args);
        assert!(result.is_ok());

        let mut expected = HashMap::new();
        expected.insert("1".to_string(), "first".to_string());
        expected.insert("2".to_string(), "SECOND".to_string());
        expected.insert("3".to_string(), "third".to_string());
        assert_eq!(result.unwrap(), to_value(expected).unwrap());
    }
}
