use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt;
use std::fmt::Formatter;
use std::hash::Hash;
use std::sync::Arc;

use serde::ser::{Serialize, SerializeMap, SerializeSeq, Serializer};

#[cfg(feature = "unicode")]
use unicode_segmentation::UnicodeSegmentation;

mod de;
mod key;
pub(crate) mod number;
mod ser;
mod utils;

use crate::HashMap;
use crate::errors::{Error, TeraResult};
use crate::value::number::Number;
pub use key::Key;

/// The internal HashMap type used by Tera.
#[cfg(not(feature = "preserve_order"))]
pub type Map = HashMap<Key<'static>, Value>;

/// The internal HashMap type used by Tera.
#[cfg(feature = "preserve_order")]
pub type Map = indexmap::IndexMap<Key<'static>, Value>;

#[inline]
pub(crate) fn format_map(map: &Map, f: &mut impl std::io::Write) -> std::io::Result<()> {
    let mut key_val: Box<_> = map.iter().collect();
    // Keys are sorted to have deterministic output if preserve_order is not used
    if cfg!(not(feature = "preserve_order")) {
        key_val.sort_by_key(|elem| elem.0);
    }
    f.write_all(b"{")?;
    for (idx, (key, value)) in key_val.iter().enumerate() {
        if idx > 0 {
            f.write_all(b", ")?;
        }
        if let Some(v) = key.as_str() {
            write!(f, "{v:?}")?
        } else {
            key.format(f)?;
        }

        f.write_all(b": ")?;
        match &value.inner {
            ValueInner::String(smart_str) => write!(f, "{:?}", smart_str.as_str())?,
            _ => value.format(f)?,
        }
    }
    f.write_all(b"}")
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(crate) enum StringKind {
    Normal,
    Safe,
}

/// The kind of values Tera can handle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ValueKind {
    /// This is mostly used internally to represent a lookup failure and you're unlikely to need to
    /// use that except when writing some special filters/functions that lookups data in the state.
    Undefined,
    /// An explicit `None` value: it is there and found, but it is `None`.
    None,
    #[allow(missing_docs)]
    Bool,
    #[allow(missing_docs)]
    U64,
    #[allow(missing_docs)]
    I64,
    #[allow(missing_docs)]
    U128,
    #[allow(missing_docs)]
    I128,
    #[allow(missing_docs)]
    F64,
    #[allow(missing_docs)]
    String,
    #[allow(missing_docs)]
    Array,
    #[allow(missing_docs)]
    Map,
    /// If you try to display bytes values in a template (eg `{{ bytes }}`)
    /// it will try to render it as lossy UTF-8.
    /// There is no way to create a `Value::Bytes` from inside a template: it's mostly there
    /// for interaction with filters etc
    Bytes,
}

/// Smart string with embedded StringKind for memory efficiency.
/// Inline storage for strings ≤21 chars, Arc for longer strings.
#[derive(Clone)]
pub(crate) enum SmartString {
    Small {
        len: u8,
        kind: StringKind,
        data: [u8; 21],
    },
    Large(Arc<str>, StringKind),
}

impl SmartString {
    fn new(s: &str, kind: StringKind) -> Self {
        if s.len() <= 21 {
            let mut data = [0; 21];
            data[..s.len()].copy_from_slice(s.as_bytes());
            Self::Small {
                len: s.len() as u8,
                kind,
                data,
            }
        } else {
            Self::Large(Arc::from(s), kind)
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        match self {
            Self::Small { len, data, .. } => {
                // SAFETY: We know this is valid UTF-8 since we constructed it from a &str
                unsafe { std::str::from_utf8_unchecked(&data[..*len as usize]) }
            }
            Self::Large(s, _) => s,
        }
    }

    pub(crate) fn len(&self) -> usize {
        match self {
            Self::Small { len, .. } => *len as usize,
            Self::Large(s, _) => s.len(),
        }
    }

    pub(crate) fn kind(&self) -> StringKind {
        match self {
            Self::Small { kind, .. } => *kind,
            Self::Large(_, kind) => *kind,
        }
    }

    pub(crate) fn mark_safe(self) -> Self {
        match self {
            Self::Small { len, data, .. } => Self::Small {
                len,
                kind: StringKind::Safe,
                data,
            },
            Self::Large(s, _) => Self::Large(s, StringKind::Safe),
        }
    }

    /// Get string content as Arc<str>, cloning only for small strings
    pub(crate) fn into_arc_str(self) -> Arc<str> {
        match self {
            Self::Small { .. } => Arc::from(self.as_str()),
            Self::Large(arc, _) => arc,
        }
    }
}

impl fmt::Display for SmartString {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl fmt::Debug for SmartString {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "\"{}\"", self)
    }
}

