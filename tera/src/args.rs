use serde::Deserialize;
use std::borrow::Cow;
use std::sync::Arc;

use crate::Value;
use crate::errors::{Error, TeraResult};
use crate::value::number::Number;
use crate::value::{Key, Map, ValueInner};

mod private {
    use super::{Map, Number, Value};
    use std::borrow::Cow;

    pub trait Sealed {}

    impl Sealed for bool {}
    impl Sealed for f32 {}
    impl Sealed for f64 {}
    impl Sealed for u8 {}
    impl Sealed for u16 {}
    impl Sealed for u32 {}
    impl Sealed for u64 {}
    impl Sealed for u128 {}
    impl Sealed for usize {}
    impl Sealed for i8 {}
    impl Sealed for i16 {}
    impl Sealed for i32 {}
    impl Sealed for i64 {}
    impl Sealed for i128 {}
    impl Sealed for isize {}
    impl Sealed for String {}
    impl Sealed for &str {}
    impl Sealed for &[Value] {}
    impl<'a> Sealed for Cow<'a, str> {}
    impl Sealed for Value {}
    impl Sealed for &Value {}
    impl Sealed for Number {}
    impl Sealed for Map {}
    impl Sealed for &Map {}
    impl<T: Sealed> Sealed for Vec<T> {}
}

#[doc(hidden)]
pub trait ArgFromValue<'k>: private::Sealed {
    type Output;

    fn from_value(value: &'k Value) -> TeraResult<Self::Output>;
}

macro_rules! impl_for_literal {
    ($ty:ident, {
        $($pat:pat $(if $if_expr:expr)? => $expr:expr,)*
    }) => {
        impl TryFrom<Value> for $ty {
            type Error = Error;

            fn try_from(value: Value) -> Result<Self, Self::Error> {
                let res = match &value.inner {
                    $($pat $(if $if_expr)? => TryFrom::try_from($expr).ok(),)*
                    _ => None
                };

                res.ok_or_else(|| Error::invalid_arg_type(stringify!($ty), value.name()))
            }
        }

        impl<'k> ArgFromValue<'k> for $ty {
            type Output = Self;
            fn from_value(value: &Value) -> Result<Self, Error> {
                let res = match &value.inner {
                    $($pat $(if $if_expr)? => TryFrom::try_from($expr).ok(),)*
                    _ => None
                };
                res.ok_or_else(|| Error::invalid_arg_type(stringify!($ty), value.name()))
            }
        }
    }
}

fn int_from_value<T>(value: &Value, target_type: &'static str) -> TeraResult<T>
where
    T: TryFrom<i64> + TryFrom<i128> + TryFrom<u64> + TryFrom<u128>,
{
    let res = match &value.inner {
        ValueInner::I64(v) => T::try_from(*v).ok(),
        ValueInner::I128(v) => T::try_from(**v).ok(),
        ValueInner::U64(v) => T::try_from(*v).ok(),
        ValueInner::U128(v) => T::try_from(**v).ok(),
        ValueInner::F64(v) if v.trunc() == *v => {
            // We try to convert to a i128 only if it fits
            if *v >= i128::MIN as f64 && *v < i128::MAX as f64 {
                T::try_from(*v as i128).ok()
            } else {
                None
            }
        }
        _ => return Err(Error::invalid_arg_type(target_type, value.name())),
    };
    res.ok_or_else(|| Error::out_of_range_arg(value, target_type))
}

macro_rules! impl_for_int {
    ($ty:ident) => {
        impl TryFrom<Value> for $ty {
            type Error = Error;

            fn try_from(value: Value) -> Result<Self, Self::Error> {
                int_from_value(&value, stringify!($ty))
            }
        }

        impl<'k> ArgFromValue<'k> for $ty {
            type Output = Self;

            fn from_value(value: &Value) -> Result<Self, Error> {
                int_from_value(value, stringify!($ty))
            }
        }
    };
}
impl_for_int!(u8);
impl_for_int!(u16);
impl_for_int!(u32);
impl_for_int!(u64);
impl_for_int!(u128);
impl_for_int!(usize);
impl_for_int!(i8);
impl_for_int!(i16);
impl_for_int!(i32);
impl_for_int!(i64);
impl_for_int!(i128);
impl_for_int!(isize);

impl_for_literal!(bool, {
    ValueInner::Bool(b) => *b,
});

// TODO: test when value doesn't fit in f32
impl_for_literal!(f32, {
    ValueInner::I64(b) => *b as f32,
    ValueInner::I128(b) => **b as f32,
    ValueInner::U64(b) => *b as f32,
    ValueInner::U128(b) => **b as f32,
    ValueInner::F64(b) => *b as f32,
});
impl_for_literal!(f64, {
    ValueInner::I64(b) => *b as f64,
    ValueInner::I128(b) => **b as f64,
    ValueInner::U64(b) => *b as f64,
    ValueInner::U128(b) => **b as f64,
    ValueInner::F64(b) => *b,
});

impl<'k> ArgFromValue<'k> for String {
    type Output = String;

    fn from_value(value: &'k Value) -> TeraResult<Self::Output> {
        Ok(format!("{value}"))
    }
}

impl<'k> ArgFromValue<'k> for &str {
    type Output = &'k str;

    fn from_value(value: &'k Value) -> TeraResult<Self::Output> {
        value
            .as_str()
            .ok_or_else(|| Error::invalid_arg_type("&str", value.name()))
    }
}

impl<'k> ArgFromValue<'k> for Cow<'_, str> {
    type Output = Cow<'k, str>;

    fn from_value(value: &'k Value) -> TeraResult<Self::Output> {
        match &value.inner {
            ValueInner::String(s) => Ok(Cow::Borrowed(s.as_str())),
            _ => Ok(Cow::Owned(format!("{value}"))),
        }
    }
}

