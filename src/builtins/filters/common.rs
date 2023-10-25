/// Filters operating on multiple types
use std::collections::HashMap;
#[cfg(feature = "date-locale")]
use std::convert::TryFrom;
use std::iter::FromIterator;

use crate::errors::{Error, Result};
use crate::utils::render_to_string;
#[cfg(feature = "builtins")]
use chrono::{
    format::{Item, StrftimeItems},
    DateTime, FixedOffset, NaiveDate, NaiveDateTime, TimeZone, Utc,
};
#[cfg(feature = "builtins")]
use chrono_tz::Tz;
use serde_json::value::{to_value, Value};
use serde_json::{to_string, to_string_pretty};

use crate::context::ValueRender;

// Returns the number of items in an array or an object, or the number of characters in a string.
pub fn length(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    match value {
        Value::Array(arr) => Ok(to_value(arr.len()).unwrap()),
        Value::Object(m) => Ok(to_value(m.len()).unwrap()),
        Value::String(s) => Ok(to_value(s.chars().count()).unwrap()),
        _ => Err(Error::msg(
            "Filter `length` was used on a value that isn't an array, an object, or a string.",
        )),
    }
}

// Reverses the elements of an array or the characters in a string.
pub fn reverse(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    match value {
        Value::Array(arr) => {
            let mut rev = arr.clone();
            rev.reverse();
            to_value(&rev).map_err(Error::json)
        }
        Value::String(s) => to_value(String::from_iter(s.chars().rev())).map_err(Error::json),
        _ => Err(Error::msg(format!(
            "Filter `reverse` received an incorrect type for arg `value`: \
             got `{}` but expected Array|String",
            value
        ))),
    }
}