// Internal implementation - can optimize freely
#[derive(Debug, Clone)]
pub(crate) enum ValueInner {
    Undefined,
    None,
    Bool(bool),
    U64(u64),
    I64(i64),
    F64(f64),
    // Box large integers since they are not used very often
    U128(Box<u128>),
    I128(Box<i128>),
    // SmartString includes whether a string is safe or not
    String(SmartString),
    Array(Arc<Vec<Value>>),
    Map(Arc<Map>),
    Bytes(Arc<Vec<u8>>),
}

/// The Value type that Tera uses internally and that will handle ser/de
#[derive(Clone)]
pub struct Value {
    pub(crate) inner: ValueInner,
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut out = Vec::new();
        let res = self.format(&mut out);
        if res.is_err() {
            return Err(fmt::Error);
        }
        write!(
            f,
            "{}",
            std::str::from_utf8(&out).expect("valid utf-8 in display")
        )
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.inner)
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (&self.inner, &other.inner) {
            // First the easy ones
            (ValueInner::Undefined, ValueInner::Undefined) => true,
            (ValueInner::None, ValueInner::None) => true,
            (ValueInner::Bool(v), ValueInner::Bool(v2)) => v == v2,
            (ValueInner::Array(v), ValueInner::Array(v2)) => v == v2,
            (ValueInner::Bytes(v), ValueInner::Bytes(v2)) => v == v2,
            // TODO: should string kind be used for partialeq? They might be equal now but
            // different later if one needs to be escape
            (ValueInner::String(v), ValueInner::String(v2)) => v.as_str() == v2.as_str(),
            (ValueInner::Map(v), ValueInner::Map(v2)) => v == v2,
            // Then the numbers
            (ValueInner::F64(a), ValueInner::F64(b)) => (a.is_nan() && b.is_nan()) || a == b,
            // First if there's a float we need to convert to float
            (ValueInner::F64(v), _) => Some(*v) == other.as_f64(),
            (_, ValueInner::F64(v)) => Some(*v) == self.as_f64(),
            (
                ValueInner::U64(_) | ValueInner::I64(_) | ValueInner::U128(_) | ValueInner::I128(_),
                ValueInner::U64(_) | ValueInner::I64(_) | ValueInner::U128(_) | ValueInner::I128(_),
            ) => match (self.as_u128(), other.as_u128()) {
                (Some(a), Some(b)) => a == b,
                (None, None) => self.as_i128() == other.as_i128(),
                _ => false,
            },
            (_, _) => false,
        }
    }
}

impl Eq for Value {}

#[allow(clippy::non_canonical_partial_ord_impl)]
impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (&self.inner, &other.inner) {
            // First the easy ones
            (ValueInner::Undefined, ValueInner::Undefined) => Some(Ordering::Equal),
            (ValueInner::None, ValueInner::None) => Some(Ordering::Equal),
            (ValueInner::Bool(v), ValueInner::Bool(v2)) => v.partial_cmp(v2),
            (ValueInner::Array(v), ValueInner::Array(v2)) => v.partial_cmp(v2),
            (ValueInner::Bytes(v), ValueInner::Bytes(v2)) => v.partial_cmp(v2),
            (ValueInner::String(v), ValueInner::String(v2)) => v.as_str().partial_cmp(v2.as_str()),
            // Then the numbers
            (ValueInner::F64(a), ValueInner::F64(b)) => Some(a.total_cmp(b)),
            // First if there's a float we need to convert to float
            (ValueInner::F64(v), _) => v.partial_cmp(&other.as_f64()?),
            (_, ValueInner::F64(v)) => self.as_f64()?.partial_cmp(v),
            (
                ValueInner::U64(_) | ValueInner::I64(_) | ValueInner::U128(_) | ValueInner::I128(_),
                ValueInner::U64(_) | ValueInner::I64(_) | ValueInner::U128(_) | ValueInner::I128(_),
            ) => match (self.as_u128(), other.as_u128()) {
                // both values are positive since they can be in a u128
                (Some(a), Some(b)) => Some(a.cmp(&b)),
                // one of them is negative
                (Some(_), None) => Some(Ordering::Greater),
                (None, Some(_)) => Some(Ordering::Less),
                // both are negative; `as_i128` is always `Some` for them today, but `?` keeps
                // us robust if the helpers ever drift (returning `None` falls to `type_order`).
                (None, None) => Some(self.as_i128()?.cmp(&other.as_i128()?)),
            },
            (_, _) => None,
        }
    }
}

impl Ord for Value {
    fn cmp(&self, other: &Self) -> Ordering {
        if let Some(res) = self.partial_cmp(other) {
            return res;
        }

        // Fallback: order by type for consistent ordering of incompatible types.
        // It's nonsensical but this way with the sort filter the None/undefined show up at the end
        fn type_order(v: &ValueInner) -> u8 {
            match v {
                ValueInner::Bool(_) => 0,
                ValueInner::U64(_)
                | ValueInner::I64(_)
                | ValueInner::F64(_)
                | ValueInner::U128(_)
                | ValueInner::I128(_) => 1,
                ValueInner::String(_) => 2,
                ValueInner::Array(_) => 3,
                ValueInner::Map(_) => 4,
                ValueInner::Bytes(_) => 5,
                ValueInner::None => 6,
                ValueInner::Undefined => 7,
            }
        }
        type_order(&self.inner).cmp(&type_order(&other.inner))
    }
}

