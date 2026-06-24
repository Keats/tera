use crate::errors::{Error, TeraResult};
use crate::value::Value;
use std::cmp::Ordering;
use std::fmt;
use std::fmt::Formatter;

/// Simpler representation of numbers so operations are simpler to handle
/// Also can be used for custom filters/tests/fn when you want to ensure you get a number
#[derive(Debug, Copy, Clone)]
#[non_exhaustive]
pub enum Number {
    /// Integers are stored in i128, which means we can't use numbers from u128 above i128::MAX
    /// for math, which is probably ok for a template engine.
    Integer(i128),
    #[allow(missing_docs)]
    Float(f64),
}

impl fmt::Display for Number {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Number::Integer(v) => write!(f, "{v}"),
            Number::Float(v) => write!(f, "{v}"),
        }
    }
}

impl From<Number> for Value {
    fn from(n: Number) -> Self {
        match n {
            Number::Integer(i) => Value::from(i),
            Number::Float(f) => Value::from(f),
        }
    }
}

impl PartialEq for Number {
    fn eq(&self, other: &Self) -> bool {
        Value::from(*self) == Value::from(*other)
    }
}

impl PartialOrd for Number {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Value::from(*self).partial_cmp(&Value::from(*other))
    }
}

impl Number {
    /// Is the number a float?
    pub fn is_float(&self) -> bool {
        matches!(self, Number::Float(..))
    }

    /// Is the number an integer? Does not check whether the float could be converted
    /// to an integer losslessly, just checks the type.
    pub fn is_integer(&self) -> bool {
        matches!(self, Number::Integer(..))
    }

    pub(crate) fn into_float(self) -> Self {
        match self {
            Number::Float(f) => Number::Float(f),
            Number::Integer(f) => Number::Float(f as f64),
        }
    }

    /// Converts a number to a float.
    pub fn as_float(&self) -> f64 {
        match self {
            Number::Float(f) => *f,
            Number::Integer(f) => *f as f64,
        }
    }

    /// Tries to convert a number to a i128.
    pub fn as_integer(&self) -> Option<i128> {
        match self {
            // `i128::MAX as f64` rounds up to `2^127`, so the upper bound is exclusive.
            Number::Float(f)
                if f.fract() == 0.0
                    && f.is_finite()
                    && *f >= i128::MIN as f64
                    && *f < i128::MAX as f64 =>
            {
                Some(*f as i128)
            }
            Number::Float(_) => None,
            Number::Integer(f) => Some(*f),
        }
    }

    pub(crate) fn is_zero(&self) -> bool {
        match self {
            Number::Float(f) => f.is_finite() && f == &0.0,
            Number::Integer(f) => f == &0i128,
        }
    }
}

fn arg_error(val: &Value) -> Error {
    if val.is_number() {
        Error::message(format!(
            "Operand `{val}` is out of range for integer arithmetic (must fit in i128)"
        ))
    } else {
        Error::message(format!(
            "Only numbers can be used in arithmetic. This is a `{}`",
            val.name()
        ))
    }
}

macro_rules! math {
    ($name:ident, $op_fn:ident, $sign:tt) => {
        pub(crate) fn $name(lhs: &Value, rhs: &Value) -> TeraResult<Value> {
            match (lhs.as_number(), rhs.as_number()) {
                (Some(mut left), Some(mut right)) => {
                    if left.is_float() || right.is_float() {
                        left = left.into_float();
                        right = right.into_float();
                    }

                    let val = match (left, right) {
                        (Number::Integer(a), Number::Integer(b)) => match a.$op_fn(b) {
                            Some(val) => Value::from(val),
                            None => return Err(Error::message(format!("Unable to perform {lhs} {} {rhs}", stringify!($sign)))),
                        },
                        (Number::Float(a), Number::Float(b)) => Value::from(a $sign b),
                        _ => unreachable!(),
                    };
                    Ok(val)
                }
                (None, _) => Err(arg_error(lhs)),
                (_, None) => Err(arg_error(rhs)),
            }
        }
    }
}

math!(add, checked_add, +);
math!(sub, checked_sub, -);
math!(mul, checked_mul, *);

pub(crate) fn rem(lhs: &Value, rhs: &Value) -> TeraResult<Value> {
    match (lhs.as_number(), rhs.as_number()) {
        (Some(mut left), Some(mut right)) => {
            if right.is_zero() {
                return Err(Error::message("Cannot divide by 0".to_string()));
            }

            if left.is_float() || right.is_float() {
                left = left.into_float();
                right = right.into_float();
            }

            let val = match (left, right) {
                (Number::Integer(a), Number::Integer(b)) => match a.checked_rem_euclid(b) {
                    Some(val) => Value::from(val),
                    None => {
                        return Err(Error::message(format!("Unable to perform {lhs} % {rhs}")));
                    }
                },
                (Number::Float(a), Number::Float(b)) => Value::from(a.rem_euclid(b)),
                _ => unreachable!(),
            };
            Ok(val)
        }
        (None, _) => Err(arg_error(lhs)),
        (_, None) => Err(arg_error(rhs)),
    }
}

