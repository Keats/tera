use std::collections::HashMap;

use serde_json::value::{Value, to_value, from_value};

use errors::Result;


pub type GlobalFn = fn(args: HashMap<String, Value>) -> Result<Value>;


pub fn range_fn(args: HashMap<String, Value>) -> Result<Value> {
    let start = match args.get("start") {
        Some(val) => match from_value::<usize>(val.clone()) {
            Ok(v) => v,
            Err(_) => bail!("Global function `range` received start={} but `start` can only be a number"),
        },
        None => 0,
    };
    let step_by = match args.get("step_by") {
        Some(val) => match from_value::<usize>(val.clone()) {
            Ok(v) => v,
            Err(_) => bail!("Global function `range` received step_by={} but `step` can only be a number"),
        },
        None => 1,
    };
    let end = match args.get("end") {
        Some(val) => match from_value::<usize>(val.clone()) {
            Ok(v) => v,
            Err(_) => bail!("Global function `range` received end={} but `end` can only be a number"),
        },
        None => bail!("Global function `range` was called without a `end` argument"),
    };

    if start > end {
        bail!("Global function `range` was called without a `start` argument greater than the `end` one");
    }

    let mut i = start;
    let mut res = vec![];
    while i < end {
        res.push(i);
        i += step_by;
    }
    Ok(to_value(res).unwrap())
}


#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serde_json::value::{to_value};

    use super::{range_fn};

    #[test]
    fn test_range_default() {
        let mut args = HashMap::new();
        args.insert("end".to_string(), to_value(5).unwrap());

        let res = range_fn(args).unwrap();
        assert_eq!(res, to_value(vec![0,1,2,3,4]).unwrap());
    }

    #[test]
    fn test_range_start() {
        let mut args = HashMap::new();
        args.insert("end".to_string(), to_value(5).unwrap());
        args.insert("start".to_string(), to_value(1).unwrap());

        let res = range_fn(args).unwrap();
        assert_eq!(res, to_value(vec![1,2,3,4]).unwrap());
    }

    #[test]
    fn test_range_start_greater_than_end() {
        let mut args = HashMap::new();
        args.insert("end".to_string(), to_value(5).unwrap());
        args.insert("start".to_string(), to_value(6).unwrap());

        assert!(range_fn(args).is_err());
    }

    #[test]
    fn test_range_step_by() {
        let mut args = HashMap::new();
        args.insert("end".to_string(), to_value(10).unwrap());
        args.insert("step_by".to_string(), to_value(2).unwrap());

        let res = range_fn(args).unwrap();
        assert_eq!(res, to_value(vec![0,2,4,6,8]).unwrap());
    }
}

