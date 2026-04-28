use jiff::fmt::temporal::DateTimeParser;
use jiff::tz::TimeZone;
use jiff::{Timestamp, Zoned};
use tera::{Kwargs, Number, State, TeraResult, Value};

static PARSER: DateTimeParser = DateTimeParser::new();

/// Parse a Value (string or integer) into a Zoned datetime.
fn parse_to_zoned(val: &Value, tz: Option<TimeZone>) -> TeraResult<Zoned> {
    let default_tz = tz.unwrap_or(TimeZone::UTC);
    if let Some(s) = val.as_str() {
        PARSER
            .parse_zoned(s)
            .or_else(|_| {
                PARSER
                    .parse_timestamp(s)
                    .map(|t| t.to_zoned(default_tz.clone()))
            })
            .or_else(|_| {
                PARSER
                    .parse_datetime(s)
                    .and_then(|d| d.to_zoned(default_tz.clone()))
            })
            .or_else(|_| PARSER.parse_date(s).and_then(|d| d.to_zoned(default_tz)))
            .map_err(|e| {
                tera::Error::message(format!(
                    "The string {s} cannot be parsed as a valid date: {e}"
                ))
            })
    } else if let Some(Number::Integer(ts)) = val.as_number() {
        let ts = i64::try_from(ts)
            .map_err(|_| tera::Error::message(format!("Invalid timestamp: {ts}")))?;
        Timestamp::new(ts, 0)
            .map(|t| t.to_zoned(default_tz))
            .map_err(|e| tera::Error::message(format!("Invalid timestamp: {e}")))
    } else {
        Err(tera::Error::message(format!(
            "Invalid value: expected a string or integer, got {}",
            val.name()
        )))
    }
}

/// Returns the current datetime.
/// You can pass an optional `timezone` name. Defaults to UTC if not provided.
///
/// ```text
/// {{ now() }}
/// {{ now(timezone="America/New_York") }}
/// ```
pub fn now(kwargs: Kwargs, _: &State) -> TeraResult<Value> {
    let tz_str = kwargs.get::<&str>("timezone")?.unwrap_or("UTC");
    let timezone = TimeZone::get(tz_str)
        .map_err(|_| tera::Error::message(format!("Unknown timezone: {tz_str}")))?;
    let now = Zoned::now().with_time_zone(timezone);
    Ok(Value::from(now.to_string()))
}

/// Formats the given value using the given format if it can be parsed as a date/datetime.
/// Takes:
///   1. optional `format` argument, defaulting to `%Y-%m-%d`
///   2. optional `timezone` argument, defaulting to not set
///
/// ```text
/// {{ value | date }}
/// {{ value | date(format="%B %d, %Y") }}
/// {{ timestamp | date(format="%Y-%m-%d %H:%M", timezone="Europe/Paris") }}
/// ```
pub fn date(val: &Value, kwargs: Kwargs, _: &State) -> TeraResult<String> {
    let format = kwargs.get::<&str>("format")?.unwrap_or("%Y-%m-%d");
    let timezone = match kwargs.get::<&str>("timezone")? {
        Some(t) => Some(
            TimeZone::get(t).map_err(|_| tera::Error::message(format!("Unknown timezone: {t}")))?,
        ),
        None => None,
    };

    let mut zoned = parse_to_zoned(val, timezone.clone())?;
    if let Some(tz) = timezone {
        zoned = zoned.with_time_zone(tz);
    }

    jiff::fmt::strtime::format(format, &zoned)
        .map_err(|e| tera::Error::message(format!("Invalid date format `{format}`: {e}")))
}

/// Tests whether a date is before another date.
/// Errors if one of the values cannot be parsed as a date.
/// Takes an optional `inclusive` argument defaulting to false to make this test be `<=` instead of `<`.
///
/// ```text
/// {% if date is before(other="2024-06-01") %}...{% endif %}
/// {% if date is before(other=other_date, inclusive=true) %}...{% endif %}
/// ```
pub fn is_before(val: &Value, kwargs: Kwargs, _: &State) -> TeraResult<bool> {
    let other = kwargs.must_get::<&Value>("other")?;
    let inclusive = kwargs.get::<bool>("inclusive")?.unwrap_or(false);
    let val_zoned = parse_to_zoned(val, None)?;
    let other_zoned = parse_to_zoned(other, None)?;
    if inclusive {
        Ok(val_zoned <= other_zoned)
    } else {
        Ok(val_zoned < other_zoned)
    }
}

