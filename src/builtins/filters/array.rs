/// Filters operating on array
use std::collections::HashMap;

use crate::context::{dotted_pointer, ValueRender};
use crate::errors::{Error, Result};
use crate::filter_utils::{get_sort_strategy_for_type, get_unique_strategy_for_type};
use crate::utils::render_to_string;
use serde_json::value::{to_value, Map, Value};

/// Returns the nth value of an array
/// If the array is empty, returns empty string
pub fn nth(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
    let arr = try_get_value!("nth", "value", Vec<Value>, value);

    if arr.is_empty() {
        return Ok(to_value("").unwrap());
    }

    let index = match args.get("n") {
        Some(val) => try_get_value!("nth", "n", usize, val),
        None => return Err(Error::msg("The `nth` filter has to have an `n` argument")),
    };

    Ok(arr.get(index).unwrap_or(&to_value("").unwrap()).to_owned())
}

/// Returns the first value of an array
/// If the array is empty, returns empty string
pub fn first(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let mut arr = try_get_value!("first", "value", Vec<Value>, value);

    if arr.is_empty() {
        Ok(to_value("").unwrap())
    } else {
        Ok(arr.swap_remove(0))
    }
}

/// Returns the last value of an array
/// If the array is empty, returns empty string
pub fn last(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let mut arr = try_get_value!("last", "value", Vec<Value>, value);

    Ok(arr.pop().unwrap_or_else(|| to_value("").unwrap()))
}

/// Joins all values in the array by the `sep` argument given
/// If no separator is given, it will use `""` (empty string) as separator
/// If the array is empty, returns empty string
pub fn join(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
    let arr = try_get_value!("join", "value", Vec<Value>, value);
    let sep = match args.get("sep") {
        Some(val) => {
            let s = try_get_value!("truncate", "sep", String, val);
            // When reading from a file, it will escape `\n` to `\\n` for example so we need
            // to replace double escape. In practice it might cause issues if someone wants to join
            // with `\\n` for real but that seems pretty unlikely
            s.replace("\\n", "\n").replace("\\t", "\t")
        }
        None => String::new(),
    };

    // Convert all the values to strings before we join them together.
    let rendered = arr
        .iter()
        .map(|v| render_to_string(|| "joining array".to_string(), |w| v.render(w)))
        .collect::<Result<Vec<_>>>()?;
    to_value(rendered.join(&sep)).map_err(Error::json)
}

/// Sorts the array in ascending order.
/// Use the 'attribute' argument to define a field to sort by.
pub fn sort(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
    let arr = try_get_value!("sort", "value", Vec<Value>, value);
    if arr.is_empty() {
        return Ok(arr.into());
    }

    let attribute = match args.get("attribute") {
        Some(val) => try_get_value!("sort", "attribute", String, val),
        None => String::new(),
    };

    let first = dotted_pointer(&arr[0], &attribute).ok_or_else(|| {
        Error::msg(format!("attribute '{}' does not reference a field", attribute))
    })?;

    let mut strategy = get_sort_strategy_for_type(first)?;
    for v in &arr {
        let key = dotted_pointer(v, &attribute).ok_or_else(|| {
            Error::msg(format!("attribute '{}' does not reference a field", attribute))
        })?;
        strategy.try_add_pair(v, key)?;
    }
    let sorted = strategy.sort();

    Ok(sorted.into())
}