/// Resolves an integer-typed `item` into a valid `0..len` index.
/// None if the index doesn't fit into a usize and errors for non-integer values
fn resolve_index(item: &Value, len: usize, kind: &str) -> TeraResult<Option<usize>> {
    if let Some(idx) = item.as_i128() {
        let normalized = if idx < 0 { idx + len as i128 } else { idx };
        Ok((0..len as i128)
            .contains(&normalized)
            .then_some(normalized as usize))
    } else if item.is_u128() {
        Ok(None)
    } else {
        Err(Error::message(format!(
            "{kind} index must be an integer, got `{}`.",
            item.name(),
        )))
    }
}

impl Value {
    #[allow(missing_docs)]
    pub fn none() -> Self {
        Value {
            inner: ValueInner::None,
        }
    }

    #[allow(missing_docs)]
    pub fn undefined() -> Self {
        Value {
            inner: ValueInner::Undefined,
        }
    }

    #[allow(missing_docs)]
    pub fn kind(&self) -> ValueKind {
        match &self.inner {
            ValueInner::Undefined => ValueKind::Undefined,
            ValueInner::None => ValueKind::None,
            ValueInner::Bool(_) => ValueKind::Bool,
            ValueInner::U64(_) => ValueKind::U64,
            ValueInner::I64(_) => ValueKind::I64,
            ValueInner::F64(_) => ValueKind::F64,
            ValueInner::U128(_) => ValueKind::U128,
            ValueInner::I128(_) => ValueKind::I128,
            ValueInner::String(_) => ValueKind::String,
            ValueInner::Array(_) => ValueKind::Array,
            ValueInner::Map(_) => ValueKind::Map,
            ValueInner::Bytes(_) => ValueKind::Bytes,
        }
    }

    // Type checks
    #[allow(missing_docs)]
    pub fn is_undefined(&self) -> bool {
        matches!(self.kind(), ValueKind::Undefined)
    }
    #[allow(missing_docs)]
    pub fn is_none(&self) -> bool {
        matches!(self.kind(), ValueKind::None)
    }
    #[allow(missing_docs)]
    pub fn is_bool(&self) -> bool {
        matches!(self.kind(), ValueKind::Bool)
    }
    #[allow(missing_docs)]
    pub fn is_string(&self) -> bool {
        matches!(self.kind(), ValueKind::String)
    }
    #[allow(missing_docs)]
    pub fn is_i128(&self) -> bool {
        matches!(self.kind(), ValueKind::I128)
    }
    #[allow(missing_docs)]
    pub fn is_u128(&self) -> bool {
        matches!(self.kind(), ValueKind::U128)
    }
    #[allow(missing_docs)]
    pub fn is_i64(&self) -> bool {
        matches!(self.kind(), ValueKind::I64)
    }
    #[allow(missing_docs)]
    pub fn is_u64(&self) -> bool {
        matches!(self.kind(), ValueKind::U64)
    }
    #[allow(missing_docs)]
    pub fn is_f64(&self) -> bool {
        matches!(self.kind(), ValueKind::F64)
    }
    #[allow(missing_docs)]
    pub fn is_array(&self) -> bool {
        matches!(self.kind(), ValueKind::Array)
    }
    #[allow(missing_docs)]
    pub fn is_map(&self) -> bool {
        matches!(self.kind(), ValueKind::Map)
    }
    #[allow(missing_docs)]
    pub fn is_bytes(&self) -> bool {
        matches!(self.kind(), ValueKind::Bytes)
    }

    pub(crate) fn format(&self, f: &mut impl std::io::Write) -> std::io::Result<()> {
        match &self.inner {
            ValueInner::None | ValueInner::Undefined => Ok(()),
            ValueInner::Bool(v) => f.write_all(if *v { b"true" } else { b"false" }),
            ValueInner::Bytes(v) => f.write_all(String::from_utf8_lossy(v).as_bytes()),
            ValueInner::String(v) => f.write_all(v.as_str().as_bytes()),
            ValueInner::Array(v) => {
                f.write_all(b"[")?;

                for (idx, elem) in v.iter().enumerate() {
                    if idx > 0 {
                        f.write_all(b", ")?;
                    }

                    match &elem.inner {
                        ValueInner::String(v) => write!(f, "{:?}", v.as_str())?,
                        _ => elem.format(f)?,
                    }
                }
                f.write_all(b"]")
            }
            ValueInner::Map(v) => format_map(v, f),
            ValueInner::F64(v) => {
                // We could use ryu to print floats but it doesn't match the output from
                // the std so tests become annoying.
                write!(f, "{v}")
            }
            ValueInner::U64(v) => {
                #[cfg(feature = "no_fmt")]
                {
                    let mut buf = itoa::Buffer::new();
                    f.write_all(buf.format(*v).as_bytes())
                }
                #[cfg(not(feature = "no_fmt"))]
                write!(f, "{v}")
            }
            ValueInner::I64(v) => {
                #[cfg(feature = "no_fmt")]
                {
                    let mut buf = itoa::Buffer::new();
                    f.write_all(buf.format(*v).as_bytes())
                }
                #[cfg(not(feature = "no_fmt"))]
                write!(f, "{v}")
            }
            ValueInner::U128(v) => {
                #[cfg(feature = "no_fmt")]
                {
                    let mut buf = itoa::Buffer::new();
                    f.write_all(buf.format(**v).as_bytes())
                }
                #[cfg(not(feature = "no_fmt"))]
                write!(f, "{}", **v)
            }
            ValueInner::I128(v) => {
                #[cfg(feature = "no_fmt")]
                {
                    let mut buf = itoa::Buffer::new();
                    f.write_all(buf.format(**v).as_bytes())
                }
                #[cfg(not(feature = "no_fmt"))]
                write!(f, "{}", **v)
            }
        }
    }