/// Tests whether a date is after another date.
/// Errors if one of the values cannot be parsed as a date.
/// Takes an optional `inclusive` argument defaulting to false to make this test be `>=` instead of `>`.
///
/// ```text
/// {% if date is after(other="2024-01-01") %}...{% endif %}
/// {% if date is after(other=other_date, inclusive=true) %}...{% endif %}
/// ```
pub fn is_after(val: &Value, kwargs: Kwargs, _: &State) -> TeraResult<bool> {
    let other = kwargs.must_get::<&Value>("other")?;
    let inclusive = kwargs.get::<bool>("inclusive")?.unwrap_or(false);
    let val_zoned = parse_to_zoned(val, None)?;
    let other_zoned = parse_to_zoned(other, None)?;
    if inclusive {
        Ok(val_zoned >= other_zoned)
    } else {
        Ok(val_zoned > other_zoned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tera::value::Map;
    use tera::{Context, Kwargs, State};

    // copy from https://github.com/Keats/tera/blob/master/src/builtins/filters/common.rs#L326
    #[test]
    fn test_ok_date() {
        let inputs = vec![
            (Value::from(1482720453), None, None, "2016-12-26"),
            (
                Value::from(1482720453),
                Some("%Y-%m-%d %H:%M"),
                None,
                "2016-12-26 02:47",
            ),
            // RFC3339
            (
                Value::from("1985-04-12T23:20:50.52Z"),
                None,
                None,
                "1985-04-12",
            ),
            // timezones are preserved
            (
                Value::from("1996-12-19T16:39:57[-08:00]"),
                Some("%Y-%m-%d %z"),
                None,
                "1996-12-19 -0800",
            ),
            // simple date
            (
                Value::from("2017-03-05"),
                Some("%a, %d %b %Y %H:%M:%S %z"),
                None,
                "Sun, 05 Mar 2017 00:00:00 +0000",
            ),
            // naive datetime
            (
                Value::from("2017-03-05T00:00:00.602"),
                Some("%a, %d %b %Y %H:%M:%S"),
                None,
                "Sun, 05 Mar 2017 00:00:00",
            ),
            // with a timezone
            (
                Value::from("2019-09-19T01:48:44.581Z"),
                None,
                Some("America/New_York"),
                "2019-09-18",
            ),
            (
                Value::from(1648252203),
                None,
                Some("Europe/Berlin"),
                "2022-03-26",
            ),
        ];

        for (value, format, timezone, expected) in inputs {
            let mut map = Map::new();
            if let Some(f) = format {
                map.insert("format".into(), f.into());
            }
            if let Some(tz) = timezone {
                map.insert("timezone".into(), tz.into());
            }
            let kwargs = Kwargs::new(Arc::new(map));
            let ctx = Context::new();
            let res = date(&value, kwargs, &State::new(&ctx)).unwrap();
            assert_eq!(expected, res);
        }
    }

    #[test]
    fn test_bad_date_call() {
        let inputs = vec![
            (Value::from(1482720453), Some("%1"), None),
            (Value::from(1482720453), Some("%+S"), None),
            (
                Value::from("2019-09-19T01:48:44.581Z"),
                Some("%+S"),
                Some("Narnia"),
            ),
        ];

        for (value, format, timezone) in inputs {
            let mut map = Map::new();
            if let Some(f) = format {
                map.insert("format".into(), f.into());
            }
            if let Some(tz) = timezone {
                map.insert("timezone".into(), tz.into());
            }
            let kwargs = Kwargs::new(Arc::new(map));
            let ctx = Context::new();
            let res = date(&value, kwargs, &State::new(&ctx));
            println!("{res:?}");
            assert!(res.is_err());
        }
    }

    #[test]
    fn test_register() {
        let mut tera = tera::Tera::default();
        tera.register_filter("date", date);
        tera.register_test("before", is_before);
        tera.register_test("after", is_after);
        tera.register_function("now", now);
    }

    #[test]
    fn test_is_before() {
        let ctx = Context::new();
        let state = State::new(&ctx);

        // (val, other, inclusive, expected)
        let cases: Vec<(Value, Value, bool, bool)> = vec![
            (Value::from(500), Value::from(1000), false, true),
            (Value::from(1000), Value::from(500), false, false),
            (
                Value::from("2024-01-01"),
                Value::from("2024-06-01"),
                false,
                true,
            ),
            (
                Value::from("2024-06-01"),
                Value::from("2024-01-01"),
                false,
                false,
            ),
            (Value::from(1000), Value::from("2020-01-01"), false, true), // mixed formats
            (
                Value::from("2024-01-01"),
                Value::from("2024-01-01"),
                false,
                false,
            ), // equal
            (
                Value::from("2024-01-01"),
                Value::from("2024-01-01"),
                true,
                true,
            ), // equal + inclusive
        ];

        for (val, other, inclusive, expected) in cases {
            let mut map = Map::new();
            map.insert("other".into(), other);
            if inclusive {
                map.insert("inclusive".into(), Value::from(true));
            }
            let kwargs = Kwargs::new(Arc::new(map));
            assert_eq!(is_before(&val, kwargs, &state).unwrap(), expected);
        }
    }

    #[test]
    fn test_is_after() {
        let ctx = Context::new();
        let state = State::new(&ctx);

        // (val, other, inclusive, expected)
        let cases: Vec<(Value, Value, bool, bool)> = vec![
            (Value::from(1000), Value::from(500), false, true),
            (Value::from(500), Value::from(1000), false, false),
            (
                Value::from("2024-06-01"),
                Value::from("2024-01-01"),
                false,
                true,
            ),
            (
                Value::from("2024-01-01"),
                Value::from("2024-06-01"),
                false,
                false,
            ),
            (Value::from("2020-01-01"), Value::from(1000), false, true), // mixed formats
            (
                Value::from("2024-01-01"),
                Value::from("2024-01-01"),
                false,
                false,
            ), // equal
            (
                Value::from("2024-01-01"),
                Value::from("2024-01-01"),
                true,
                true,
            ), // equal + inclusive
        ];

        for (val, other, inclusive, expected) in cases {
            let mut map = Map::new();
            map.insert("other".into(), other);
            if inclusive {
                map.insert("inclusive".into(), Value::from(true));
            }
            let kwargs = Kwargs::new(Arc::new(map));
            assert_eq!(is_after(&val, kwargs, &state).unwrap(), expected);
        }
    }
}
