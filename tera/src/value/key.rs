use crate::Value;
use serde::{Serialize, Serializer};
use std::borrow::Cow;
use std::cmp::Ordering;
use std::fmt;
use std::fmt::Formatter;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// The key of anything looking like a hashmap (struct/hashmaps)
#[derive(Debug, Clone)]
pub enum Key<'a> {
    #[allow(missing_docs)]
    Bool(bool),
    #[allow(missing_docs)]
    U64(u64),
    #[allow(missing_docs)]
    I64(i64),
    #[allow(missing_docs)]
    U128(u128),
    #[allow(missing_docs)]
    I128(i128),
    #[allow(missing_docs)]
    String(Arc<str>),
    #[allow(missing_docs)]
    Str(&'a str),
}

impl<'a> Key<'a> {
    /// Returns the content if the key is a string
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Key::String(s) => Some(s),
            Key::Str(s) => Some(s),
            _ => None,
        }
    }

    #[allow(missing_docs)]
    pub fn as_value(&self) -> Value {
        match self {
            Key::Bool(b) => Value::from(*b),
            Key::U64(b) => Value::from(*b),
            Key::I64(b) => Value::from(*b),
            Key::U128(b) => Value::from(*b),
            Key::I128(b) => Value::from(*b),
            Key::String(b) => Value::normal_string(b.as_ref()),
            Key::Str(b) => Value::normal_string(b),
        }
    }

    pub(crate) fn format(&self, f: &mut impl std::io::Write) -> std::io::Result<()> {
        match self {
            Key::Bool(v) => f.write_all(if *v { b"true" } else { b"false" }),
            Key::String(v) => f.write_all(v.as_bytes()),
            Key::Str(v) => f.write_all(v.as_bytes()),
            #[cfg(feature = "no_fmt")]
            Key::U64(v) => {
                let mut buf = itoa::Buffer::new();
                f.write_all(buf.format(*v).as_bytes())
            }
            #[cfg(feature = "no_fmt")]
            Key::I64(v) => {
                let mut buf = itoa::Buffer::new();
                f.write_all(buf.format(*v).as_bytes())
            }
            #[cfg(feature = "no_fmt")]
            Key::U128(v) => {
                let mut buf = itoa::Buffer::new();
                f.write_all(buf.format(*v).as_bytes())
            }
            #[cfg(feature = "no_fmt")]
            Key::I128(v) => {
                let mut buf = itoa::Buffer::new();
                f.write_all(buf.format(*v).as_bytes())
            }
            #[cfg(not(feature = "no_fmt"))]
            _ => write!(f, "{self}"),
        }
    }
}

impl<'a> PartialEq for Key<'a> {
    fn eq(&self, other: &Self) -> bool {
        if let (Some(a), Some(b)) = (self.as_str(), other.as_str()) {
            return a.eq(b);
        }

        match (self, other) {
            (Key::Bool(a), Key::Bool(b)) => a == b,
            (a, b) => match (a.as_number(), b.as_number()) {
                (Some(left), Some(right)) => left == right,
                _ => false,
            },
        }
    }
}

impl<'a> Eq for Key<'a> {}

impl<'a> PartialOrd for Key<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> Ord for Key<'a> {
    fn cmp(&self, other: &Self) -> Ordering {
        if let (Some(a), Some(b)) = (self.as_str(), other.as_str()) {
            return a.cmp(b);
        }

        if let (Key::Bool(a), Key::Bool(b)) = (self, other) {
            return a.cmp(b);
        }

        match (self.as_number(), other.as_number()) {
            (Some(a), Some(b)) => a.cmp(&b),
            _ => type_order(self).cmp(&type_order(other)),
        }
    }
}

impl<'a> Hash for Key<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        if let Some(s) = self.as_str() {
            return s.hash(state);
        }

        match self {
            Key::Bool(v) => v.hash(state),
            _ => {
                if let Some(num) = self.as_number() {
                    num.hash(state)
                }
            }
        }
    }
}

impl<'a> fmt::Display for Key<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Key::Bool(v) => write!(f, "{v}"),
            Key::U64(v) => write!(f, "{v}"),
            Key::I64(v) => write!(f, "{v}"),
            Key::U128(v) => write!(f, "{v}"),
            Key::I128(v) => write!(f, "{v}"),
            Key::Str(v) => write!(f, "{v}"),
            Key::String(v) => write!(f, "{v}"),
        }
    }
}

