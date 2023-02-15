use crate::errors::{Error, Result};
use serde_json::Value;
use std::cmp::Ordering;

#[derive(PartialEq, Default, Copy, Clone)]
pub struct OrderedF64(f64);

impl OrderedF64 {
    fn new(n: f64) -> Self {
        OrderedF64(n)
    }
}

impl Eq for OrderedF64 {}

impl Ord for OrderedF64 {
    fn cmp(&self, other: &OrderedF64) -> Ordering {
        // unwrap is safe because self.0 is finite.
        self.partial_cmp(other).unwrap()
    }
}

impl PartialOrd for OrderedF64 {
    fn partial_cmp(&self, other: &OrderedF64) -> Option<Ordering> {
        Some(total_cmp(&self.0, &other.0))
    }
}

/// Return the ordering between `self` and `other` f64.
///
/// https://doc.rust-lang.org/std/primitive.f64.html#method.total_cmp
///
/// Backported from Rust 1.62 to keep MSRV at 1.56
///
/// Unlike the standard partial comparison between floating point numbers,
/// this comparison always produces an ordering in accordance to
/// the `totalOrder` predicate as defined in the IEEE 754 (2008 revision)
/// floating point standard. The values are ordered in the following sequence:
///
/// - negative quiet NaN
/// - negative signaling NaN
/// - negative infinity
/// - negative numbers
/// - negative subnormal numbers
/// - negative zero
/// - positive zero
/// - positive subnormal numbers
/// - positive numbers
/// - positive infinity
/// - positive signaling NaN
/// - positive quiet NaN.
///
/// The ordering established by this function does not always agree with the
/// [`PartialOrd`] and [`PartialEq`] implementations of `f64`. For example,
/// they consider negative and positive zero equal, while `total_cmp`
/// doesn't.
///
/// The interpretation of the signaling NaN bit follows the definition in
/// the IEEE 754 standard, which may not match the interpretation by some of
/// the older, non-conformant (e.g. MIPS) hardware implementations.
///
#[must_use]
#[inline]
fn total_cmp(a: &f64, b: &f64) -> Ordering {
    let mut left = a.to_bits() as i64;
    let mut right = b.to_bits() as i64;
    left ^= (((left >> 63) as u64) >> 1) as i64;
    right ^= (((right >> 63) as u64) >> 1) as i64;

    left.cmp(&right)
}

#[derive(Default, Eq, PartialEq, Ord, PartialOrd, Copy, Clone)]
pub struct ArrayLen(usize);

pub trait GetValue: Ord + Sized + Clone {
    fn get_value(val: &Value) -> Result<Self>;
}

impl GetValue for OrderedF64 {
    fn get_value(val: &Value) -> Result<Self> {
        let n = val.as_f64().ok_or_else(|| Error::msg(format!("expected number got {}", val)))?;
        Ok(OrderedF64::new(n))
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
pub struct SortPairs<K: Ord> {
    pairs: Vec<(Value, K)>,
}

type SortNumbers = SortPairs<OrderedF64>;
type SortBools = SortPairs<bool>;
type SortStrings = SortPairs<String>;
type SortArrays = SortPairs<ArrayLen>;

impl<K: GetValue> SortPairs<K> {
    fn try_add_pair(&mut self, val: &Value, key: &Value) -> Result<()> {
        let key = K::get_value(key)?;
        self.pairs.push((val.clone(), key));
        Ok(())
    }

    fn sort(&mut self) -> Vec<Value> {
        self.pairs.sort_by_key(|a| a.1.clone());
        self.pairs.iter().map(|a| a.0.clone()).collect()
    }
}

pub trait SortStrategy {
    fn try_add_pair(&mut self, val: &Value, key: &Value) -> Result<()>;
    fn sort(&mut self) -> Vec<Value>;
}

impl<K: GetValue> SortStrategy for SortPairs<K> {
    fn try_add_pair(&mut self, val: &Value, key: &Value) -> Result<()> {
        SortPairs::try_add_pair(self, val, key)
    }

    fn sort(&mut self) -> Vec<Value> {
        SortPairs::sort(self)
    }
}

pub fn get_sort_strategy_for_type(ty: &Value) -> Result<Box<dyn SortStrategy>> {
    use crate::Value::*;
    match *ty {
        Null => Err(Error::msg("Null is not a sortable value")),
        Bool(_) => Ok(Box::<SortBools>::default()),
        Number(_) => Ok(Box::<SortNumbers>::default()),
        String(_) => Ok(Box::<SortStrings>::default()),
        Array(_) => Ok(Box::<SortArrays>::default()),
        Object(_) => Err(Error::msg("Object is not a sortable value")),
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
        Bool(_) => Ok(Box::<UniqueBools>::default()),
        Number(ref val) => {
            if val.is_f64() {
                Err(Error::msg("Unique floats are not implemented"))
            } else {
                Ok(Box::<UniqueNumbers>::default())
            }
        }
        String(_) => Ok(Box::new(UniqueStrings::new(case_sensitive))),
        Array(_) => Err(Error::msg("Unique arrays are not implemented")),
        Object(_) => Err(Error::msg("Unique objects are not implemented")),
    }
}
