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
pub fn sort(value: Value, args: HashMap<String, Value>) -> Result<Value> {
    let mut arr = try_get_value!("sort", "value", Vec<Value>, value);
    if arr.is_empty() {
        return Ok(arr.into());
    }

    let attribute = try_get_value!("sort", "attribute", String, args.get("attribute").unwrap_or(&"".into()));
    let ptr = match attribute.as_str() {
        "" => "".to_string(),
        s => get_json_pointer(s)
    };

    let first = arr[0].pointer(&ptr).ok_or(format!("attribute '{}' does not reference a field", attribute))?;
    let mut strategy = get_sort_strategy_for_type(first)?;
    for v in &arr {
        let key = v.pointer(&ptr).ok_or(format!("attribute '{}' does not reference a field", attribute))?;
        strategy.try_add_pair(v, key)?;
    }
    let sorted = strategy.sort();

    Ok(sorted.into())
}

#[derive(PartialEq, PartialOrd, Default, Copy, Clone)]
struct OrderedF64(f64);

impl OrderedF64 {
    fn new(n: f64) -> Result<Self> {
        if n.is_finite() {
            Ok(OrderedF64(n))
        } else {
            bail!("{} cannot be sorted", n)
        }
    }
}

impl Eq for OrderedF64 {}

impl Ord for OrderedF64 {
    fn cmp(&self, other: &OrderedF64) -> Ordering {
        // unwrap is safe because self.0 is finite.
        self.partial_cmp(other).unwrap()
    }
}

#[derive(Default, Eq, PartialEq, Ord, PartialOrd, Copy, Clone)]
struct ArrayLen(usize);

trait GetSortKey: Ord + Sized + Clone {
    fn get_sort_key(val: &Value) -> Result<Self>;
}

impl GetSortKey for OrderedF64 {
    fn get_sort_key(val: &Value) -> Result<Self> {
        let n = val.as_f64().ok_or(format!("expected number got {}", val))?;
        OrderedF64::new(n)
    }
}

impl GetSortKey for bool {
    fn get_sort_key(val: &Value) -> Result<Self> {
        val.as_bool().ok_or(format!("expected bool got {}", val).into())
    }
}

impl GetSortKey for String {
    fn get_sort_key(val: &Value) -> Result<Self> {
        let str: Result<&str> = val.as_str().ok_or(format!("expected string got {}", val).into());
        Ok(str?.to_owned())
    }
}

impl GetSortKey for ArrayLen {
    fn get_sort_key(val: &Value) -> Result<Self> {
        let arr = val.as_array().ok_or(format!("expected array got {}", val))?;
        Ok(ArrayLen(arr.len()))
    }
}

#[derive(Default)]
struct SortPairs<K: Ord> {
    pairs: Vec<(Value, K)>
}

type Numbers = SortPairs<OrderedF64>;
type Bools = SortPairs<bool>;
type Strings = SortPairs<String>;
type Arrays = SortPairs<ArrayLen>;

impl<K: GetSortKey> SortPairs<K> {
    fn try_add_pair(&mut self, val: &Value, key: &Value) -> Result<()> {
        let key = K::get_sort_key(key)?;
        self.pairs.push((val.clone(), key));
        Ok(())
    }

    fn sort(&mut self) -> Vec<Value> {
        self.pairs.sort_by_key(|a| a.1.clone());
        self.pairs.iter()
            .map(|a| a.0.clone())
            .collect()
    }
}

trait SortStrategy {
    fn try_add_pair(&mut self, val: &Value, key: &Value) -> Result<()>;
    fn sort(&mut self) -> Vec<Value>;
}

impl<K: GetSortKey> SortStrategy for SortPairs<K> {
    fn try_add_pair(&mut self, val: &Value, key: &Value) -> Result<()> {
        SortPairs::try_add_pair(self, val, key)
    }

    fn sort(&mut self) -> Vec<Value> {
        SortPairs::sort(self)
    }
}

fn get_sort_strategy_for_type(ty: &Value) -> Result<Box<SortStrategy>> {
    use Value::*;
    match *ty {
        Null => bail!("Null is not a sortable value"),
        Bool(_) => Ok(Box::new(Bools::default())),
        Number(_) => Ok(Box::new(Numbers::default())),
        String(_) => Ok(Box::new(Strings::default())),
        Array(_) => Ok(Box::new(Arrays::default())),
        Object(_) => bail!("Object is not a sortable value")
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

    #[test]
    fn test_sort_invalid_attribute() {
        let v = to_value(vec![
            Foo {a: 3, b: 5}
        ]).unwrap();
        let mut args = HashMap::new();
        args.insert("attribute".to_string(), to_value(&"invalid_field").unwrap());

        let result = sort(v, args);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().description(), "attribute 'invalid_field' does not reference a field");
    }

    #[test]
    fn test_sort_multiple_types() {
        let v = to_value(vec![
            Value::Number(12.into()),
            Value::Array(vec![])
        ]).unwrap();
        let args = HashMap::new();

        let result = sort(v, args);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().description(), "expected number got []");
    }

    #[test]
    fn test_sort_non_finite_numbers() {
        let v = to_value(vec![
            ::std::f64::NEG_INFINITY, // NaN and friends get deserialized as Null by serde.
            ::std::f64::NAN
        ]).unwrap();
        let args = HashMap::new();

        let result = sort(v, args);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().description(), "Null is not a sortable value");
    }

    #[derive(Serialize)]
    struct TupleStruct(i32, i32);

    #[test]
    fn test_sort_tuple() {
        let v = to_value(vec![
            TupleStruct(0, 1),
            TupleStruct(7, 0),
            TupleStruct(-1, 12),
            TupleStruct(18, 18)
        ]).unwrap();
        let mut args = HashMap::new();
        args.insert("attribute".to_string(), to_value("0").unwrap());

        let result = sort(v, args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(vec![
            TupleStruct(-1, 12),
            TupleStruct(0, 1),
            TupleStruct(7, 0),
            TupleStruct(18, 18)
        ]).unwrap());
    }
}
