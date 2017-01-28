/// Filters operating on array
use std::collections::HashMap;

use serde_json::value::{Value, to_value};
use context::ValueRender;
use errors::Result;

/// Returns the first value of an array
/// If the array is empty, returns empty string
pub fn first(value: Value, _: HashMap<String, Value>) -> Result<Value> {
    let mut arr = try_get_value!("first", "value", Vec<Value>, value);

    if arr.is_empty() {
        Ok(to_value("").unwrap())
    } else {
        Ok(arr.swap_remove(0))
    }
}

/// Returns the last value of an array
/// If the array is empty, returns empty string
pub fn last(value: Value, _: HashMap<String, Value>) -> Result<Value> {
    let mut arr = try_get_value!("last", "value", Vec<Value>, value);

    Ok(arr.pop().unwrap_or_else(|| to_value("").unwrap()))
}

/// Joins all values in the array by the `sep` argument given
/// If no separator is given, it will use `""` (empty string) as separator
/// If the array is empty, returns empty string
pub fn join(value: Value, mut args: HashMap<String, Value>) -> Result<Value> {
    let arr = try_get_value!("join", "value", Vec<Value>, value);
    let sep = match args.remove("sep") {
        Some(val) => try_get_value!("truncate", "sep", String, val),
        None => "".to_string(),
    };

    // Convert all the values to strings before we join them together.
    let rendered = arr.iter().map(|val| val.render()).collect::<Vec<_>>();
    Ok(to_value(&rendered.join(&sep))?)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use serde_json::value::{Value, to_value};
    use super::*;

    #[test]
    fn test_first() {
        let result = first(to_value(&vec![1, 2, 3, 4]).unwrap(), HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(&1).unwrap());
    }

    #[test]
    fn test_first_empty() {
        let v: Vec<Value> = Vec::new();

        let result = first(to_value(&v).unwrap(), HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.ok().unwrap(), to_value("").unwrap());
    }

    #[test]
    fn test_last() {
        let result = last(to_value(&vec!["Hello", "World"]).unwrap(), HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("World").unwrap());
    }

    #[test]
    fn test_last_empty() {
        let v: Vec<Value> = Vec::new();

        let result = last(to_value(&v).unwrap(), HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.ok().unwrap(), to_value("").unwrap());
    }

    #[test]
    fn test_join_sep() {
        let mut args = HashMap::new();
        args.insert("sep".to_owned(), to_value(&"==").unwrap());

        let result = join(to_value(&vec!["Cats", "Dogs"]).unwrap(), args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(&"Cats==Dogs").unwrap());
    }

    #[test]
    fn test_join_sep_omitted() {
        let result = join(to_value(&vec![1.2, 3.4]).unwrap(), HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(&"1.23.4").unwrap());
    }

    #[test]
    fn test_join_empty() {
        let v: Vec<Value> = Vec::new();
        let mut args = HashMap::new();
        args.insert("sep".to_owned(), to_value(&"==").unwrap());

        let result = join(to_value(&v).unwrap(), args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(&"").unwrap());
    }
}
