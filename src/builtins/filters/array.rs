/// Filters operating on array
use std::collections::HashMap;
use std::cmp::Ordering;

use serde_json::value::{Value, to_value};
use context::{ValueRender, get_json_pointer};
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
pub fn join(value: Value, args: HashMap<String, Value>) -> Result<Value> {
    let arr = try_get_value!("join", "value", Vec<Value>, value);
    let sep = match args.get("sep") {
        Some(val) => try_get_value!("truncate", "sep", String, val),
        None => "".to_string(),
    };

    // Convert all the values to strings before we join them together.
    let rendered = arr.iter().map(|val| val.render()).collect::<Vec<_>>();
    Ok(to_value(&rendered.join(&sep))?)
}

/// Sorts the array in ascending order.
/// Use the 'attribute' argument to define a field to sort by.
/// Set the 'reverse' argument to sort in descending order.
pub fn sort(value: Value, args: HashMap<String, Value>) -> Result<Value> {
    let mut arr = try_get_value!("sort", "value", Vec<Value>, value);
    let reverse = try_get_value!("sort", "reverse", bool, args.get("reverse").unwrap_or(&false.into()));
    let attribute = try_get_value!("sort", "attribute", String, args.get("attribute").unwrap_or(&"".into()));
    let attribute = get_json_pointer(&attribute);

    arr.sort_unstable_by(|a, b| {
        value_cmp(
            a.pointer(&attribute).unwrap_or(&Value::Null),
            b.pointer(&attribute).unwrap_or(&Value::Null)
        )
    });
    if reverse {
        arr.reverse();
    }

    Ok(to_value(arr)?)
}

fn value_cmp(a: &Value, b: &Value) -> Ordering {
    use Value::*;
    use self::Ordering::*;
    match (a, b) {
        (&Null, &Null) => Equal,
        (&Bool(ref a), &Bool(ref b)) => a.cmp(b),
        (&Number(ref a), &Number(ref b)) => a.as_f64().unwrap().partial_cmp(&b.as_f64().unwrap()).unwrap(),
        (&String(ref a), &String(ref b)) => a.cmp(b),
        (&Array(ref a), &Array(ref b)) => a.len().cmp(&b.len()),
        (&Object(ref a), &Object(ref b)) => a.len().cmp(&b.len()),
        (a, b) => type_of(a).cmp(&type_of(b))
    }
}

fn type_of(val: &Value) -> usize {
    use Value::*;
    match *val {
        Null => 0,
        Bool(_) => 1,
        Number(_) => 2,
        String(_) => 3,
        Array(_) => 4, Object(_) => 5
    }
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

    #[test]
    fn test_sort() {
        let v = to_value(vec![3, 1, 2, 5, 4]).unwrap();
        let args = HashMap::new();
        let result = sort(v, args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(vec![1, 2, 3, 4, 5]).unwrap());
    }

    #[test]
    fn test_sort_descending() {
        let v = to_value(vec![3, 1, 2, 5, 4]).unwrap();
        let mut args = HashMap::new();
        args.insert("reverse".to_string(), to_value(true).unwrap());

        let result = sort(v, args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(vec![5, 4, 3, 2, 1]).unwrap());
    }

    #[derive(Serialize)]
    struct Foo {
        a: i32,
        b: i32
    }

    #[test]
    fn test_sort_attribute() {
        let v = to_value(vec![
            Foo {a: 3, b: 5},
            Foo {a: 2, b: 8},
            Foo {a: 4, b: 7},
            Foo {a: 1, b: 6},
        ]).unwrap();
        let mut args = HashMap::new();
        args.insert("attribute".to_string(), to_value(&"a").unwrap());

        let result = sort(v, args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(vec![
            Foo {a: 1, b: 6},
            Foo {a: 2, b: 8},
            Foo {a: 3, b: 5},
            Foo {a: 4, b: 7},
        ]).unwrap());
    }
}