    /// Creates a Value from something that impl Serialize.
    /// Panics if serialization fails; see [`Self::try_from_serializable`] for the fallible variant.
    pub fn from_serializable<T: Serialize + ?Sized>(value: &T) -> Value {
        Self::try_from_serializable(value).unwrap()
    }

    /// Fallible way to create a Value from something that impl Serialize
    pub fn try_from_serializable<T: Serialize + ?Sized>(value: &T) -> TeraResult<Value> {
        Serialize::serialize(value, ser::ValueSerializer)
            .map_err(|err| Error::message(err.to_string()))
    }

    /// Creates a normal string that will be escaped
    pub fn normal_string(val: &str) -> Value {
        Value {
            inner: ValueInner::String(SmartString::new(val, StringKind::Normal)),
        }
    }

    /// Creates a safe string that won't be escaped
    pub fn safe_string(val: &str) -> Value {
        Value {
            inner: ValueInner::String(SmartString::new(val, StringKind::Safe)),
        }
    }

    /// If the Value is an integer that can fit in a i128, return that otherwise None.
    pub fn as_i128(&self) -> Option<i128> {
        match &self.inner {
            ValueInner::U64(v) => Some(*v as i128),
            ValueInner::I64(v) => Some(*v as i128),
            ValueInner::U128(v) => i128::try_from(**v).ok(),
            ValueInner::I128(v) => Some(**v),
            _ => None,
        }
    }

    /// If the Value is an integer that can fit in a u128, return that otherwise None.
    pub fn as_u128(&self) -> Option<u128> {
        match &self.inner {
            ValueInner::U64(v) => Some(*v as u128),
            ValueInner::I64(v) => u128::try_from(*v).ok(),
            ValueInner::U128(v) => Some(**v),
            ValueInner::I128(v) => u128::try_from(**v).ok(),
            _ => None,
        }
    }

    /// If the Value is a f64 return the associated f64, otherwise None.
    pub fn as_f64(&self) -> Option<f64> {
        const MAX: u128 = 1u128 << f64::MANTISSA_DIGITS;
        match &self.inner {
            ValueInner::F64(v) => Some(*v),
            ValueInner::U64(v) => (*v as u128 <= MAX).then_some(*v as f64),
            ValueInner::I64(v) => (v.unsigned_abs() as u128 <= MAX).then_some(*v as f64),
            ValueInner::U128(v) => (**v <= MAX).then_some(**v as f64),
            ValueInner::I128(v) => (v.unsigned_abs() <= MAX).then_some(**v as f64),
            _ => None,
        }
    }

    /// Returns `None` for non-numeric values **and** for `u128` values exceeding `i128::MAX`,
    /// since `Number::Integer` only carries `i128`. i128 ought to be enough for everyone.
    pub fn as_number(&self) -> Option<Number> {
        match &self.inner {
            ValueInner::U64(v) => Some(Number::Integer(*v as i128)),
            ValueInner::I64(v) => Some(Number::Integer(*v as i128)),
            ValueInner::F64(v) => Some(Number::Float(*v)),
            ValueInner::U128(v) => i128::try_from(**v).ok().map(Number::Integer),
            ValueInner::I128(v) => Some(Number::Integer(**v)),
            _ => None,
        }
    }

    /// Returns `true` if the value is an integer or a float
    pub fn is_number(&self) -> bool {
        matches!(
            &self.inner,
            ValueInner::U64(..)
                | ValueInner::I64(..)
                | ValueInner::F64(..)
                | ValueInner::U128(..)
                | ValueInner::I128(..)
        )
    }

