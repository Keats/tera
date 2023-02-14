use crate::errors::{Error, Result};
use serde_json::Value;
use std::cmp::Ordering;

pub fn compare_values(a: &Value, b: &Value) -> Result<Ordering> {
    if std::mem::discriminant(a) != std::mem::discriminant(b) {
        return Err(Error::msg(format!("expected same types but got {a} and {b}")));
    }
    match a {
        Value::Null => Ok(Ordering::Equal),
        Value::Bool(a) => Ok(a.cmp(&b.as_bool().unwrap())),
        Value::Number(a) => {
            if let Some(a) = a.as_f64() {
                if let Some(b) = b.as_f64() {
                    return Ok(a.total_cmp(&b));
                }
            }
            if let Some(a) = a.as_i64() {
                if let Some(b) = b.as_i64() {
                    return Ok(a.cmp(&b));
                }
            }
            if let Some(a) = a.as_u64() {
                if let Some(b) = b.as_u64() {
                    return Ok(a.cmp(&b));
                }
            }
            Err(Error::msg(format!("{a} cannot be sorted")))
        }
        Value::String(a) => Ok(a.as_str().cmp(b.as_str().unwrap())),
        Value::Array(a) => Ok(a.len().cmp(&b.as_array().unwrap().len())),
        Value::Object(a) => Ok(a.len().cmp(&b.as_array().unwrap().len())),
    }
}

#[derive(PartialEq, PartialOrd, Default, Copy, Clone)]
pub struct OrderedF64(f64);

impl OrderedF64 {
    fn new(n: f64) -> Result<Self> {
        if n.is_finite() {
            Ok(OrderedF64(n))
        } else {
            Err(Error::msg(format!("{} cannot be sorted", n)))
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
pub struct ArrayLen(usize);

pub trait GetValue: Ord + Sized + Clone {
    fn get_value(val: &Value) -> Result<Self>;
}

impl GetValue for OrderedF64 {
    fn get_value(val: &Value) -> Result<Self> {
        let n = val.as_f64().ok_or_else(|| Error::msg(format!("expected number got {}", val)))?;
        OrderedF64::new(n)
    }
}

impl GetValue for i64 {
    fn get_value(val: &Value) -> Result<Self> {
        val.as_i64().ok_or_else(|| Error::msg(format!("expected number got {}", val)))
    }
}

impl GetValue for bool {
    fn get_value(val: &Value) -> Result<Self> {
        val.as_bool().ok_or_else(|| Error::msg(format!("expected bool got {}", val)))
    }
}

impl GetValue for String {
    fn get_value(val: &Value) -> Result<Self> {
        let str: Result<&str> =
            val.as_str().ok_or_else(|| Error::msg(format!("expected string got {}", val)));
        Ok(str?.to_owned())
    }
}

impl GetValue for ArrayLen {
    fn get_value(val: &Value) -> Result<Self> {
        let arr =
            val.as_array().ok_or_else(|| Error::msg(format!("expected array got {}", val)))?;
        Ok(ArrayLen(arr.len()))
    }
}

#[derive(Default)]
pub struct Unique<K: Eq + std::hash::Hash> {
    unique: std::collections::HashSet<K>,
}

type UniqueNumbers = Unique<i64>;
type UniqueBools = Unique<bool>;
struct UniqueStrings {
    u: Unique<String>,
    case_sensitive: bool,
}

pub trait UniqueStrategy {
    fn insert(&mut self, val: &Value) -> Result<bool>;
}

impl<K: GetValue + Eq + std::hash::Hash> UniqueStrategy for Unique<K> {
    fn insert(&mut self, val: &Value) -> Result<bool> {
        Ok(self.unique.insert(K::get_value(val)?))
    }
}

impl UniqueStrings {
    fn new(case_sensitive: bool) -> UniqueStrings {
        UniqueStrings { u: Unique::<String>::default(), case_sensitive }
    }
}

impl UniqueStrategy for UniqueStrings {
    fn insert(&mut self, val: &Value) -> Result<bool> {
        let mut key = String::get_value(val)?;
        if !self.case_sensitive {
            key = key.to_lowercase()
        }
        Ok(self.u.unique.insert(key))
    }
}

pub fn get_unique_strategy_for_type(
    ty: &Value,
    case_sensitive: bool,
) -> Result<Box<dyn UniqueStrategy>> {
    use crate::Value::*;
    match *ty {
        Null => Err(Error::msg("Null is not a unique value")),
        Bool(_) => Ok(Box::new(UniqueBools::default())),
        Number(ref val) => {
            if val.is_f64() {
                Err(Error::msg("Unique floats are not implemented"))
            } else {
                Ok(Box::new(UniqueNumbers::default()))
            }
        }
        String(_) => Ok(Box::new(UniqueStrings::new(case_sensitive))),
        Array(_) => Err(Error::msg("Unique arrays are not implemented")),
        Object(_) => Err(Error::msg("Unique objects are not implemented")),
    }
}