pub(crate) fn div(lhs: &Value, rhs: &Value) -> TeraResult<Value> {
    match (lhs.as_number(), rhs.as_number()) {
        (Some(left), Some(right)) => {
            if right.is_zero() {
                return Err(Error::message("Cannot divide by 0".to_string()));
            }

            Ok((left.as_float() / right.as_float()).into())
        }
        (None, _) => Err(arg_error(lhs)),
        (_, None) => Err(arg_error(rhs)),
    }
}

pub(crate) fn floor_div(lhs: &Value, rhs: &Value) -> TeraResult<Value> {
    match (lhs.as_number(), rhs.as_number()) {
        (Some(mut left), Some(mut right)) => {
            if right.is_zero() {
                return Err(Error::message("Cannot divide by 0".to_string()));
            }

            if left.is_float() || right.is_float() {
                left = left.into_float();
                right = right.into_float();
            }

            let val = match (left, right) {
                (Number::Integer(a), Number::Integer(b)) => match a.checked_div_euclid(b) {
                    Some(val) => Value::from(val),
                    None => {
                        return Err(Error::message(format!("Unable to perform {lhs} // {rhs}")));
                    }
                },
                (Number::Float(a), Number::Float(b)) => Value::from(a.div_euclid(b)),
                _ => unreachable!(),
            };
            Ok(val)
        }
        (None, _) => Err(arg_error(lhs)),
        (_, None) => Err(arg_error(rhs)),
    }
}

pub(crate) fn pow(lhs: &Value, rhs: &Value) -> TeraResult<Value> {
    match (lhs.as_number(), rhs.as_number()) {
        (Some(mut left), Some(mut right)) => {
            // Convert to float is one of them is or if exponent is < 0
            let negative_int_exp = matches!(right, Number::Integer(b) if b < 0);
            if left.is_float() || right.is_float() || negative_int_exp {
                left = left.into_float();
                right = right.into_float();
            }

            let val = match (left, right) {
                (Number::Integer(a), Number::Integer(b)) => {
                    let exp = u32::try_from(b).map_err(|_| {
                        Error::message(format!(
                            "Exponent {b} is out of range for integer ** (must fit in u32)"
                        ))
                    })?;
                    match a.checked_pow(exp) {
                        Some(val) => Value::from(val),
                        None => {
                            return Err(Error::message(format!(
                                "Unable to perform {lhs} ** {rhs}"
                            )));
                        }
                    }
                }
                (Number::Float(a), Number::Float(b)) => Value::from(a.powf(b)),
                _ => unreachable!(),
            };
            Ok(val)
        }
        (None, _) => Err(arg_error(lhs)),
        (_, None) => Err(arg_error(rhs)),
    }
}

pub(crate) fn negate(val: &Value) -> TeraResult<Value> {
    if let Some(num) = val.as_number() {
        let val = match num {
            Number::Float(f) => Value::from(-f),
            Number::Integer(f) => match f.checked_neg() {
                Some(n) => Value::from(n),
                None => {
                    return Err(Error::message(format!(
                        "Cannot negate {f}: result would overflow i128"
                    )));
                }
            },
        };
        Ok(val)
    } else if val.is_number() {
        Err(Error::message(format!(
            "Cannot negate {val}: result would overflow i128"
        )))
    } else {
        Err(Error::message(format!(
            "Only numbers can be negated. This is a `{}`",
            val.name()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_negate() {
        let err = negate(&Value::from(i128::MIN)).unwrap_err();
        assert!(err.to_string().contains("overflow"));
        assert_eq!(negate(&Value::from(5i64)).unwrap(), Value::from(-5i64));
        assert_eq!(negate(&Value::from(-5i64)).unwrap(), Value::from(5i64));
        assert_eq!(
            negate(&Value::from(i128::MAX)).unwrap(),
            Value::from(-i128::MAX)
        );
        assert_eq!(negate(&Value::from(5u128)).unwrap(), Value::from(-5i128));
        let err = negate(&Value::from(u128::MAX)).unwrap_err();
        assert!(err.to_string().contains("overflow"));
    }

    #[test]
    fn arithmetic_in_i128() {
        let res = add(&Value::from(-1i64), &Value::from(2u128)).unwrap();
        assert_eq!(res.as_i128(), Some(1));
        let res = sub(&Value::from(5u128), &Value::from(10u64)).unwrap();
        assert_eq!(res.as_i128(), Some(-5));
        let err = add(&Value::from(i128::MAX), &Value::from(1i128)).unwrap_err();
        assert!(err.to_string().contains("Unable to perform"));
        let err = add(&Value::from(u128::MAX), &Value::from(0u64)).unwrap_err();
        assert!(err.to_string().contains("out of range"));
    }
}
