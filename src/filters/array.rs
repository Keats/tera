/// Filters operating on string
use std::collections::HashMap;

use serde_json::value::{Value, to_value};
use context::JsonRender;
use errors::TeraResult;
use errors::TeraError;

pub fn first(value: Value, _: HashMap<String, Value>) -> TeraResult<Value> {
    let arr = try_get_value!("first", "value", Vec<Value>, value);

    if let Some(val) = arr.first() {
        Ok(val.clone())
    } else {
        Err(TeraError::EmptyArray("first".to_string()))
    }
}

pub fn last(value: Value, _: HashMap<String, Value>) -> TeraResult<Value> {
    let arr = try_get_value!("last","value",Vec<Value>, value);

    if let Some(val) = arr.last() {
        Ok(val.clone())
    } else {
        Err(TeraError::EmptyArray("last".to_string()))
    }
}

pub fn join(value: Value, args: HashMap<String, Value>) -> TeraResult<Value> {
    let arr = try_get_value!("join","value",Vec<Value>, value);
    let separator = match args.get("d") {
        Some(val) => try_get_value!("truncate", "d", String, val.clone()),
        None => "".to_string(),
    };
    let mut reduction = arr.iter()
        .fold("".to_string(),
              |acc, val| acc + &*separator + &*val.render());
    if reduction.len() > 0 {
        // if we had any elements, trim the separator off the front.
        reduction = reduction.split_at(separator.len()).1.to_string();
    }

    Ok(to_value(&reduction))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serde_json::value::{Value, to_value};

    use errors::TeraError::*;

    use super::*;

    #[test]
    fn test_first() {
        let result = first(to_value(&vec![1,2,3,4]), HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(&1));
    }

    #[test]
    fn test_first_empty() {
        let v: Vec<Value> = Vec::new();

        let result = first(to_value(&v), HashMap::new());
        assert!(result.is_err());
        assert_eq!(result.err().unwrap(), EmptyArray("first".to_string()));
    }

    #[test]
    fn test_last() {
        let result = last(to_value(&vec!["Hello","World"]), HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(&"World"));
    }

    #[test]
    fn test_last_empty() {
        let v: Vec<Value> = Vec::new();

        let result = last(to_value(&v), HashMap::new());
        assert!(result.is_err());
        assert_eq!(result.err().unwrap(), EmptyArray("last".to_string()));
    }

    #[test]
    fn test_join_sep() {
        let sep = "==".to_string();
        let mut args = HashMap::new();
        args.insert("d".to_owned(), to_value(&sep));

        let result = join(to_value(&vec!["Cats","Dogs"]), args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(&"Cats==Dogs"));
    }

    #[test]
    fn test_join_sep_omitted() {
        let result = join(to_value(&vec![1.2,3.4]), HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(&"1.23.4"));
    }

    #[test]
    fn test_join_empty() {
        let v: Vec<Value> = Vec::new();
        let sep = "==".to_string();
        let mut args = HashMap::new();
        args.insert("d".to_owned(), to_value(&sep));

        let result = join(to_value(&v), args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(&""));
    }
}