    /// If the Value is a string return the associated str, otherwise None.
    pub fn as_str(&self) -> Option<&str> {
        match &self.inner {
            ValueInner::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// If the Value is a bool return the associated bool, otherwise None.
    pub fn as_bool(&self) -> Option<bool> {
        match &self.inner {
            ValueInner::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Returns whether the value is safe (i.e. does not need escaping). Non-safe strings and
    /// arrays/maps/bytes return `false`; safe strings and all other scalar kinds return `true`.
    #[inline]
    pub fn is_safe(&self) -> bool {
        match &self.inner {
            ValueInner::String(s) => s.kind() == StringKind::Safe,
            ValueInner::Array(_) | ValueInner::Map(_) | ValueInner::Bytes(_) => false,
            _ => true,
        }
    }

    /// Consumes the Value, marking it as a safe string if it is one.
    #[inline]
    pub fn mark_safe(self) -> Self {
        match self.inner {
            ValueInner::String(s) => Value {
                inner: ValueInner::String(s.mark_safe()),
            },
            _ => self,
        }
    }

    /// If the Value is a map return the associated Map, otherwise None.
    pub fn as_map(&self) -> Option<&Map> {
        match &self.inner {
            ValueInner::Map(s) => Some(s),
            _ => None,
        }
    }

    /// If the Value is an array return the associated Vec, otherwise None.
    pub fn as_vec(&self) -> Option<&Vec<Value>> {
        match &self.inner {
            ValueInner::Array(s) => Some(s),
            _ => None,
        }
    }

    /// If the Value is a Bytes return the associated bytes, otherwise None.
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match &self.inner {
            ValueInner::Bytes(s) => Some(s),
            _ => None,
        }
    }

    /// Consumes the current Value to return its inner Map if it is one, otherwise None.
    pub fn into_map(self) -> Option<Map> {
        match self.inner {
            ValueInner::Map(arc) => Some(Arc::try_unwrap(arc).unwrap_or_else(|arc| (*arc).clone())),
            _ => None,
        }
    }

    pub(crate) fn into_vec(self) -> Option<Vec<Value>> {
        match self.inner {
            ValueInner::Array(arc) => {
                Some(Arc::try_unwrap(arc).unwrap_or_else(|arc| (*arc).clone()))
            }
            _ => None,
        }
    }

    /// Returns the Value at the given path, or Undefined if there's nothing there.
    pub fn get_from_path(&self, path: &str) -> Value {
        if matches!(&self.inner, ValueInner::Undefined | ValueInner::None) {
            return self.clone();
        }

        let mut current = self;

        for elem in path.split('.') {
            match elem.parse::<usize>() {
                Ok(idx) => match &current.inner {
                    ValueInner::Array(arr) => match arr.get(idx) {
                        Some(v) => current = v,
                        None => {
                            return Value {
                                inner: ValueInner::Undefined,
                            };
                        }
                    },
                    _ => {
                        return Value {
                            inner: ValueInner::Undefined,
                        };
                    }
                },
                Err(_) => match &current.inner {
                    ValueInner::Map(map) => match map.get(&Key::Str(elem)) {
                        Some(v) => current = v,
                        None => {
                            return Value {
                                inner: ValueInner::Undefined,
                            };
                        }
                    },
                    _ => {
                        return Value {
                            inner: ValueInner::Undefined,
                        };
                    }
                },
            }
        }

        current.clone()
    }

    /// Returns the truthiness of a value, eg not empty map/arrays/string and numbers different
    /// from 0
    pub fn is_truthy(&self) -> bool {
        match &self.inner {
            ValueInner::Undefined => false,
            ValueInner::None => false,
            ValueInner::Bool(v) => *v,
            ValueInner::U64(v) => *v != 0,
            ValueInner::I64(v) => *v != 0,
            ValueInner::F64(v) => *v != 0.0,
            ValueInner::U128(v) => **v != 0,
            ValueInner::I128(v) => **v != 0,
            ValueInner::Array(v) => !v.is_empty(),
            ValueInner::Bytes(v) => !v.is_empty(),
            ValueInner::String(v) => !v.as_str().is_empty(),
            ValueInner::Map(v) => !v.is_empty(),
        }
    }

    /// Returns the length of the value. Only works with maps, arrays, bytes and strings.
    /// For strings, if you use the `unicode` feature it will return the number of graphemes otherwise
    /// the number of chars.
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> Option<usize> {
        match &self.inner {
            ValueInner::Map(v) => Some(v.len()),
            ValueInner::Array(v) => Some(v.len()),
            ValueInner::Bytes(v) => Some(v.len()),
            ValueInner::String(v) => {
                #[cfg(feature = "unicode")]
                {
                    Some(v.as_str().graphemes(true).count())
                }
                #[cfg(not(feature = "unicode"))]
                {
                    Some(v.as_str().chars().count())
                }
            }
            _ => None,
        }
    }

    /// Reverses the content: only works for arrays, bytes and strings
    pub fn reverse(&self) -> TeraResult<Value> {
        match &self.inner {
            ValueInner::Array(v) => {
                let mut rev = (**v).clone();
                rev.reverse();
                Ok(Self::from(rev))
            }
            ValueInner::Bytes(v) => Ok(Self::from(v.iter().rev().copied().collect::<Vec<_>>())),
            ValueInner::String(v) => {
                #[cfg(feature = "unicode")]
                let reversed: String = v.as_str().graphemes(true).rev().collect();
                #[cfg(not(feature = "unicode"))]
                let reversed: String = v.as_str().chars().rev().collect();
                Ok(Self::from(reversed))
            }
            _ => Err(Error::message(format!(
                "Value of type {} cannot be reversed",
                self.name()
            ))),
        }
    }

    pub(crate) fn can_be_iterated_on(&self) -> bool {
        matches!(
            &self.inner,
            ValueInner::Map(..)
                | ValueInner::Array(..)
                | ValueInner::Bytes(..)
                | ValueInner::String(..)
        )
    }

    pub(crate) fn as_key(&self) -> TeraResult<Key<'static>> {
        let key = match &self.inner {
            ValueInner::Bool(v) => Key::Bool(*v),
            ValueInner::U64(v) => Key::U64(*v),
            ValueInner::I64(v) => Key::I64(*v),
            ValueInner::U128(v) => Key::U128(**v),
            ValueInner::I128(v) => Key::I128(**v),
            ValueInner::String(v) => Key::String(Arc::from(v.as_str())),
            _ => return Err(Error::message("Not a valid key type".to_string())),
        };
        Ok(key)
    }

    pub(crate) fn contains(&self, needle: &Value) -> TeraResult<bool> {
        match &self.inner {
            ValueInner::Array(arr) => Ok(arr.contains(needle)),
            ValueInner::String(s) => {
                if let Some(needle_str) = needle.as_str() {
                    Ok(s.as_str().contains(needle_str))
                } else {
                    Ok(false)
                }
            }
            // If they needle cannot index a map, then it can contain it
            ValueInner::Map(m) => match &needle.as_key() {
                Ok(k) => Ok(m.contains_key(k)),
                Err(_) => Ok(false),
            },
            _ => Err(Error::message(format!(
                "`in` cannot be used on a container of type `{}`. It can only be used on arrays, strings and map/structs",
                self.name()
            ))),
        }
    }

    /// When doing hello.name, name is the attr
    pub(crate) fn get_attr<'a>(&'a self, attr: &'a str) -> Option<&'a Value> {
        // We do either a linear scan or a hashmap lookup depending on the size of the map.
        // Linear scans can be _much_ faster for small maps
        #[cfg(not(feature = "preserve_order"))]
        const ATTR_SCAN_CUTOFF: usize = 6;
        #[cfg(feature = "preserve_order")]
        const ATTR_SCAN_CUTOFF: usize = 12;

        match &self.inner {
            ValueInner::Map(m) if m.len() <= ATTR_SCAN_CUTOFF => {
                m.iter().find_map(|(k, v)| match k.as_str() {
                    Some(s) if s == attr => Some(v),
                    _ => None,
                })
            }
            ValueInner::Map(m) => m.get(&Key::Str(attr)),
            _ => None,
        }
    }

    /// When doing hello[0], hello[name] etc, item is the value in the brackets
    pub(crate) fn get_item(&self, item: Value) -> TeraResult<Value> {
        match &self.inner {
            ValueInner::Map(m) => match item.as_key() {
                Ok(k) => Ok(m.get(&k).cloned().unwrap_or(Value {
                    inner: ValueInner::Undefined,
                })),
                Err(_) => Err(Error::message(format!(
                    "Map keys must be strings, integers, or bools, got `{}`.",
                    item.name()
                ))),
            },
            ValueInner::Array(arr) => Ok(match resolve_index(&item, arr.len(), "Array")? {
                Some(i) => arr[i].clone(),
                None => Value::undefined(),
            }),
            ValueInner::String(s) => {
                let kind = s.kind();
                #[cfg(feature = "unicode")]
                let chars: Vec<&str> = s.as_str().graphemes(true).collect();
                #[cfg(not(feature = "unicode"))]
                let chars: Vec<char> = s.as_str().chars().collect();
                Ok(match resolve_index(&item, chars.len(), "String")? {
                    Some(i) => {
                        let c = &chars[i];
                        #[cfg(feature = "unicode")]
                        let out = (*c).to_string();
                        #[cfg(not(feature = "unicode"))]
                        let out = c.to_string();
                        Value {
                            inner: ValueInner::String(SmartString::new(&out, kind)),
                        }
                    }
                    None => Value::undefined(),
                })
            }
            _ => Ok(Value {
                inner: ValueInner::Undefined,
            }),
        }
    }

    /// This uses python semantics for slicing
    pub(crate) fn slice(
        &self,
        start: Option<i128>,
        end: Option<i128>,
        step: Option<i128>,
    ) -> TeraResult<Value> {
        let step = step.unwrap_or(1);
        if step == 0 {
            return Err(Error::message("Slicing step cannot be 0".to_string()));
        }

        fn slice_items<T: Clone>(
            items: &[T],
            start: Option<i128>,
            end: Option<i128>,
            step: i128,
        ) -> Vec<T> {
            let len = items.len() as i128;

            // We will need to clamp depending on the step. If step is > 0, then it's just [0, len]
            // but if it's negative it's [-1, len - 1]. It's -1 because we need to go past idx 0
            let (lo, hi) = if step > 0 { (0, len) } else { (-1, len - 1) };
            let resolve = |param: Option<i128>, default: i128| -> i128 {
                match param {
                    None => default,
                    Some(p) => {
                        let p = if p < 0 { p.saturating_add(len) } else { p };
                        p.clamp(lo, hi)
                    }
                }
            };
            let s = resolve(start, if step > 0 { lo } else { hi });
            let e = resolve(end, if step > 0 { hi } else { lo });
            let mut out = Vec::new();
            let mut i = s;
            while if step > 0 { i < e } else { i > e } {
                out.push(items[i as usize].clone());
                i = i.saturating_add(step);
            }
            out
        }

        match &self.inner {
            ValueInner::Array(arr) => Ok(slice_items(arr, start, end, step).into()),
            ValueInner::String(s) => {
                let kind = s.kind();
                #[cfg(feature = "unicode")]
                let input: Vec<&str> = s.as_str().graphemes(true).collect();
                #[cfg(not(feature = "unicode"))]
                let input: Vec<char> = s.as_str().chars().collect();

                let parts = slice_items(&input, start, end, step);

                #[cfg(feature = "unicode")]
                let out_str = parts.join("");
                #[cfg(not(feature = "unicode"))]
                let out_str: String = parts.into_iter().collect();

                Ok(Value {
                    inner: ValueInner::String(SmartString::new(&out_str, kind)),
                })
            }
            _ => Err(Error::message(format!(
                "Slicing can only be used on arrays or strings, not on `{}`.",
                self.name()
            ))),
        }
    }

    /// Returns a sorted list of available field names if this is a map.
    /// Used for error messages only.
    pub(crate) fn available_fields(&self) -> Vec<String> {
        self.as_map()
            .map(|m| {
                m.keys()
                    .map(|k| k.to_string())
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Returns a string name for the current enum member.
    /// Used in error messages
    pub fn name(&self) -> &'static str {
        match &self.inner {
            ValueInner::Undefined => "undefined",
            ValueInner::None => "none",
            ValueInner::Bool(_) => "bool",
            ValueInner::U64(_) => "u64",
            ValueInner::I64(_) => "i64",
            ValueInner::F64(_) => "f64",
            ValueInner::U128(_) => "u128",
            ValueInner::I128(_) => "i128",
            ValueInner::Array(_) => "array",
            ValueInner::Bytes(_) => "bytes",
            ValueInner::String(_) => "string",
            ValueInner::Map(_) => "map/struct",
        }
    }
}

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match &self.inner {
            ValueInner::None | ValueInner::Undefined => serializer.serialize_unit(),
            ValueInner::Bool(b) => serializer.serialize_bool(*b),
            ValueInner::U64(u) => serializer.serialize_u64(*u),
            ValueInner::I64(i) => serializer.serialize_i64(*i),
            ValueInner::F64(f) => serializer.serialize_f64(*f),
            ValueInner::U128(u) => serializer.serialize_u128(**u),
            ValueInner::I128(i) => serializer.serialize_i128(**i),
            ValueInner::Bytes(b) => serializer.serialize_bytes(b),
            ValueInner::String(s) => serializer.serialize_str(s.as_str()),
            ValueInner::Array(arr) => {
                let mut seq = serializer.serialize_seq(Some(arr.len()))?;
                for val in arr.iter() {
                    seq.serialize_element(val)?;
                }
                seq.end()
            }
            ValueInner::Map(map) => {
                let mut m = serializer.serialize_map(Some(map.len()))?;
                for (key, val) in map.iter() {
                    m.serialize_entry(key, val)?;
                }
                m.end()
            }
        }
    }
}

impl From<ValueInner> for Value {
    fn from(inner: ValueInner) -> Self {
        Value { inner }
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Value {
            inner: ValueInner::Bool(value),
        }
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Value {
            inner: ValueInner::String(SmartString::new(value, StringKind::Normal)),
        }
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Value {
            inner: ValueInner::String(SmartString::new(&value, StringKind::Normal)),
        }
    }
}

impl From<std::borrow::Cow<'_, str>> for Value {
    fn from(value: std::borrow::Cow<'_, str>) -> Self {
        Value {
            inner: ValueInner::String(SmartString::new(&value, StringKind::Normal)),
        }
    }
}

impl From<u8> for Value {
    fn from(value: u8) -> Self {
        Value {
            inner: ValueInner::U64(value as u64),
        }
    }
}

impl From<i8> for Value {
    fn from(value: i8) -> Self {
        Value {
            inner: ValueInner::I64(value as i64),
        }
    }
}

impl From<u32> for Value {
    fn from(value: u32) -> Self {
        Value {
            inner: ValueInner::U64(value as u64),
        }
    }
}

impl From<u64> for Value {
    fn from(value: u64) -> Self {
        Value {
            inner: ValueInner::U64(value),
        }
    }
}

impl From<usize> for Value {
    fn from(value: usize) -> Self {
        Value {
            inner: ValueInner::U64(value as u64),
        }
    }
}

impl From<u128> for Value {
    fn from(value: u128) -> Self {
        Value {
            inner: ValueInner::U128(Box::new(value)),
        }
    }
}

impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Value {
            inner: ValueInner::I64(value as i64),
        }
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Value {
            inner: ValueInner::I64(value),
        }
    }
}

