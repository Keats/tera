/// Filters operating on array
use std::collections::HashMap;

use serde_json::value::{to_value, Value};
use context::{get_json_pointer, ValueRender};
use errors::Result;
use sort_utils::get_sort_strategy_for_type;

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
pub fn join(value: Value, args: HashMap<String, Value>) -> Result<Value> {
    let arr = try_get_value!("join", "value", Vec<Value>, value);
    let sep = match args.get("sep") {
        Some(val) => try_get_value!("truncate", "sep", String, val),
        None => String::new(),
    };

    // Convert all the values to strings before we join them together.
    let rendered = arr.iter().map(|val| val.render()).collect::<Vec<_>>();
    Ok(to_value(&rendered.join(&sep))?)
}

/// Sorts the array in ascending order.
/// Use the 'attribute' argument to define a field to sort by.
pub fn sort(value: Value, args: HashMap<String, Value>) -> Result<Value> {
    let mut arr = try_get_value!("sort", "value", Vec<Value>, value);
    if arr.is_empty() {
        return Ok(arr.into());
    }

    let attribute = match args.get("attribute") {
        Some(val) => try_get_value!("sort", "attribute", String, val),
        None => String::new(),
    };
    let ptr = match attribute.as_str() {
        "" => "".to_string(),
        s => get_json_pointer(s),
    };

    let first = arr[0]
        .pointer(&ptr)
        .ok_or_else(|| format!("attribute '{}' does not reference a field", attribute))?;

    let mut strategy = get_sort_strategy_for_type(first)?;
    for v in &arr {
        let key = v.pointer(&ptr)
            .ok_or_else(|| format!("attribute '{}' does not reference a field", attribute))?;
        strategy.try_add_pair(v, key)?;
    }
    let sorted = strategy.sort();

    Ok(sorted.into())
}

/// Slice the array
/// Use the `start` argument to define where to start (inclusive, default to `0`)
/// and `end` argument to define where to stop (exclusive, default to the length of the array)
/// `start` and `end` are 0-indexed
pub fn slice(value: Value, args: HashMap<String, Value>) -> Result<Value> {
    let mut arr = try_get_value!("slice", "value", Vec<Value>, value);
    if arr.is_empty() {
        return Ok(arr.into());
    }

    let start = match args.get("start") {
        Some(val) => try_get_value!("slice", "start", f64, val) as usize,
        None => 0,
    };
    // Not an error, but returns an empty Vec
    if start > arr.len() {
        return Ok(Vec::<Value>::new().into());
    }
    let mut end = match args.get("end") {
        Some(val) => try_get_value!("slice", "end", f64, val) as usize,
        None => arr.len(),
    };
    if end > arr.len() {
        end = arr.len();
    }

    Ok(arr[start..end].into())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use serde_json::value::{to_value, Value};
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

    #[test]
    fn test_sort() {
        let v = to_value(vec![3, 1, 2, 5, 4]).unwrap();
        let args = HashMap::new();
        let result = sort(v, args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(vec![1, 2, 3, 4, 5]).unwrap());
    }

    #[test]
    fn test_sort_empty() {
        let v = to_value(Vec::<f64>::new()).unwrap();
        let args = HashMap::new();
        let result = sort(v, args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(Vec::<f64>::new()).unwrap());
    }

    #[derive(Serialize)]
    struct Foo {
        a: i32,
        b: i32,
    }

    #[test]
    fn test_sort_attribute() {
        let v = to_value(vec![
            Foo { a: 3, b: 5 },
            Foo { a: 2, b: 8 },
            Foo { a: 4, b: 7 },
            Foo { a: 1, b: 6 },
        ]).unwrap();
        let mut args = HashMap::new();
        args.insert("attribute".to_string(), to_value(&"a").unwrap());

        let result = sort(v, args);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            to_value(vec![
                Foo { a: 1, b: 6 },
                Foo { a: 2, b: 8 },
                Foo { a: 3, b: 5 },
                Foo { a: 4, b: 7 },
            ]).unwrap()
        );
    }

    #[test]
    fn test_sort_invalid_attribute() {
        let v = to_value(vec![Foo { a: 3, b: 5 }]).unwrap();
        let mut args = HashMap::new();
        args.insert("attribute".to_string(), to_value(&"invalid_field").unwrap());

        let result = sort(v, args);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().description(),
            "attribute 'invalid_field' does not reference a field"
        );
    }

    #[test]
    fn test_sort_multiple_types() {
        let v = to_value(vec![Value::Number(12.into()), Value::Array(vec![])]).unwrap();
        let args = HashMap::new();

        let result = sort(v, args);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().description(), "expected number got []");
    }

    #[test]
    fn test_sort_non_finite_numbers() {
        let v = to_value(vec![
            ::std::f64::NEG_INFINITY, // NaN and friends get deserialized as Null by serde.
            ::std::f64::NAN,
        ]).unwrap();
        let args = HashMap::new();

        let result = sort(v, args);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().description(),
            "Null is not a sortable value"
        );
    }

    #[derive(Serialize)]
    struct TupleStruct(i32, i32);

    #[test]
    fn test_sort_tuple() {
        let v = to_value(vec![
            TupleStruct(0, 1),
            TupleStruct(7, 0),
            TupleStruct(-1, 12),
            TupleStruct(18, 18),
        ]).unwrap();
        let mut args = HashMap::new();
        args.insert("attribute".to_string(), to_value("0").unwrap());

        let result = sort(v, args);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            to_value(vec![
                TupleStruct(-1, 12),
                TupleStruct(0, 1),
                TupleStruct(7, 0),
                TupleStruct(18, 18),
            ]).unwrap()
        );
    }

    #[test]
    fn test_slice() {
        fn make_args(start: Option<usize>, end: Option<usize>) -> HashMap<String, Value> {
            let mut args = HashMap::new();
            if let Some(s) = start {
                args.insert("start".to_string(), to_value(s).unwrap());
            }
            if let Some(e) = end {
                args.insert("end".to_string(), to_value(e).unwrap());
            }
            args
        }

        let v = to_value(vec![1, 2, 3, 4, 5]).unwrap();

        let inputs = vec![
            (make_args(Some(1), None), vec![2, 3, 4, 5]),
            (make_args(None, Some(2)), vec![1, 2]),
            (make_args(Some(1), Some(2)), vec![2]),
            (make_args(None, None), vec![1, 2, 3, 4, 5]),
        ];

        for (args, expected) in inputs {
            let res = slice(v.clone(), args);
            assert!(res.is_ok());
            assert_eq!(res.unwrap(), to_value(expected).unwrap());
        }
    }
}
