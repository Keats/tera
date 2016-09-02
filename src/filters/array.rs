/// Filters operating on array
use std::collections::HashMap;

use serde_json::value::{Value, to_value};
use context::ValueRender;
use errors::TeraResult;

/// Returns the first value of an array
/// If the array is empty, returns empty string
pub fn first(value: Value, _: HashMap<String, Value>) -> TeraResult<Value> {
    let arr = try_get_value!("first", "value", Vec<Value>, value);

    if let Some(val) = arr.first() {
        Ok(val.clone())
    } else {
        Ok(to_value(&""))
    }
}

/// Returns the last value of an array
/// If the array is empty, returns empty string
pub fn last(value: Value, _: HashMap<String, Value>) -> TeraResult<Value> {
    let arr = try_get_value!("last", "value", Vec<Value>, value);

    if let Some(val) = arr.last() {
        Ok(val.clone())
    } else {
        Ok(to_value(&""))
    }
}

/// Joins all values in the array by the `sep` argument given
/// If no separator is given, it will use `""` (empty string) as separator
/// If the array is empty, returns empty string
pub fn join(value: Value, args: HashMap<String, Value>) -> TeraResult<Value> {
    let arr = try_get_value!("join", "value", Vec<Value>, value);
    let sep = match args.get("sep") {
        Some(val) => try_get_value!("truncate", "sep", String, val.clone()),
        None => "".to_string(),
    };

    // Convert all the values to strings before we join them together.
    let rendered = arr.iter().map(|val| val.render()).collect::<Vec<_>>();

    Ok(to_value(&rendered.join(&sep)))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use serde_json::value::{Value, to_value};
    use super::*;

    #[test]
    fn test_first() {
        let result = first(to_value(&vec![1, 2, 3, 4]), HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(&1));
    }

    #[test]
    fn test_first_empty() {
        let v: Vec<Value> = Vec::new();

        let result = first(to_value(&v), HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.ok().unwrap(), to_value(&""));
    }

    #[test]
    fn test_last() {
        let result = last(to_value(&vec!["Hello", "World"]), HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(&"World"));
    }

    #[test]
    fn test_last_empty() {
        let v: Vec<Value> = Vec::new();

        let result = last(to_value(&v), HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.ok().unwrap(), to_value(&""));
    }

    #[test]
    fn test_join_sep() {
        let mut args = HashMap::new();
        args.insert("sep".to_owned(), to_value(&"=="));

        let result = join(to_value(&vec!["Cats", "Dogs"]), args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(&"Cats==Dogs"));
    }

    #[test]
    fn test_join_sep_omitted() {
        let result = join(to_value(&vec![1.2, 3.4]), HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(&"1.23.4"));
    }

    #[test]
    fn test_join_empty() {
        let v: Vec<Value> = Vec::new();
        let mut args = HashMap::new();
        args.insert("sep".to_owned(), to_value(&"=="));

        let result = join(to_value(&v), args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(&""));
    }
}
