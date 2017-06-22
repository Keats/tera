/// Filters operating on multiple types
use std::collections::HashMap;
use std::iter::FromIterator;

use serde_json::value::{Value, to_value};
use errors::Result;

use chrono::{NaiveDateTime, NaiveDate, DateTime, FixedOffset, Utc};

// Returns the number of items in an array or the number of characters in a string.
// Returns 0 if not an array or string.
pub fn length(value: Value, _: HashMap<String, Value>) -> Result<Value> {
    match value {
        Value::Array(arr) => Ok(to_value(&arr.len()).unwrap()),
        Value::String(s) => Ok(to_value(&s.chars().count()).unwrap()),
        _ => Ok(to_value(0).unwrap()),
    }
}

// Reverses the elements of an array or the characters in a string.
pub fn reverse(value: Value, _: HashMap<String, Value>) -> Result<Value> {
    match value {
        Value::Array(mut arr) => {
            arr.reverse();
            Ok(to_value(&arr)?)
        }
        Value::String(s) => Ok(to_value(&String::from_iter(s.chars().rev()))?),
        _ => {
            bail!(
                "Filter `reverse` received an incorrect type for arg `value`: got `{}` but expected Array|String",
                value.to_string()
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
/// a full reference for the time formatting syntax is available
/// on [chrono docs](https://lifthrasiir.github.io/rust-chrono/chrono/format/strftime/index.html)
pub fn date(value: Value, mut args: HashMap<String, Value>) -> Result<Value> {
    let format = match args.remove("format") {
        Some(val) => try_get_value!("date", "format", String, val),
        None => "%Y-%m-%d".to_string(),
    };

    match value {
        Value::Number(n) => {
            match n.as_i64() {
                Some(i) => Ok(to_value(&NaiveDateTime::from_timestamp(i, 0).format(&format).to_string())?),
                None => bail!("Filter `date` was invoked on a float: {:?}", n)
            }
        },
        Value::String(s) => {
            if s.contains('T') {
                match s.parse::<DateTime<FixedOffset>>() {
                    Ok(val) => Ok(to_value(&val.format(&format).to_string())?),
                    Err(_) => bail!("Error parsing `{:?}` as rfc3339 date", s)
                }
            } else {
                match NaiveDate::parse_from_str(&s, "%Y-%m-%d") {
                    Ok(val) => {
                        Ok(to_value(&DateTime::<Utc>::from_utc(val.and_hms(0, 0, 0), Utc).format(&format).to_string())?)
                    },
                    Err(_) => bail!("Error parsing `{:?}` as YYYY-MM-DD date", s)
                }
            }
        },
        _ => {
            bail!(
                "Filter `date` received an incorrect type for arg `value`: got `{:?}` but expected i64|u64|String",
                value
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use serde_json::value::to_value;
    use super::*;
    use chrono::{DateTime, Local};

    #[test]
    fn test_length_vec() {
        let result = length(to_value(&vec![1, 2, 3, 4]).unwrap(), HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(&4).unwrap());
    }

    #[test]
    fn test_length_str() {
        let result = length(to_value(&"Hello World").unwrap(), HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(&11).unwrap());
    }

    #[test]
    fn test_length_str_nonascii() {
        let result = length(to_value(&"日本語").unwrap(), HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(&3).unwrap());
    }

    #[test]
    fn test_length_num() {
        let result = length(to_value(&15).unwrap(), HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(&0).unwrap());
    }

    #[test]
    fn test_reverse_vec() {
        let result = reverse(to_value(&vec![1, 2, 3, 4]).unwrap(), HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(&vec![4, 3, 2, 1]).unwrap());
    }

    #[test]
    fn test_reverse_str() {
        let result = reverse(to_value(&"Hello World").unwrap(), HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(&"dlroW olleH").unwrap());
    }

    #[test]
    fn test_reverse_num() {
        let result = reverse(to_value(&1.23).unwrap(), HashMap::new());
        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap().description(),
            "Filter `reverse` received an incorrect type for arg `value`: got `1.23` but expected Array|String"
        );
    }

    #[test]
    fn test_date_default() {
        let args = HashMap::new();
        let result = date(to_value(1482720453).unwrap(), args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("2016-12-26").unwrap());
    }

    #[test]
    fn test_date_custom_format() {
        let mut args = HashMap::new();
        args.insert("format".to_string(), to_value("%Y-%m-%d %H:%M").unwrap());
        let result = date(to_value(1482720453).unwrap(), args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("2016-12-26 02:47").unwrap());
    }

    #[test]
    fn test_date_rfc3339() {
        let args = HashMap::new();
        let dt: DateTime<Local> = Local::now();
        let result = date(to_value(dt.to_rfc3339()).unwrap(), args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(dt.format("%Y-%m-%d").to_string()).unwrap());
    }

    #[test]
    fn test_date_rfc3339_preserves_timezone() {
        let mut args = HashMap::new();
        args.insert("format".to_string(), to_value("%Y-%m-%d %z").unwrap());
        let result = date(to_value("1996-12-19T16:39:57-08:00").unwrap(), args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("1996-12-19 -0800").unwrap());
    }

    #[test]
    fn test_date_yyyy_mm_dd() {
        let mut args = HashMap::new();
        args.insert("format".to_string(), to_value("%a, %d %b %Y %H:%M:%S %z").unwrap());
        let result = date(to_value("2017-03-05").unwrap(), args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("Sun, 05 Mar 2017 00:00:00 +0000").unwrap());
    }
}
