use std::sync::Arc;

use crate::Value;
use crate::args::{ArgFromValue, Kwargs};
use crate::errors::{Error, TeraResult};
use crate::value::ValueKind;
use crate::value::number::Number;
use crate::vm::state::State;

pub trait TestResult {
    fn into_result(self) -> TeraResult<bool>;
}

impl TestResult for TeraResult<bool> {
    fn into_result(self) -> TeraResult<bool> {
        self
    }
}

impl TestResult for bool {
    fn into_result(self) -> TeraResult<bool> {
        Ok(self)
    }
}

/// The test function type definition
pub trait Test<Arg, Res>: Sync + Send + 'static {
    /// The test function type definition
    fn call(&self, value: Arg, kwargs: Kwargs, state: &State) -> Res;
}

impl<Func, Arg, Res> Test<Arg, Res> for Func
where
    Func: Fn(Arg, Kwargs, &State) -> Res + Sync + Send + 'static,
    Arg: for<'a> ArgFromValue<'a>,
    Res: TestResult,
{
    fn call(&self, value: Arg, kwargs: Kwargs, state: &State) -> Res {
        (self)(value, kwargs, state)
    }
}

type TestFunc = dyn Fn(&Value, Kwargs, &State) -> TeraResult<bool> + Sync + Send + 'static;

#[derive(Clone)]
pub(crate) struct StoredTest(Arc<TestFunc>);

impl StoredTest {
    pub fn new<Func, Arg, Res>(f: Func) -> Self
    where
        Func: Test<Arg, Res> + for<'a> Test<<Arg as ArgFromValue<'a>>::Output, Res>,
        Arg: for<'a> ArgFromValue<'a>,
        Res: TestResult,
    {
        let closure = move |arg: &Value, kwargs, state: &State| -> TeraResult<bool> {
            f.call(Arg::from_value(arg)?, kwargs, state).into_result()
        };

        StoredTest(Arc::new(closure))
    }

    pub fn call(&self, arg: &Value, kwargs: Kwargs, state: &State) -> TeraResult<bool> {
        (self.0)(arg, kwargs, state)
    }
}

pub(crate) fn is_string(val: &Value, _: Kwargs, _: &State) -> bool {
    val.is_string()
}

pub(crate) fn is_number(val: &Value, _: Kwargs, _: &State) -> bool {
    val.is_number()
}

pub(crate) fn is_map(val: &Value, _: Kwargs, _: &State) -> bool {
    val.is_map()
}

pub(crate) fn is_bool(val: &Value, _: Kwargs, _: &State) -> bool {
    val.is_bool()
}

pub(crate) fn is_array(val: &Value, _: Kwargs, _: &State) -> bool {
    val.is_array()
}

pub(crate) fn is_none(val: &Value, _: Kwargs, _: &State) -> bool {
    val.is_none()
}

pub(crate) fn is_undefined(val: &Value, _: Kwargs, _: &State) -> bool {
    val.is_undefined()
}

pub(crate) fn is_defined(val: &Value, _: Kwargs, _: &State) -> bool {
    !val.is_undefined()
}

pub(crate) fn is_iterable(val: &Value, _: Kwargs, _: &State) -> bool {
    val.is_map() || val.is_array() || val.is_string() || val.is_bytes()
}

pub(crate) fn is_integer(val: &Value, _: Kwargs, _: &State) -> bool {
    val.is_number() && !val.is_f64()
}

pub(crate) fn is_float(val: &Value, _: Kwargs, _: &State) -> bool {
    val.is_f64()
}

pub(crate) fn is_odd(val: Number, _: Kwargs, _: &State) -> TeraResult<bool> {
    match val {
        Number::Integer(u) => Ok(u % 2 != 0),
        Number::Float(u) => Err(Error::message(format!(
            "Value `{u}` is a float; cannot determine if it's odd"
        ))),
    }
}

pub(crate) fn is_even(val: Number, _: Kwargs, _: &State) -> TeraResult<bool> {
    match val {
        Number::Integer(u) => Ok(u % 2 == 0),
        Number::Float(u) => Err(Error::message(format!(
            "Value `{u}` is a float; cannot determine if it's even"
        ))),
    }
}

pub(crate) fn is_divisible_by(val: Number, kwargs: Kwargs, _: &State) -> TeraResult<bool> {
    let divisor = kwargs.must_get::<i128>("divisor")?;
    if divisor == 0 {
        return Ok(false);
    }
    match val {
        Number::Integer(u) => match u.checked_rem_euclid(divisor) {
            Some(r) => Ok(r == 0),
            None => Ok(true),
        },
        Number::Float(u) => Err(Error::message(format!(
            "Value `{u}` is a float; cannot check divisibility"
        ))),
    }
}

pub(crate) fn is_starting_with(val: &str, kwargs: Kwargs, _: &State) -> TeraResult<bool> {
    let pat = kwargs.must_get::<&str>("pat")?;
    Ok(val.starts_with(pat))
}

pub(crate) fn is_ending_with(val: &str, kwargs: Kwargs, _: &State) -> TeraResult<bool> {
    let pat = kwargs.must_get::<&str>("pat")?;
    Ok(val.ends_with(pat))
}

pub(crate) fn is_containing(val: &Value, kwargs: Kwargs, _: &State) -> TeraResult<bool> {
    let pat = kwargs.must_get::<&Value>("pat")?;
    match val.kind() {
        ValueKind::String => {
            let s = <&str as ArgFromValue>::from_value(pat)?;
            Ok(val.as_str().unwrap().contains(s))
        }
        ValueKind::Array => Ok(val.as_vec().unwrap().contains(pat)),
        ValueKind::Map => Ok(match pat.as_key() {
            Ok(key) => val.as_map().unwrap().contains_key(&key),
            Err(_) => false,
        }),
        _ => Err(Error::message(format!(
            "Value `{val}` is not a container; cannot check for containment"
        ))),
    }
}