impl From<isize> for Value {
    fn from(value: isize) -> Self {
        Value {
            inner: ValueInner::I64(value as i64),
        }
    }
}

impl From<i128> for Value {
    fn from(value: i128) -> Self {
        Value {
            inner: ValueInner::I128(Box::new(value)),
        }
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Value {
            inner: ValueInner::F64(value),
        }
    }
}

impl From<Key<'static>> for Value {
    fn from(value: Key<'static>) -> Self {
        match value {
            Key::Bool(b) => Value {
                inner: ValueInner::Bool(b),
            },
            Key::U64(u) => Value {
                inner: ValueInner::U64(u),
            },
            Key::I64(i) => Value {
                inner: ValueInner::I64(i),
            },
            Key::U128(u) => Value {
                inner: ValueInner::U128(Box::new(u)),
            },
            Key::I128(i) => Value {
                inner: ValueInner::I128(Box::new(i)),
            },
            Key::String(s) => Value {
                inner: ValueInner::String(SmartString::new(&s, StringKind::Normal)),
            },
            Key::Str(s) => Value {
                inner: ValueInner::String(SmartString::new(s, StringKind::Normal)),
            },
        }
    }
}

impl From<&[Value]> for Value {
    fn from(value: &[Value]) -> Self {
        Value {
            inner: ValueInner::Array(Arc::new(value.to_vec())),
        }
    }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(value: Vec<T>) -> Self {
        Value {
            inner: ValueInner::Array(Arc::new(value.into_iter().map(|v| v.into()).collect())),
        }
    }
}