impl<'k> ArgFromValue<'k> for &Value {
    type Output = &'k Value;

    fn from_value(value: &'k Value) -> TeraResult<Self::Output> {
        Ok(value)
    }
}

impl<'k> ArgFromValue<'k> for Value {
    type Output = Value;

    fn from_value(value: &'k Value) -> TeraResult<Self::Output> {
        Ok(value.clone())
    }
}

impl<'k> ArgFromValue<'k> for Number {
    type Output = Number;

    fn from_value(value: &'k Value) -> TeraResult<Self::Output> {
        if let Some(n) = value.as_number() {
            Ok(n)
        } else if value.is_number() {
            Err(Error::message(format!(
                "Number `{value}` is out of range for i128"
            )))
        } else {
            Err(Error::invalid_arg_type("Number", value.name()))
        }
    }
}

impl<'k> ArgFromValue<'k> for Map {
    type Output = Map;

    fn from_value(value: &'k Value) -> TeraResult<Self::Output> {
        value
            .as_map()
            .cloned()
            .ok_or_else(|| Error::invalid_arg_type("Map", value.name()))
    }
}

impl<'k> ArgFromValue<'k> for &Map {
    type Output = &'k Map;

    fn from_value(value: &'k Value) -> TeraResult<Self::Output> {
        value
            .as_map()
            .ok_or_else(|| Error::invalid_arg_type("Map", value.name()))
    }
}

impl<'k, T: ArgFromValue<'k, Output = T>> ArgFromValue<'k> for Vec<T> {
    type Output = Vec<T>;

    fn from_value(value: &'k Value) -> TeraResult<Self::Output> {
        match &value.inner {
            ValueInner::Array(arr) => {
                let mut res = Vec::with_capacity(arr.len());
                for v in arr.iter() {
                    res.push(T::from_value(v)?);
                }
                Ok(res)
            }
            _ => Err(Error::invalid_arg_type("Vec<Value>", value.name())),
        }
    }
}

impl<'k> ArgFromValue<'k> for &[Value] {
    type Output = &'k [Value];

    fn from_value(value: &'k Value) -> TeraResult<Self::Output> {
        match &value.inner {
            ValueInner::Array(arr) => Ok(arr.as_slice()),
            _ => Err(Error::invalid_arg_type("&[Value]", value.name())),
        }
    }
}

/// The keyword arguments of a filter/function
#[derive(Debug, Clone, Default)]
pub struct Kwargs {
    values: Arc<Map>,
}

impl Kwargs {
    /// Creates a new Kwargs struct from a Map. The Map is Arc<_> since internally
    /// that's what we have.
    pub fn new(map: Arc<Map>) -> Self {
        Self { values: map }
    }

    /// Deserialize the kwargs into something that impl Deserialize
    pub fn deserialize<'a, T: Deserialize<'a>>(&'a self) -> TeraResult<T> {
        T::deserialize(&Value {
            inner: ValueInner::Map(self.values.clone()),
        })
        .map_err(Error::message)
    }

    /// Try to get the given key value and convert it to the given type
    /// Returns None if not found
    pub fn get<'k, T>(&'k self, key: &'k str) -> TeraResult<Option<T>>
    where
        T: ArgFromValue<'k, Output = T>,
    {
        match self.values.get(&Key::Str(key)) {
            Some(v) => T::from_value(v).map(|v| Some(v)),
            None => Ok(None),
        }
    }

    /// Try to get the given key value.
    /// Returns an error if not found.
    pub fn must_get<'k, T>(&'k self, key: &'k str) -> TeraResult<T>
    where
        T: ArgFromValue<'k, Output = T>,
    {
        if let Some(v) = self.get(key)? {
            Ok(v)
        } else {
            Err(Error::missing_arg(key))
        }
    }
}

impl<const N: usize> From<[(&'static str, Value); N]> for Kwargs {
    fn from(pairs: [(&'static str, Value); N]) -> Self {
        let mut map = Map::new();
        for (k, v) in pairs {
            map.insert(k.into(), v);
        }
        Kwargs::new(Arc::new(map))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_get_kwarg_with_type() {
        #[derive(Debug, Deserialize)]
        struct Data {
            hello: String,
            num: f64,
        }

        let mut map = Map::new();
        map.insert("hello".into(), Value::from("world"));
        map.insert("num".into(), Value::from(1.1));
        let kwargs = Kwargs::new(Arc::new(map));
        assert_eq!(kwargs.get("hello").unwrap(), Some("world"));
        assert_eq!(kwargs.get("num").unwrap(), Some(1.1));
        assert_eq!(kwargs.get::<i64>("unknown").unwrap(), None);

        let data: Data = kwargs.deserialize().unwrap();
        assert_eq!(data.num, 1.1);
        assert_eq!(data.hello, "world");
    }

    #[test]
    fn int_out_of_range_reports_range_not_type() {
        let kwargs = Kwargs::from([("n", Value::from(300))]);
        let err = kwargs.get::<u8>("n").unwrap_err();
        assert_eq!(err.to_string(), "Value `300` is out of range for `u8`");

        let kwargs = Kwargs::from([("n", Value::from(-1))]);
        let err = kwargs.get::<usize>("n").unwrap_err();
        assert_eq!(err.to_string(), "Value `-1` is out of range for `usize`");

        let kwargs = Kwargs::from([("n", Value::from(1e40_f64))]);
        assert!(
            kwargs
                .get::<i128>("n")
                .unwrap_err()
                .to_string()
                .contains("out of range")
        );
    }
}