// Encodes a value of any type into json, optionally `pretty`-printing it
// `pretty` can be true to enable pretty-print, or omitted for compact printing
pub fn json_encode(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
    let pretty = args.get("pretty").and_then(Value::as_bool).unwrap_or(false);

    if pretty {
        to_string_pretty(&value).map(Value::String).map_err(Error::json)
    } else {
        to_string(&value).map(Value::String).map_err(Error::json)
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
#[cfg(feature = "builtins")]
pub fn date(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
    let format = match args.get("format") {
        Some(val) => try_get_value!("date", "format", String, val),
        None => "%Y-%m-%d".to_string(),
    };

    let items: Vec<Item> =
        StrftimeItems::new(&format).filter(|item| matches!(item, Item::Error)).collect();
    if !items.is_empty() {
        return Err(Error::msg(format!("Invalid date format `{}`", format)));
    }

    let timezone = match args.get("timezone") {
        Some(val) => {
            let timezone = try_get_value!("date", "timezone", String, val);
            match timezone.parse::<Tz>() {
                Ok(timezone) => Some(timezone),
                Err(_) => {
                    return Err(Error::msg(format!("Error parsing `{}` as a timezone", timezone)))
                }
            }
        }
        None => None,
    };

    #[cfg(feature = "date-locale")]
    let formatted = {
        let locale = match args.get("locale") {
            Some(val) => {
                let locale = try_get_value!("date", "locale", String, val);
                chrono::Locale::try_from(locale.as_str())
                    .map_err(|_| Error::msg(format!("Error parsing `{}` as a locale", locale)))?
            }
            None => chrono::Locale::POSIX,
        };
        match value {
            Value::Number(n) => match n.as_i64() {
                Some(i) => {
                    let date = NaiveDateTime::from_timestamp_opt(i, 0).expect(
                        "out of bound seconds should not appear, as we set nanoseconds to zero",
                    );
                    match timezone {
                        Some(timezone) => {
                            timezone.from_utc_datetime(&date).format_localized(&format, locale)
                        }
                        None => date.format(&format),
                    }
                }
                None => {
                    return Err(Error::msg(format!("Filter `date` was invoked on a float: {}", n)))
                }
            },
            Value::String(s) => {
                if s.contains('T') {
                    match s.parse::<DateTime<FixedOffset>>() {
                        Ok(val) => match timezone {
                            Some(timezone) => {
                                val.with_timezone(&timezone).format_localized(&format, locale)
                            }
                            None => val.format_localized(&format, locale),
                        },
                        Err(_) => match s.parse::<NaiveDateTime>() {
                            Ok(val) => DateTime::<Utc>::from_naive_utc_and_offset(val, Utc)
                                .format_localized(&format, locale),
                            Err(_) => {
                                return Err(Error::msg(format!(
                                    "Error parsing `{:?}` as rfc3339 date or naive datetime",
                                    s
                                )));
                            }
                        },
                    }
                } else {
                    match NaiveDate::parse_from_str(s, "%Y-%m-%d") {
                        Ok(val) => DateTime::<Utc>::from_naive_utc_and_offset(
                            val.and_hms_opt(0, 0, 0).expect(
                                "out of bound should not appear, as we set the time to zero",
                            ),
                            Utc,
                        )
                        .format_localized(&format, locale),
                        Err(_) => {
                            return Err(Error::msg(format!(
                                "Error parsing `{:?}` as YYYY-MM-DD date",
                                s
                            )));
                        }
                    }
                }
            }
            _ => {
                return Err(Error::msg(format!(
                    "Filter `date` received an incorrect type for arg `value`: \
                     got `{:?}` but expected i64|u64|String",
                    value
                )));
            }
        }
    };

    #[cfg(not(feature = "date-locale"))]
    let formatted = match value {
        Value::Number(n) => match n.as_i64() {
            Some(i) => {
                let date = NaiveDateTime::from_timestamp_opt(i, 0).expect(
                    "out of bound seconds should not appear, as we set nanoseconds to zero",
                );
                match timezone {
                    Some(timezone) => timezone.from_utc_datetime(&date).format(&format),
                    None => date.format(&format),
                }
            }
            None => return Err(Error::msg(format!("Filter `date` was invoked on a float: {}", n))),
        },
        Value::String(s) => {
            if s.contains('T') {
                match s.parse::<DateTime<FixedOffset>>() {
                    Ok(val) => match timezone {
                        Some(timezone) => val.with_timezone(&timezone).format(&format),
                        None => val.format(&format),
                    },
                    Err(_) => match s.parse::<NaiveDateTime>() {
                        Ok(val) => {
                            DateTime::<Utc>::from_naive_utc_and_offset(val, Utc).format(&format)
                        }
                        Err(_) => {
                            return Err(Error::msg(format!(
                                "Error parsing `{:?}` as rfc3339 date or naive datetime",
                                s
                            )));
                        }
                    },
                }
            } else {
                match NaiveDate::parse_from_str(s, "%Y-%m-%d") {
                    Ok(val) => DateTime::<Utc>::from_naive_utc_and_offset(
                        val.and_hms_opt(0, 0, 0)
                            .expect("out of bound should not appear, as we set the time to zero"),
                        Utc,
                    )
                    .format(&format),
                    Err(_) => {
                        return Err(Error::msg(format!(
                            "Error parsing `{:?}` as YYYY-MM-DD date",
                            s
                        )));
                    }
                }
            }
        }
        _ => {
            return Err(Error::msg(format!(
                "Filter `date` received an incorrect type for arg `value`: \
                 got `{:?}` but expected i64|u64|String",
                value
            )));
        }
    };

    to_value(formatted.to_string()).map_err(Error::json)
}

// Returns the given value as a string.
pub fn as_str(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let value =
        render_to_string(|| format!("as_str for value of kind {}", value), |w| value.render(w))?;
    to_value(value).map_err(Error::json)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "builtins")]
    use chrono::{DateTime, Local};
    use serde_json;
    use serde_json::value::to_value;
    use std::collections::HashMap;

    #[test]
    fn as_str_object() {
        let map: HashMap<String, String> = HashMap::new();
        let result = as_str(&to_value(map).unwrap(), &HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("[object]").unwrap());
    }

    #[test]
    fn as_str_vec() {
        let result = as_str(&to_value(vec![1, 2, 3, 4]).unwrap(), &HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("[1, 2, 3, 4]").unwrap());
    }

    #[test]
    fn length_vec() {
        let result = length(&to_value(vec![1, 2, 3, 4]).unwrap(), &HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(4).unwrap());
    }

    #[test]
    fn length_object() {
        let mut map: HashMap<String, String> = HashMap::new();
        map.insert("foo".to_string(), "bar".to_string());
        let result = length(&to_value(&map).unwrap(), &HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(1).unwrap());
    }

    #[test]
    fn length_str() {
        let result = length(&to_value("Hello World").unwrap(), &HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(11).unwrap());
    }

    #[test]
    fn length_str_nonascii() {
        let result = length(&to_value("日本語").unwrap(), &HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(3).unwrap());
    }

    #[test]
    fn length_num() {
        let result = length(&to_value(15).unwrap(), &HashMap::new());
        assert!(result.is_err());
    }

    #[test]
    fn reverse_vec() {
        let result = reverse(&to_value(vec![1, 2, 3, 4]).unwrap(), &HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(vec![4, 3, 2, 1]).unwrap());
    }

    #[test]
    fn reverse_str() {
        let result = reverse(&to_value("Hello World").unwrap(), &HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("dlroW olleH").unwrap());
    }

    #[test]
    fn reverse_num() {
        let result = reverse(&to_value(1.23).unwrap(), &HashMap::new());
        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap().to_string(),
            "Filter `reverse` received an incorrect type for arg `value`: got `1.23` but expected Array|String"
        );
    }

    #[cfg(feature = "builtins")]
    #[test]
    fn date_default() {
        let args = HashMap::new();
        let result = date(&to_value(1482720453).unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("2016-12-26").unwrap());
    }

    #[cfg(feature = "builtins")]
    #[test]
    fn date_custom_format() {
        let mut args = HashMap::new();
        args.insert("format".to_string(), to_value("%Y-%m-%d %H:%M").unwrap());
        let result = date(&to_value(1482720453).unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("2016-12-26 02:47").unwrap());
    }

    // https://zola.discourse.group/t/can-i-generate-a-random-number-within-a-range/238?u=keats
    // https://github.com/chronotope/chrono/issues/47
    #[cfg(feature = "builtins")]
    #[test]
    fn date_errors_on_incorrect_format() {
        let mut args = HashMap::new();
        args.insert("format".to_string(), to_value("%2f").unwrap());
        let result = date(&to_value(1482720453).unwrap(), &args);
        assert!(result.is_err());
    }

    #[cfg(feature = "builtins")]
    #[test]
    fn date_rfc3339() {
        let args = HashMap::new();
        let dt: DateTime<Local> = Local::now();
        let result = date(&to_value(dt.to_rfc3339()).unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(dt.format("%Y-%m-%d").to_string()).unwrap());
    }

    #[cfg(feature = "builtins")]
    #[test]
    fn date_rfc3339_preserves_timezone() {
        let mut args = HashMap::new();
        args.insert("format".to_string(), to_value("%Y-%m-%d %z").unwrap());
        let result = date(&to_value("1996-12-19T16:39:57-08:00").unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("1996-12-19 -0800").unwrap());
    }

    #[cfg(feature = "builtins")]
    #[test]
    fn date_yyyy_mm_dd() {
        let mut args = HashMap::new();
        args.insert("format".to_string(), to_value("%a, %d %b %Y %H:%M:%S %z").unwrap());
        let result = date(&to_value("2017-03-05").unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("Sun, 05 Mar 2017 00:00:00 +0000").unwrap());
    }

    #[cfg(feature = "builtins")]
    #[test]
    fn date_from_naive_datetime() {
        let mut args = HashMap::new();
        args.insert("format".to_string(), to_value("%a, %d %b %Y %H:%M:%S").unwrap());
        let result = date(&to_value("2017-03-05T00:00:00.602").unwrap(), &args);
        println!("{:?}", result);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("Sun, 05 Mar 2017 00:00:00").unwrap());
    }

    // https://github.com/getzola/zola/issues/1279
    #[cfg(feature = "builtins")]
    #[test]
    fn date_format_doesnt_panic() {
        let mut args = HashMap::new();
        args.insert("format".to_string(), to_value("%+S").unwrap());
        let result = date(&to_value("2017-01-01T00:00:00").unwrap(), &args);
        assert!(result.is_ok());
    }

    #[cfg(feature = "builtins")]
    #[test]
    fn date_with_timezone() {
        let mut args = HashMap::new();
        args.insert("timezone".to_string(), to_value("America/New_York").unwrap());
        let result = date(&to_value("2019-09-19T01:48:44.581Z").unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("2019-09-18").unwrap());
    }

    #[cfg(feature = "builtins")]
    #[test]
    fn date_with_invalid_timezone() {
        let mut args = HashMap::new();
        args.insert("timezone".to_string(), to_value("Narnia").unwrap());
        let result = date(&to_value("2019-09-19T01:48:44.581Z").unwrap(), &args);
        assert!(result.is_err());
        assert_eq!(result.err().unwrap().to_string(), "Error parsing `Narnia` as a timezone");
    }

    #[cfg(feature = "builtins")]
    #[test]
    fn date_timestamp() {
        let mut args = HashMap::new();
        args.insert("format".to_string(), to_value("%Y-%m-%d").unwrap());
        let result = date(&to_value(1648302603).unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("2022-03-26").unwrap());
    }

    #[cfg(feature = "builtins")]
    #[test]
    fn date_timestamp_with_timezone() {
        let mut args = HashMap::new();
        args.insert("timezone".to_string(), to_value("Europe/Berlin").unwrap());
        let result = date(&to_value(1648252203).unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("2022-03-26").unwrap());
    }

    #[cfg(feature = "date-locale")]
    #[test]
    fn date_timestamp_with_timezone_and_locale() {
        let mut args = HashMap::new();
        args.insert("format".to_string(), to_value("%A %-d %B").unwrap());
        args.insert("timezone".to_string(), to_value("Europe/Paris").unwrap());
        args.insert("locale".to_string(), to_value("fr_FR").unwrap());
        let result = date(&to_value(1659817310).unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("samedi 6 août").unwrap());
    }

    #[cfg(feature = "date-locale")]
    #[test]
    fn date_with_invalid_locale() {
        let mut args = HashMap::new();
        args.insert("locale".to_string(), to_value("xx_XX").unwrap());
        let result = date(&to_value("2019-09-19T01:48:44.581Z").unwrap(), &args);
        assert!(result.is_err());
        assert_eq!(result.err().unwrap().to_string(), "Error parsing `xx_XX` as a locale");
    }

    #[test]
    fn test_json_encode() {
        let args = HashMap::new();
        let result =
            json_encode(&serde_json::from_str("{\"key\": [\"value1\", 2, true]}").unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("{\"key\":[\"value1\",2,true]}").unwrap());
    }

    #[test]
    fn test_json_encode_pretty() {
        let mut args = HashMap::new();
        args.insert("pretty".to_string(), to_value(true).unwrap());
        let result =
            json_encode(&serde_json::from_str("{\"key\": [\"value1\", 2, true]}").unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            to_value("{\n  \"key\": [\n    \"value1\",\n    2,\n    true\n  ]\n}").unwrap()
        );
    }
}
