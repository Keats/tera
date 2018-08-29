use std::collections::HashMap;

use errors::Result;
use serde_json::value::Value;

pub mod array;
pub mod common;
pub mod number;
pub mod object;
pub mod string;

/// The filter function type definition
pub type FilterFn = fn(Value, HashMap<String, Value>) -> Result<Value>;