impl<'a> Serialize for Key<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Key::Bool(b) => serializer.serialize_bool(*b),
            Key::U64(u) => serializer.serialize_u64(*u),
            Key::I64(i) => serializer.serialize_i64(*i),
            Key::U128(u) => serializer.serialize_u128(*u),
            Key::I128(i) => serializer.serialize_i128(*i),
            Key::String(s) => serializer.serialize_str(s),
            Key::Str(s) => serializer.serialize_str(s),
        }
    }
}

impl From<&'static str> for Key<'static> {
    fn from(value: &'static str) -> Self {
        Key::Str(value)
    }
}

impl From<String> for Key<'static> {
    fn from(value: String) -> Self {
        Key::String(Arc::from(value))
    }
}

impl From<u128> for Key<'static> {
    fn from(value: u128) -> Self {
        Key::U128(value)
    }
}

impl From<i128> for Key<'static> {
    fn from(value: i128) -> Self {
        Key::I128(value)
    }
}

impl<'a> From<Cow<'a, str>> for Key<'static> {
    fn from(value: Cow<'a, str>) -> Self {
        match value {
            Cow::Borrowed(s) => Key::String(Arc::from(s)),
            Cow::Owned(s) => Key::String(Arc::from(s)),
        }
    }
}

#[derive(Clone, Copy)]
enum KeyNumber {
    Signed(i128),
    Unsigned(u128),
}

impl KeyNumber {
    fn from_key(key: &Key<'_>) -> Option<Self> {
        match key {
            Key::U64(v) => Some(KeyNumber::Unsigned(*v as u128)),
            Key::I64(v) => Some(KeyNumber::Signed(*v as i128)),
            Key::U128(v) => Some(KeyNumber::Unsigned(*v)),
            Key::I128(v) => Some(KeyNumber::Signed(*v)),
            _ => None,
        }
    }
}

/// Just to be able to have Ord on Key
fn type_order(key: &Key<'_>) -> u8 {
    match key {
        Key::Bool(_) => 0,
        Key::U64(_) | Key::I64(_) | Key::U128(_) | Key::I128(_) => 1,
        Key::String(_) | Key::Str(_) => 2,
    }
}

impl Key<'_> {
    fn as_number(&self) -> Option<KeyNumber> {
        KeyNumber::from_key(self)
    }
}

impl PartialOrd for KeyNumber {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for KeyNumber {
    fn eq(&self, other: &Self) -> bool {
        match (*self, *other) {
            (KeyNumber::Signed(a), KeyNumber::Signed(b)) => a == b,
            (KeyNumber::Unsigned(a), KeyNumber::Unsigned(b)) => a == b,
            (KeyNumber::Signed(a), KeyNumber::Unsigned(b)) => {
                if a < 0 {
                    false
                } else {
                    (a as u128) == b
                }
            }
            (KeyNumber::Unsigned(a), KeyNumber::Signed(b)) => {
                if b < 0 {
                    false
                } else {
                    a == (b as u128)
                }
            }
        }
    }
}

impl Eq for KeyNumber {}

impl Ord for KeyNumber {
    fn cmp(&self, other: &Self) -> Ordering {
        match (*self, *other) {
            (KeyNumber::Signed(a), KeyNumber::Signed(b)) => a.cmp(&b),
            (KeyNumber::Unsigned(a), KeyNumber::Unsigned(b)) => a.cmp(&b),
            (KeyNumber::Signed(a), KeyNumber::Unsigned(b)) => {
                if a < 0 {
                    Ordering::Less
                } else {
                    (a as u128).cmp(&b)
                }
            }
            (KeyNumber::Unsigned(a), KeyNumber::Signed(b)) => {
                if b < 0 {
                    Ordering::Greater
                } else {
                    a.cmp(&(b as u128))
                }
            }
        }
    }
}

impl Hash for KeyNumber {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match *self {
            KeyNumber::Signed(v) if v < 0 => {
                1u8.hash(state);
                v.hash(state);
            }
            KeyNumber::Signed(v) => {
                0u8.hash(state);
                (v as u128).hash(state);
            }
            KeyNumber::Unsigned(v) => {
                0u8.hash(state);
                v.hash(state);
            }
        }
    }
}
