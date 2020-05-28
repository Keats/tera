/// Filters operating on numbers
use std::collections::HashMap;

#[cfg(feature = "builtins")]
use humansize::{file_size_opts, FileSize};
use serde_json::value::{to_value, Value};

use crate::errors::{Error, Result};

/// Returns a plural suffix if the value is not equal to ±1, or a singular
/// suffix otherwise. The plural suffix defaults to `s` and the singular suffix
/// defaults to the empty string (i.e nothing).
pub fn pluralize(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
    let num = try_get_value!("pluralize", "value", f64, value);

    let plural = match args.get("plural") {
        Some(val) => try_get_value!("pluralize", "plural", String, val),
        None => "s".to_string(),
    };

    let singular = match args.get("singular") {
        Some(val) => try_get_value!("pluralize", "singular", String, val),
        None => "".to_string(),
    };

    // English uses plural when it isn't one
    if (num.abs() - 1.).abs() > ::std::f64::EPSILON {
        Ok(to_value(&plural).unwrap())
    } else {
        Ok(to_value(&singular).unwrap())
    }
}

/// Returns a rounded number using the `method` arg and `precision` given.
/// `method` defaults to `common` which will round to the nearest number.
/// `ceil` and `floor` are also available as method.
/// `precision` defaults to `0`, meaning it will round to an integer
pub fn round(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
    let num = try_get_value!("round", "value", f64, value);
    let method = match args.get("method") {
        Some(val) => try_get_value!("round", "method", String, val),
        None => "common".to_string(),
    };
    let precision = match args.get("precision") {
        Some(val) => try_get_value!("round", "precision", i32, val),
        None => 0,
    };
    let multiplier = if precision == 0 { 1.0 } else { 10.0_f64.powi(precision) };

    match method.as_ref() {
        "common" => Ok(to_value((multiplier * num).round() / multiplier).unwrap()),
        "ceil" => Ok(to_value((multiplier * num).ceil() / multiplier).unwrap()),
        "floor" => Ok(to_value((multiplier * num).floor() / multiplier).unwrap()),
        _ => Err(Error::msg(format!(
            "Filter `round` received an incorrect value for arg `method`: got `{:?}`, \
             only common, ceil and floor are allowed",
            method
        ))),
    }
}

/// Returns a human-readable file size (i.e. '110 MB') from an integer
#[cfg(feature = "builtins")]
pub fn filesizeformat(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let num = try_get_value!("filesizeformat", "value", usize, value);
    num.file_size(file_size_opts::CONVENTIONAL)
        .or_else(|_| {
            Err(Error::msg(format!(
                "Filter `filesizeformat` was called on a negative number: {}",
                num
            )))
        })
        .map(to_value)
        .map(std::result::Result::unwrap)
}