/// Remove duplicates from an array.
/// Use the 'attribute' argument to define a field to filter on.
/// For strings, use the 'case_sensitive' argument (defaults to false) to control the comparison.
pub fn unique(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
    let arr = try_get_value!("unique", "value", Vec<Value>, value);
    if arr.is_empty() {
        return Ok(arr.into());
    }

    let case_sensitive = match args.get("case_sensitive") {
        Some(val) => try_get_value!("unique", "case_sensitive", bool, val),
        None => false,
    };

    let attribute = match args.get("attribute") {
        Some(val) => try_get_value!("unique", "attribute", String, val),
        None => String::new(),
    };

    let first = dotted_pointer(&arr[0], &attribute).ok_or_else(|| {
        Error::msg(format!("attribute '{}' does not reference a field", attribute))
    })?;

    let disc = std::mem::discriminant(first);
    let mut strategy = get_unique_strategy_for_type(first, case_sensitive)?;

    let arr = arr
        .into_iter()
        .filter_map(|v| match dotted_pointer(&v, &attribute) {
            Some(key) => {
                if disc == std::mem::discriminant(key) {
                    match strategy.insert(key) {
                        Ok(false) => None,
                        Ok(true) => Some(Ok(v)),
                        Err(e) => Some(Err(e)),
                    }
                } else {
                    Some(Err(Error::msg("unique filter can't compare multiple types")))
                }
            }
            None => None,
        })
        .collect::<Result<Vec<_>>>();

    Ok(to_value(arr?).unwrap())
}

/// Group the array values by the `attribute` given
/// Returns a hashmap of key => values, items without the `attribute` or where `attribute` is `null` are discarded.
/// The returned keys are stringified
pub fn group_by(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
    let arr = try_get_value!("group_by", "value", Vec<Value>, value);
    if arr.is_empty() {
        return Ok(Map::new().into());
    }

    let key = match args.get("attribute") {
        Some(val) => try_get_value!("group_by", "attribute", String, val),
        None => {
            return Err(Error::msg("The `group_by` filter has to have an `attribute` argument"))
        }
    };

    let mut grouped = Map::new();

    for val in arr {
        if let Some(key_val) = dotted_pointer(&val, &key).cloned() {
            if key_val.is_null() {
                continue;
            }

            let str_key = match key_val.as_str() {
                Some(key) => key.to_owned(),
                None => format!("{}", key_val),
            };

            if let Some(vals) = grouped.get_mut(&str_key) {
                vals.as_array_mut().unwrap().push(val);
                continue;
            }

            grouped.insert(str_key, Value::Array(vec![val]));
        }
    }

    Ok(to_value(grouped).unwrap())
}

/// Filter the array values, returning only the values where the `attribute` is equal to the `value`
/// Values without the `attribute` or with a null `attribute` are discarded
/// If the `value` is not passed, discard all elements where the attribute is null.
pub fn filter(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
    let mut arr = try_get_value!("filter", "value", Vec<Value>, value);
    if arr.is_empty() {
        return Ok(arr.into());
    }

    let key = match args.get("attribute") {
        Some(val) => try_get_value!("filter", "attribute", String, val),
        None => return Err(Error::msg("The `filter` filter has to have an `attribute` argument")),
    };
    let value = args.get("value").unwrap_or(&Value::Null);

    arr = arr
        .into_iter()
        .filter(|v| {
            let val = dotted_pointer(v, &key).unwrap_or(&Value::Null);
            if value.is_null() {
                !val.is_null()
            } else {
                val == value
            }
        })
        .collect::<Vec<_>>();

    Ok(to_value(arr).unwrap())
}

