/// Filters operating on multiple types
use std::collections::HashMap;
use std::iter::FromIterator;

use serde_json::value::{Value, to_value};
use errors::Result;

use chrono::{NaiveDateTime, DateTime, FixedOffset};

// Returns the number of items in an array or the number of characters in a string.
// Returns 0 if not an array or string.
pub fn length(value: Value, _: HashMap<String, Value>) -> Result<Value> {
    match value {
        Value::Array(arr) => Ok(to_value(&arr.len())),
        Value::String(s) => Ok(to_value(&s.chars().count())),
        _ => Ok(to_value(&0)),
    }
}

// Reverses the elements of an array or the characters in a string.
pub fn reverse(value: Value, _: HashMap<String, Value>) -> Result<Value> {
    match value {
        Value::Array(mut arr) => {
            arr.reverse();
            Ok(to_value(&arr))
        }
        Value::String(s) => Ok(to_value(&String::from_iter(s.chars().rev()))),
        _ => {
            bail!(
                "Filter `reverse` received an incorrect type for arg `value`: got `{:?}` but expected Array|String",
                value
            );
        }
    }
}


/// Returns a formatted time according to the given `format` argument.
/// `format` defaults to the ISO 8601 `YYYY-MM-DD` format.
///
/// Input can be an i64 timestamp (seconds since epoch) or an RFC3339 string
/// (default serialization format for `chrono::DateTime`).
///
/// Time formatting syntax is inspired from strftime and a full reference is available
/// on [chrono docs](https://lifthrasiir.github.io/rust-chrono/chrono/format/strftime/index.html)
pub fn date(value: Value, mut args: HashMap<String, Value>) -> Result<Value> {
    let dt = match value {
        Value::I64(i) => NaiveDateTime::from_timestamp(i, 0),
        Value::U64(u) => NaiveDateTime::from_timestamp(u as i64, 0),
        Value::String(s) => {
            match s.parse::<DateTime<FixedOffset>>() {
                Ok(val) => val.naive_local(),
                Err(_) => bail!("Error parsing `{:?}` as rfc3339 date", s)
            }
        },
        _ => {
            bail!(
                "Filter `date` received an incorrect type for arg `value`: got `{:?}` but expected i64|u64|String",
                value
            );
        }
    };

    let format = match args.remove("format") {
        Some(val) => try_get_value!("date", "format", String, val),
        None => "%Y-%m-%d".to_string(),
    };

    Ok(to_value(&dt.format(&format).to_string()))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use serde_json::value::to_value;
    use super::*;
    use chrono::{DateTime, Local};

    #[test]
    fn test_length_vec() {
        let result = length(to_value(&vec![1, 2, 3, 4]), HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(&4));
    }

    #[test]
    fn test_length_str() {
        let result = length(to_value(&"Hello World"), HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(&11));
    }

    #[test]
    fn test_length_str_nonascii() {
        let result = length(to_value(&"日本語"), HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(&3));
    }

    #[test]
    fn test_length_num() {
        let result = length(to_value(&15), HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(&0));
    }

    #[test]
    fn test_reverse_vec() {
        let result = reverse(to_value(&vec![1, 2, 3, 4]), HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(&vec![4, 3, 2, 1]));
    }

    #[test]
    fn test_reverse_str() {
        let result = reverse(to_value(&"Hello World"), HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(&"dlroW olleH"));
    }

    #[test]
    fn test_reverse_num() {
        let result = reverse(to_value(&1.23), HashMap::new());
        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap().description(),
            "Filter `reverse` received an incorrect type for arg `value`: got `1.23` but expected Array|String"
        );
    }

    #[test]
    fn test_date_default() {
        let args = HashMap::new();
        let result = date(to_value(1482720453), args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("2016-12-26"));
    }

    #[test]
    fn test_date_custom_format() {
        let mut args = HashMap::new();
        args.insert("format".to_string(), to_value("%Y-%m-%d %H:%M"));
        let result = date(to_value(1482720453), args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("2016-12-26 02:47"));
    }

    #[test]
    fn test_date_rfc3339() {
        let args = HashMap::new();
        let dt: DateTime<Local> = Local::now();
        let result = date(to_value(dt.to_rfc3339()), args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(dt.format("%Y-%m-%d").to_string()));
    }
}
