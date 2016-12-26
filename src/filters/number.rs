/// Filters operating on numbers
use std::collections::HashMap;

use serde_json::value::{Value, to_value};
use humansize::{FileSize, file_size_opts};
use chrono::{NaiveDateTime};

use errors::Result;


/// Returns a suffix if the value is greater or equal than 2. Suffix defaults to `s`
pub fn pluralize(value: Value, args: HashMap<String, Value>) -> Result<Value> {
    let num = try_get_value!("pluralize", "value", f64, value);
    let suffix = match args.get("suffix") {
        Some(val) => try_get_value!("pluralize", "suffix", String, val.clone()),
        None => "s".to_string(),
    };

    if num >= 2.0 {
        Ok(to_value(&suffix))
    } else {
        Ok(to_value(&""))
    }
}

/// Returns a rounded number using the `method` arg and `precision` given.
/// `method` defaults to `common` which will round to the nearest number.
/// `ceil` and `floor` are also available as method.
/// `precision` defaults to `0`, meaning it will round to an integer
pub fn round(value: Value, args: HashMap<String, Value>) -> Result<Value> {
    let num = try_get_value!("round", "value", f64, value);
    let method = match args.get("method") {
        Some(val) => try_get_value!("round", "method", String, val.clone()),
        None => "common".to_string(),
    };
    let precision = match args.get("precision") {
        Some(val) => try_get_value!("round", "precision", i32, val.clone()),
        None => 0,
    };
    let multiplier = if precision == 0 { 1.0 } else { 10.0_f64.powi(precision) } ;

    match method.as_ref() {
        "common" => Ok(to_value((multiplier * num).round() / multiplier)),
        "ceil" => Ok(to_value((multiplier * num).ceil() / multiplier)),
        "floor" => Ok(to_value((multiplier * num).floor() / multiplier)),
        _ => bail!(
                "Filter `round` received an incorrect value for arg `method`: got `{:?}`, \
                only common, ceil and floor are allowed",
                method
            )
    }
}


/// Returns a human-readable file size (i.e. '110 MB') from an integer
pub fn filesizeformat(value: Value, _: HashMap<String, Value>) -> Result<Value> {
    let num = try_get_value!("filesizeformat", "value", i64, value);
    num
        .file_size(file_size_opts::CONVENTIONAL)
        .or_else(|_| Err(format!("Filter `filesizeformat` was called on a negative number: {}", num).into()))
        .map(to_value)
}


/// Returns a formatted timestamp according to the given `format` argument.
/// `format` defaults to the ISO 8601 `YYYY-MM-DD` format
/// Time formatting syntax is inspired from strftime and a full reference is available
/// on [chrono docs](https://lifthrasiir.github.io/rust-chrono/chrono/format/strftime/index.html)
pub fn date(value: Value, args: HashMap<String, Value>) -> Result<Value> {
    let timestamp = try_get_value!("date", "value", i64, value);
    let format = match args.get("format") {
        Some(val) => try_get_value!("date", "format", String, val.clone()),
        None => "%Y-%m-%d".to_string(),
    };
    let dt = NaiveDateTime::from_timestamp(timestamp, 0);
    Ok(to_value(&dt.format(&format).to_string()))
}


#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use serde_json::value::to_value;
    use super::*;

    #[test]
    fn test_pluralize_single() {
        let result = pluralize(to_value(1), HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(""));
    }

    #[test]
    fn test_pluralize_multiple() {
        let result = pluralize(to_value(2), HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("s"));
    }

    #[test]
    fn test_pluralize_multiple_custom_suffix() {
        let mut args = HashMap::new();
        args.insert("suffix".to_string(), to_value("es"));
        let result = pluralize(to_value(2), args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("es"));
    }

    #[test]
    fn test_round_default() {
        let result = round(to_value(2.1), HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(2.0));
    }

    #[test]
    fn test_round_default_precision() {
        let mut args = HashMap::new();
        args.insert("precision".to_string(), to_value(2));
        let result = round(to_value(3.15159265359), args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(3.15));
    }

    #[test]
    fn test_round_ceil() {
        let mut args = HashMap::new();
        args.insert("method".to_string(), to_value("ceil"));
        let result = round(to_value(2.1), args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(3.0));
    }

    #[test]
    fn test_round_ceil_precision() {
        let mut args = HashMap::new();
        args.insert("method".to_string(), to_value("ceil"));
        args.insert("precision".to_string(), to_value(1));
        let result = round(to_value(2.11), args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(2.2));
    }

    #[test]
    fn test_round_floor() {
        let mut args = HashMap::new();
        args.insert("method".to_string(), to_value("floor"));
        let result = round(to_value(2.1), args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(2.0));
    }

    #[test]
    fn test_round_floor_precision() {
        let mut args = HashMap::new();
        args.insert("method".to_string(), to_value("floor"));
        args.insert("precision".to_string(), to_value(1));
        let result = round(to_value(2.91), args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(2.9));
    }

    #[test]
    fn test_filesizeformat() {
        let args = HashMap::new();
        let result = filesizeformat(to_value(123456789), args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("117.74 MB"));
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
}