/// Map retrieves an attribute from a list of objects.
/// The 'attribute' argument specifies what to retrieve.
pub fn map(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
    let arr = try_get_value!("map", "value", Vec<Value>, value);
    if arr.is_empty() {
        return Ok(arr.into());
    }

    let attribute = match args.get("attribute") {
        Some(val) => try_get_value!("map", "attribute", String, val),
        None => return Err(Error::msg("The `map` filter has to have an `attribute` argument")),
    };

    let arr = arr
        .into_iter()
        .filter_map(|v| match dotted_pointer(&v, &attribute) {
            Some(val) if !val.is_null() => Some(val.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();

    Ok(to_value(arr).unwrap())
}

#[inline]
fn get_index(i: f64, array: &[Value]) -> usize {
    if i >= 0.0 {
        i as usize
    } else {
        (array.len() as f64 + i) as usize
    }
}

/// Slice the array
/// Use the `start` argument to define where to start (inclusive, default to `0`)
/// and `end` argument to define where to stop (exclusive, default to the length of the array)
/// `start` and `end` are 0-indexed
pub fn slice(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
    let arr = try_get_value!("slice", "value", Vec<Value>, value);
    if arr.is_empty() {
        return Ok(arr.into());
    }

    let start = match args.get("start") {
        Some(val) => get_index(try_get_value!("slice", "start", f64, val), &arr),
        None => 0,
    };

    let mut end = match args.get("end") {
        Some(val) => get_index(try_get_value!("slice", "end", f64, val), &arr),
        None => arr.len(),
    };

    if end > arr.len() {
        end = arr.len();
    }

    // Not an error, but returns an empty Vec
    if start >= end {
        return Ok(Vec::<Value>::new().into());
    }

    Ok(arr[start..end].into())
}

/// Concat the array with another one if the `with` parameter is an array or
/// just append it otherwise
pub fn concat(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
    let mut arr = try_get_value!("concat", "value", Vec<Value>, value);

    let value = match args.get("with") {
        Some(val) => val,
        None => return Err(Error::msg("The `concat` filter has to have a `with` argument")),
    };

    if value.is_array() {
        match value {
            Value::Array(vals) => {
                for val in vals {
                    arr.push(val.clone());
                }
            }
            _ => unreachable!("Got something other than an array??"),
        }
    } else {
        arr.push(value.clone());
    }

    Ok(to_value(arr).unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_derive::{Deserialize, Serialize};
    use serde_json::json;
    use serde_json::value::{to_value, Value};
    use std::collections::HashMap;

    #[test]
    fn test_nth() {
        let mut args = HashMap::new();
        args.insert("n".to_string(), to_value(1).unwrap());
        let result = nth(&to_value(vec![1, 2, 3, 4]).unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(2).unwrap());
    }

    #[test]
    fn test_nth_empty() {
        let v: Vec<Value> = Vec::new();
        let mut args = HashMap::new();
        args.insert("n".to_string(), to_value(1).unwrap());
        let result = nth(&to_value(v).unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("").unwrap());
    }

    #[test]
    fn test_first() {
        let result = first(&to_value(vec![1, 2, 3, 4]).unwrap(), &HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(1).unwrap());
    }

    #[test]
    fn test_first_empty() {
        let v: Vec<Value> = Vec::new();

        let result = first(&to_value(v).unwrap(), &HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.ok().unwrap(), to_value("").unwrap());
    }

    #[test]
    fn test_last() {
        let result = last(&to_value(vec!["Hello", "World"]).unwrap(), &HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("World").unwrap());
    }

    #[test]
    fn test_last_empty() {
        let v: Vec<Value> = Vec::new();

        let result = last(&to_value(v).unwrap(), &HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.ok().unwrap(), to_value("").unwrap());
    }

    #[test]
    fn test_join_sep() {
        let mut args = HashMap::new();
        args.insert("sep".to_owned(), to_value("==").unwrap());

        let result = join(&to_value(vec!["Cats", "Dogs"]).unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("Cats==Dogs").unwrap());
    }

    #[test]
    fn test_join_sep_omitted() {
        let result = join(&to_value(vec![1.2, 3.4]).unwrap(), &HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("1.23.4").unwrap());
    }

    #[test]
    fn test_join_empty() {
        let v: Vec<Value> = Vec::new();
        let mut args = HashMap::new();
        args.insert("sep".to_owned(), to_value("==").unwrap());

        let result = join(&to_value(v).unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("").unwrap());
    }

    #[test]
    fn test_join_newlines_and_tabs() {
        let mut args = HashMap::new();
        args.insert("sep".to_owned(), to_value(",\\n\\t").unwrap());
        let result = join(&to_value(vec!["Cats", "Dogs"]).unwrap(), &args);
        assert_eq!(result.unwrap(), to_value("Cats,\n\tDogs").unwrap());
    }

    #[test]
    fn test_sort() {
        let v = to_value(vec![3, -1, 2, 5, 4]).unwrap();
        let args = HashMap::new();
        let result = sort(&v, &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(vec![-1, 2, 3, 4, 5]).unwrap());
    }

    #[test]
    fn test_sort_empty() {
        let v = to_value(Vec::<f64>::new()).unwrap();
        let args = HashMap::new();
        let result = sort(&v, &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(Vec::<f64>::new()).unwrap());
    }

    #[derive(Deserialize, Eq, Hash, PartialEq, Serialize)]
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
        ])
        .unwrap();
        let mut args = HashMap::new();
        args.insert("attribute".to_string(), to_value("a").unwrap());

        let result = sort(&v, &args);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            to_value(vec![
                Foo { a: 1, b: 6 },
                Foo { a: 2, b: 8 },
                Foo { a: 3, b: 5 },
                Foo { a: 4, b: 7 },
            ])
            .unwrap()
        );
    }

    #[test]
    fn test_sort_invalid_attribute() {
        let v = to_value(vec![Foo { a: 3, b: 5 }]).unwrap();
        let mut args = HashMap::new();
        args.insert("attribute".to_string(), to_value("invalid_field").unwrap());

        let result = sort(&v, &args);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "attribute 'invalid_field' does not reference a field"
        );
    }

    #[test]
    fn test_sort_multiple_types() {
        let v = to_value(vec![Value::Number(12.into()), Value::Array(vec![])]).unwrap();
        let args = HashMap::new();

        let result = sort(&v, &args);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "expected number got []");
    }

    #[test]
    fn test_sort_non_finite_numbers() {
        let v = to_value(vec![
            ::std::f64::NEG_INFINITY, // NaN and friends get deserialized as Null by serde.
            ::std::f64::NAN,
        ])
        .unwrap();
        let args = HashMap::new();

        let result = sort(&v, &args);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Null is not a sortable value");
    }

    #[derive(Deserialize, Eq, Hash, PartialEq, Serialize)]
    struct TupleStruct(i32, i32);

    #[test]
    fn test_sort_tuple() {
        let v = to_value(vec![
            TupleStruct(0, 1),
            TupleStruct(7, 0),
            TupleStruct(-1, 12),
            TupleStruct(18, 18),
        ])
        .unwrap();
        let mut args = HashMap::new();
        args.insert("attribute".to_string(), to_value("0").unwrap());

        let result = sort(&v, &args);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            to_value(vec![
                TupleStruct(-1, 12),
                TupleStruct(0, 1),
                TupleStruct(7, 0),
                TupleStruct(18, 18),
            ])
            .unwrap()
        );
    }

    #[test]
    fn test_unique_numbers() {
        let v = to_value(vec![3, -1, 3, 3, 5, 2, 5, 4]).unwrap();
        let args = HashMap::new();
        let result = unique(&v, &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(vec![3, -1, 5, 2, 4]).unwrap());
    }

    #[test]
    fn test_unique_strings() {
        let v = to_value(vec!["One", "Two", "Three", "one", "Two"]).unwrap();
        let mut args = HashMap::new();
        let result = unique(&v, &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(vec!["One", "Two", "Three"]).unwrap());

        args.insert("case_sensitive".to_string(), to_value(true).unwrap());
        let result = unique(&v, &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(vec!["One", "Two", "Three", "one"]).unwrap());
    }

    #[test]
    fn test_unique_empty() {
        let v = to_value(Vec::<f64>::new()).unwrap();
        let args = HashMap::new();
        let result = sort(&v, &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(Vec::<f64>::new()).unwrap());
    }

    #[test]
    fn test_unique_attribute() {
        let v = to_value(vec![
            Foo { a: 1, b: 2 },
            Foo { a: 3, b: 3 },
            Foo { a: 1, b: 3 },
            Foo { a: 0, b: 4 },
        ])
        .unwrap();
        let mut args = HashMap::new();
        args.insert("attribute".to_string(), to_value("a").unwrap());

        let result = unique(&v, &args);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            to_value(vec![Foo { a: 1, b: 2 }, Foo { a: 3, b: 3 }, Foo { a: 0, b: 4 },]).unwrap()
        );
    }

    #[test]
    fn test_unique_invalid_attribute() {
        let v = to_value(vec![Foo { a: 3, b: 5 }]).unwrap();
        let mut args = HashMap::new();
        args.insert("attribute".to_string(), to_value("invalid_field").unwrap());

        let result = unique(&v, &args);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "attribute 'invalid_field' does not reference a field"
        );
    }

    #[test]
    fn test_unique_multiple_types() {
        let v = to_value(vec![Value::Number(12.into()), Value::Array(vec![])]).unwrap();
        let args = HashMap::new();

        let result = unique(&v, &args);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "unique filter can't compare multiple types");
    }

    #[test]
    fn test_unique_non_finite_numbers() {
        let v = to_value(vec![
            ::std::f64::NEG_INFINITY, // NaN and friends get deserialized as Null by serde.
            ::std::f64::NAN,
        ])
        .unwrap();
        let args = HashMap::new();

        let result = unique(&v, &args);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Null is not a unique value");
    }

    #[test]
    fn test_unique_tuple() {
        let v = to_value(vec![
            TupleStruct(0, 1),
            TupleStruct(-7, -1),
            TupleStruct(-1, 1),
            TupleStruct(18, 18),
        ])
        .unwrap();
        let mut args = HashMap::new();
        args.insert("attribute".to_string(), to_value("1").unwrap());

        let result = unique(&v, &args);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            to_value(vec![TupleStruct(0, 1), TupleStruct(-7, -1), TupleStruct(18, 18),]).unwrap()
        );
    }

    #[test]
    fn test_slice() {
        fn make_args(start: Option<usize>, end: Option<f64>) -> HashMap<String, Value> {
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
            (make_args(None, Some(2.0)), vec![1, 2]),
            (make_args(Some(1), Some(2.0)), vec![2]),
            (make_args(None, Some(-2.0)), vec![1, 2, 3]),
            (make_args(None, None), vec![1, 2, 3, 4, 5]),
            (make_args(Some(3), Some(1.0)), vec![]),
            (make_args(Some(9), None), vec![]),
        ];

        for (args, expected) in inputs {
            let res = slice(&v, &args);
            assert!(res.is_ok());
            assert_eq!(res.unwrap(), to_value(expected).unwrap());
        }
    }

    #[test]
    fn test_group_by() {
        let input = json!([
            {"id": 1, "year": 2015},
            {"id": 2, "year": 2015},
            {"id": 3, "year": 2016},
            {"id": 4, "year": 2017},
            {"id": 5, "year": 2017},
            {"id": 6, "year": 2017},
            {"id": 7, "year": 2018},
            {"id": 8},
            {"id": 9, "year": null},
        ]);
        let mut args = HashMap::new();
        args.insert("attribute".to_string(), to_value("year").unwrap());

        let expected = json!({
            "2015": [{"id": 1, "year": 2015}, {"id": 2, "year": 2015}],
            "2016": [{"id": 3, "year": 2016}],
            "2017": [{"id": 4, "year": 2017}, {"id": 5, "year": 2017}, {"id": 6, "year": 2017}],
            "2018": [{"id": 7, "year": 2018}],
        });

        let res = group_by(&input, &args);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), to_value(expected).unwrap());
    }

    #[test]
    fn test_group_by_nested_key() {
        let input = json!([
            {"id": 1, "company": {"id": 1}},
            {"id": 2, "company": {"id": 2}},
            {"id": 3, "company": {"id": 3}},
            {"id": 4, "company": {"id": 4}},
            {"id": 5, "company": {"id": 4}},
            {"id": 6, "company": {"id": 5}},
            {"id": 7, "company": {"id": 5}},
            {"id": 8},
            {"id": 9, "company": null},
        ]);
        let mut args = HashMap::new();
        args.insert("attribute".to_string(), to_value("company.id").unwrap());

        let expected = json!({
            "1": [{"id": 1, "company": {"id": 1}}],
            "2": [{"id": 2, "company": {"id": 2}}],
            "3": [{"id": 3, "company": {"id": 3}}],
            "4": [{"id": 4, "company": {"id": 4}}, {"id": 5, "company": {"id": 4}}],
            "5": [{"id": 6, "company": {"id": 5}}, {"id": 7, "company": {"id": 5}}],
        });

        let res = group_by(&input, &args);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), to_value(expected).unwrap());
    }

    #[test]
    fn test_filter_empty() {
        let res = filter(&json!([]), &HashMap::new());
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), json!([]));
    }

    #[test]
    fn test_filter() {
        let input = json!([
            {"id": 1, "year": 2015},
            {"id": 2, "year": 2015},
            {"id": 3, "year": 2016},
            {"id": 4, "year": 2017},
            {"id": 5, "year": 2017},
            {"id": 6, "year": 2017},
            {"id": 7, "year": 2018},
            {"id": 8},
            {"id": 9, "year": null},
        ]);
        let mut args = HashMap::new();
        args.insert("attribute".to_string(), to_value("year").unwrap());
        args.insert("value".to_string(), to_value(2015).unwrap());

        let expected = json!([
            {"id": 1, "year": 2015},
            {"id": 2, "year": 2015},
        ]);

        let res = filter(&input, &args);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), to_value(expected).unwrap());
    }

    #[test]
    fn test_filter_no_value() {
        let input = json!([
            {"id": 1, "year": 2015},
            {"id": 2, "year": 2015},
            {"id": 3, "year": 2016},
            {"id": 4, "year": 2017},
            {"id": 5, "year": 2017},
            {"id": 6, "year": 2017},
            {"id": 7, "year": 2018},
            {"id": 8},
            {"id": 9, "year": null},
        ]);
        let mut args = HashMap::new();
        args.insert("attribute".to_string(), to_value("year").unwrap());

        let expected = json!([
            {"id": 1, "year": 2015},
            {"id": 2, "year": 2015},
            {"id": 3, "year": 2016},
            {"id": 4, "year": 2017},
            {"id": 5, "year": 2017},
            {"id": 6, "year": 2017},
            {"id": 7, "year": 2018},
        ]);

        let res = filter(&input, &args);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), to_value(expected).unwrap());
    }

    #[test]
    fn test_map_empty() {
        let res = map(&json!([]), &HashMap::new());
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), json!([]));
    }

    #[test]
    fn test_map() {
        let input = json!([
            {"id": 1, "year": 2015},
            {"id": 2, "year": true},
            {"id": 3, "year": 2016.5},
            {"id": 4, "year": "2017"},
            {"id": 5, "year": 2017},
            {"id": 6, "year": 2017},
            {"id": 7, "year": [1900, 1901]},
            {"id": 8, "year": {"a": 2018, "b": 2019}},
            {"id": 9},
            {"id": 10, "year": null},
        ]);
        let mut args = HashMap::new();
        args.insert("attribute".to_string(), to_value("year").unwrap());

        let expected =
            json!([2015, true, 2016.5, "2017", 2017, 2017, [1900, 1901], {"a": 2018, "b": 2019}]);

        let res = map(&input, &args);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), to_value(expected).unwrap());
    }

    #[test]
    fn test_concat_array() {
        let input = json!([1, 2, 3,]);
        let mut args = HashMap::new();
        args.insert("with".to_string(), json!([3, 4]));
        let expected = json!([1, 2, 3, 3, 4,]);

        let res = concat(&input, &args);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), to_value(expected).unwrap());
    }

    #[test]
    fn test_concat_single_value() {
        let input = json!([1, 2, 3,]);
        let mut args = HashMap::new();
        args.insert("with".to_string(), json!(4));
        let expected = json!([1, 2, 3, 4,]);

        let res = concat(&input, &args);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), to_value(expected).unwrap());
    }
}
