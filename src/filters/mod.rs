use std::collections::HashMap;

use serde_json::value::Value;
use errors::TeraResult;


pub mod string;
pub mod number;
pub mod array;
pub mod common;

pub type FilterFn = fn(Value, HashMap<String, Value>) -> TeraResult<Value>;
