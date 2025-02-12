/// Filters operating on numbers
use std::collections::HashMap;


use crate::dotted_pointer;
use crate::errors::{Error, Result};
use serde_json::value::Value;

/// Returns a value by a `key` argument from a given object
pub fn get(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
    let default = args.get("default");
    let key = match args.get("key") {
        Some(val) => try_get_value!("get", "key", String, val),
        None => return Err(Error::msg("The `get` filter has to have an `key` argument")),
    };

    if value.is_object() {
        match dotted_pointer(&value, &key) {
            None => match default {
                Some(def) => Ok(def.clone()),
                None => Err(Error::msg(format!(
                    "Filter `get` tried to get key `{}` but it wasn't found",
                    &key
                ))),
            },
            Some(val) => Ok(val.clone()),
        }
    } else {
        Err(Error::msg("Filter `get` was used on a value that isn't an object"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::value::to_value;
    use std::collections::HashMap;
    use serde_json::json;
    use crate::filters::array::batch;

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
    fn test_get_attribute() {
        let input = json!({"id": 7, "year": [1900, 1901], "children": [{"id": 0}]});
        let mut args = HashMap::new();
        args.insert("key".to_string(), to_value("id").unwrap());
        args.insert("default".to_string(), to_value(3).unwrap());

        let expected = json!(7);

        let res = get(&input, &args);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), to_value(expected).unwrap());


        let mut args = HashMap::new();
        args.insert("key".to_string(), to_value("id2").unwrap());
        args.insert("default".to_string(), to_value(3).unwrap());

        let expected = json!(3);

        let res = get(&input, &args);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), to_value(expected).unwrap());


        let mut args = HashMap::new();
        args.insert("key".to_string(), to_value("year.3").unwrap());
        args.insert("default".to_string(), to_value(3).unwrap());

        let expected = json!(3);

        let res = get(&input, &args);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), to_value(expected).unwrap());

        let mut args = HashMap::new();
        args.insert("key".to_string(), to_value("children.0.id").unwrap());
        args.insert("default".to_string(), to_value(3).unwrap());

        let expected = json!(0);

        let res = get(&input, &args);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), to_value(expected).unwrap());
    }
}
