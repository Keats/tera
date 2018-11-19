use std::collections::HashMap;

use chrono::prelude::*;
use serde_json::value::{from_value, to_value, Value};

use errors::{Error, Result};

/// The global function type definition
pub trait Function: Sync + Send {
    /// The global function type definition
    fn call(&self, args: &HashMap<String, Value>) -> Result<Value>;
}

impl<F> Function for F
where
    F: Fn(&HashMap<String, Value>) -> Result<Value> + Sync + Send,
{
    fn call(&self, args: &HashMap<String, Value>) -> Result<Value> {
        self(args)
    }
}

pub fn range(args: &HashMap<String, Value>) -> Result<Value> {
    let start = match args.get("start") {
        Some(val) => match from_value::<usize>(val.clone()) {
            Ok(v) => v,
            Err(_) => {
                return Err(Error::msg(format!(
                    "Global function `range` received start={} but `start` can only be a number",
                    val
                )))
            }
        },
        None => 0,
    };
    let step_by = match args.get("step_by") {
        Some(val) => match from_value::<usize>(val.clone()) {
            Ok(v) => v,
            Err(_) => {
                return Err(Error::msg(format!(
                    "Global function `range` received step_by={} but `step` can only be a number",
                    val
                )))
            }
        },
        None => 1,
    };
    let end = match args.get("end") {
        Some(val) => match from_value::<usize>(val.clone()) {
            Ok(v) => v,
            Err(_) => {
                return Err(Error::msg(format!(
                    "Global function `range` received end={} but `end` can only be a number",
                    val
                )))
            }
        },
        None => {
            return Err(Error::msg("Global function `range` was called without a `end` argument"))
        }
    };

    if start > end {
        return Err(Error::msg("Global function `range` was called without a `start` argument greater than the `end` one"));
    }

    let mut i = start;
    let mut res = vec![];
    while i < end {
        res.push(i);
        i += step_by;
    }
    Ok(to_value(res).unwrap())
}

pub fn now(args: &HashMap<String, Value>) -> Result<Value> {
    let utc = match args.get("utc") {
        Some(val) => match from_value::<bool>(val.clone()) {
            Ok(v) => v,
            Err(_) => {
                return Err(Error::msg(format!(
                    "Global function `now` received utc={} but `utc` can only be a boolean",
                    val
                )))
            }
        },
        None => false,
    };
    let timestamp = match args.get("timestamp") {
        Some(val) => match from_value::<bool>(val.clone()) {
            Ok(v) => v,
            Err(_) => return Err(Error::msg(format!(
                "Global function `now` received timestamp={} but `timestamp` can only be a boolean",
                val
            ))),
        },
        None => false,
    };

    if utc {
        let datetime = Utc::now();
        if timestamp {
            return Ok(to_value(datetime.timestamp()).unwrap());
        }
        Ok(to_value(datetime.to_rfc3339()).unwrap())
    } else {
        let datetime = Local::now();
        if timestamp {
            return Ok(to_value(datetime.timestamp()).unwrap());
        }
        Ok(to_value(datetime.to_rfc3339()).unwrap())
    }
}

pub fn throw(args: &HashMap<String, Value>) -> Result<Value> {
    match args.get("message") {
        Some(val) => match from_value::<String>(val.clone()) {
            Ok(v) => Err(Error::msg(v)),
            Err(_) => Err(Error::msg(format!(
                "Global function `throw` received message={} but `message` can only be a string",
                val
            ))),
        },
        None => Err(Error::msg("Global function `throw` was called without a `message` argument")),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serde_json::value::to_value;

    use super::*;

    #[test]
    fn range_default() {
        let mut args = HashMap::new();
        args.insert("end".to_string(), to_value(5).unwrap());

        let res = range(&args).unwrap();
        assert_eq!(res, to_value(vec![0, 1, 2, 3, 4]).unwrap());
    }

    #[test]
    fn range_start() {
        let mut args = HashMap::new();
        args.insert("end".to_string(), to_value(5).unwrap());
        args.insert("start".to_string(), to_value(1).unwrap());

        let res = range(&args).unwrap();
        assert_eq!(res, to_value(vec![1, 2, 3, 4]).unwrap());
    }

    #[test]
    fn range_start_greater_than_end() {
        let mut args = HashMap::new();
        args.insert("end".to_string(), to_value(5).unwrap());
        args.insert("start".to_string(), to_value(6).unwrap());

        assert!(range(&args).is_err());
    }

    #[test]
    fn range_step_by() {
        let mut args = HashMap::new();
        args.insert("end".to_string(), to_value(10).unwrap());
        args.insert("step_by".to_string(), to_value(2).unwrap());

        let res = range(&args).unwrap();
        assert_eq!(res, to_value(vec![0, 2, 4, 6, 8]).unwrap());
    }

    #[test]
    fn now_default() {
        let args = HashMap::new();

        let res = now(&args).unwrap();
        assert!(res.is_string());
        assert!(res.as_str().unwrap().contains("T"));
    }

    #[test]
    fn now_datetime_utc() {
        let mut args = HashMap::new();
        args.insert("utc".to_string(), to_value(true).unwrap());

        let res = now(&args).unwrap();
        assert!(res.is_string());
        let val = res.as_str().unwrap();
        println!("{}", val);
        assert!(val.contains("T"));
        assert!(val.contains("+00:00"));
    }

    #[test]
    fn now_timestamp() {
        let mut args = HashMap::new();
        args.insert("timestamp".to_string(), to_value(true).unwrap());

        let res = now(&args).unwrap();
        assert!(res.is_number());
    }

    #[test]
    fn throw_errors_with_message() {
        let mut args = HashMap::new();
        args.insert("message".to_string(), to_value("Hello").unwrap());

        let res = throw(&args);
        assert!(res.is_err());
        let err = res.unwrap_err();
        assert_eq!(err.to_string(), "Hello");
    }
}