/// Formats integers using the `fmt` string given
///
/// Formatting options (All options passed via `format(fmt="<option>")`:
///     * `:X` Upper hex (`42` => `2A`)
///     * `:x` Lower hex (`42` => `2a`)
///     * `:o` Octal (`42` => `52`)
///     * `:b` Binary (`42` => `101010`)
///     * `:E` Upper exponent (`42.0` => `4.2E1`)
///     * `:e` Lower exponent (`42.0` => `4.2e1`)
///
/// Additionally, the `#` modifier can be passed to some formatters as well:
///     * `:#X` Upper hex (`42` => `0x2A`)
///     * `:#x` Lower hex (`42` => `0x2a`)
///     * `:#o` Octal (`42` => `0o52`)
///     * `:#b` Binary (`42` => `0b101010`)
pub fn format(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
    let fmt = if let Some(fmt) = args.get("fmt") {
        try_get_value!("format", "fmt", String, fmt)
    } else {
        return Err(Error::msg("Filter `format` expected an arg called `fmt`"));
    };
    let mut chars = fmt.chars();

    if !matches!(chars.next(), Some(':')) {
        return Err(Error::msg("Format specifiers for the `format` filter must start with `:`"));
    }

    let mut spec = chars.next().ok_or_else(|| {
        Error::msg("Format specifiers for the `format` filter must have more than one character")
    })?;

    let alternative = if spec == '#' {
        spec = chars.next().ok_or_else(|| {
            Error::msg("Format strings for the `format` filter with a modifier must have a format specifier")
        })?;
        true
    } else {
        false
    };

    macro_rules! unwrap_integers {
        ($val:expr, if $alt:ident { $alt_fmt:expr } else { $fmt:expr }, $err:expr) => {
            if let Some(uint) = $val.as_u64() {
                if $alt {
                    format!($alt_fmt, uint)
                } else {
                    format!($fmt, uint)
                }
            } else if let Some(int) = $val.as_i64() {
                if $alt {
                    format!($alt_fmt, int)
                } else {
                    format!($fmt, int)
                }
            } else {
                return Err($err);
            }
        };
    }

    let value = match spec {
        'X' => unwrap_integers!(
            value,
            if alternative { "{:#X}" } else { "{:X}" },
            Error::msg("`:X` only takes integer values")
        ),
        'x' => unwrap_integers!(
            value,
            if alternative { "{:#x}" } else { "{:x}" },
            Error::msg("`:x` only takes integer values")
        ),
        'o' => unwrap_integers!(
            value,
            if alternative { "{:#o}" } else { "{:o}" },
            Error::msg("`:o` only takes integer values")
        ),
        'b' => unwrap_integers!(
            value,
            if alternative { "{:#b}" } else { "{:b}" },
            Error::msg("`:b` only takes integer values")
        ),

        'E' => {
            let float = try_get_value!("format", "value", f64, value);
            format!("{:E}", float)
        }
        'e' => {
            let float = try_get_value!("format", "value", f64, value);
            format!("{:e}", float)
        }

        unrecognized => {
            return Err(Error::msg(format!("Unrecognized format specifier: `:{}`", unrecognized)))
        }
    };

    Ok(Value::String(value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::value::to_value;
    use std::collections::HashMap;

    #[test]
    fn test_pluralize_single() {
        let result = pluralize(&to_value(1).unwrap(), &HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("").unwrap());
    }

    #[test]
    fn test_pluralize_multiple() {
        let result = pluralize(&to_value(2).unwrap(), &HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("s").unwrap());
    }

    #[test]
    fn test_pluralize_zero() {
        let result = pluralize(&to_value(0).unwrap(), &HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("s").unwrap());
    }

    #[test]
    fn test_pluralize_multiple_custom_plural() {
        let mut args = HashMap::new();
        args.insert("plural".to_string(), to_value("es").unwrap());
        let result = pluralize(&to_value(2).unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("es").unwrap());
    }

    #[test]
    fn test_pluralize_multiple_custom_singular() {
        let mut args = HashMap::new();
        args.insert("singular".to_string(), to_value("y").unwrap());
        let result = pluralize(&to_value(1).unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("y").unwrap());
    }

    #[test]
    fn test_round_default() {
        let result = round(&to_value(2.1).unwrap(), &HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(2.0).unwrap());
    }

    #[test]
    fn test_round_default_precision() {
        let mut args = HashMap::new();
        args.insert("precision".to_string(), to_value(2).unwrap());
        let result = round(&to_value(3.15159265359).unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(3.15).unwrap());
    }

    #[test]
    fn test_round_ceil() {
        let mut args = HashMap::new();
        args.insert("method".to_string(), to_value("ceil").unwrap());
        let result = round(&to_value(2.1).unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(3.0).unwrap());
    }

    #[test]
    fn test_round_ceil_precision() {
        let mut args = HashMap::new();
        args.insert("method".to_string(), to_value("ceil").unwrap());
        args.insert("precision".to_string(), to_value(1).unwrap());
        let result = round(&to_value(2.11).unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(2.2).unwrap());
    }

    #[test]
    fn test_round_floor() {
        let mut args = HashMap::new();
        args.insert("method".to_string(), to_value("floor").unwrap());
        let result = round(&to_value(2.1).unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(2.0).unwrap());
    }

    #[test]
    fn test_round_floor_precision() {
        let mut args = HashMap::new();
        args.insert("method".to_string(), to_value("floor").unwrap());
        args.insert("precision".to_string(), to_value(1).unwrap());
        let result = round(&to_value(2.91).unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(2.9).unwrap());
    }

    #[cfg(feature = "builtins")]
    #[test]
    fn test_filesizeformat() {
        let args = HashMap::new();
        let result = filesizeformat(&to_value(123456789).unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("117.74 MB").unwrap());
    }
}
