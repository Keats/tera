/// Filters operating on multiple types
use std::collections::HashMap;
use std::iter::FromIterator;

use serde_json::value::{Value, to_value};
use errors::{TeraError, TeraResult};

// Returns the number of items in an array or the number of characters in a string.
// Returns 0 if not an array or string.
pub fn length(value: Value, _: HashMap<String, Value>) -> TeraResult<Value> {
    match value {
        Value::Array(arr) => Ok(to_value(&arr.len())),
        Value::String(s) => Ok(to_value(&s.chars().count())),
        _ => Ok(to_value(&0)),
    }
}

// Reverses the elements of an array or the characters in a string.
pub fn reverse(value: Value, _: HashMap<String, Value>) -> TeraResult<Value> {
    match value {
        Value::Array(arr) => {
            // Clone the array so that we don't mutate the original.
            let mut rev = arr.clone();
            rev.reverse();
            Ok(to_value(&rev))
        }
        Value::String(s) => Ok(to_value(&String::from_iter(s.chars().rev()))),
        _ => {
            Err(TeraError::FilterIncorrectArgType(
                "reverse".to_string(), "value".to_string(), value, "Array|String".to_string()
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use serde_json::value::to_value;
    use errors::TeraError;
    use super::*;

    #[test]
    fn test_length_vec() {
        let result = length(to_value(&vec![1, 2, 3, 4]), HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(&4));
    }

    #[test]
    fn test_length_str() {
        let result = length(to_value(&"Hello World"), HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(&11));
    }

    #[test]
    fn test_length_str_nonascii() {
        let result = length(to_value(&"日本語"), HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(&3));
    }

    #[test]
    fn test_length_num() {
        let result = length(to_value(&15), HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(&0));
    }

    #[test]
    fn test_reverse_vec() {
        let result = reverse(to_value(&vec![1, 2, 3, 4]), HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(&vec![4, 3, 2, 1]));
    }

    #[test]
    fn test_reverse_str() {
        let result = reverse(to_value(&"Hello World"), HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(&"dlroW olleH"));
    }

    #[test]
    fn test_reverse_num() {
        let result = reverse(to_value(&1.23), HashMap::new());
        assert!(result.is_err());
        assert_eq!(result.err().unwrap(),
                   TeraError::FilterIncorrectArgType("reverse".to_string(),
                                                     "value".to_string(),
                                                     to_value(&1.23),
                                                     "Array|String".to_string()));
    }
}