impl<T: Into<Value>> From<BTreeSet<T>> for Value {
    fn from(value: BTreeSet<T>) -> Self {
        Value {
            inner: ValueInner::Array(Arc::new(value.into_iter().map(|v| v.into()).collect())),
        }
    }
}

impl<K: Into<Key<'static>>, T: Into<Value>> From<HashMap<K, T>> for Value {
    fn from(input: HashMap<K, T>) -> Self {
        let mut map = Map::with_capacity(input.len());
        for (key, value) in input {
            map.insert(key.into(), value.into());
        }
        Value {
            inner: ValueInner::Map(Arc::new(map)),
        }
    }
}

impl<K: Into<Key<'static>>, T: Into<Value>> From<BTreeMap<K, T>> for Value {
    fn from(input: BTreeMap<K, T>) -> Self {
        let mut map = Map::with_capacity(input.len());
        for (key, value) in input {
            map.insert(key.into(), value.into());
        }
        Value {
            inner: ValueInner::Map(Arc::new(map)),
        }
    }
}

#[cfg(feature = "preserve_order")]
impl<K: Into<Key<'static>>, T: Into<Value>> From<indexmap::IndexMap<K, T>> for Value {
    fn from(input: indexmap::IndexMap<K, T>) -> Self {
        let mut map = Map::with_capacity(input.len());
        for (key, value) in input {
            map.insert(key.into(), value.into());
        }
        Value {
            inner: ValueInner::Map(Arc::new(map)),
        }
    }
}

/// The trait that automatically converts a value into a `TeraResult<Value>`
pub trait FunctionResult {
    #[allow(missing_docs)]
    fn into_result(self) -> TeraResult<Value>;
}

impl<I: Into<Value>> FunctionResult for TeraResult<I> {
    fn into_result(self) -> TeraResult<Value> {
        self.map(Into::into)
    }
}

impl<I: Into<Value>> FunctionResult for I {
    fn into_result(self) -> TeraResult<Value> {
        Ok(self.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // "école" with é = 'e' + U+0301
    #[cfg(not(feature = "unicode"))]
    #[test]
    fn len_and_reverse_use_chars() {
        let v = Value::from("e\u{0301}cole");
        assert_eq!(v.len(), Some(6));
        assert_eq!(v.reverse().unwrap().as_str(), Some("eloc\u{0301}e"));
    }

    #[cfg(feature = "unicode")]
    #[test]
    fn len_and_reverse_use_graphemes() {
        let v = Value::from("e\u{0301}cole");
        assert_eq!(v.len(), Some(5));
        assert_eq!(v.reverse().unwrap().as_str(), Some("eloce\u{0301}"));
    }
}
