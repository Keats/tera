use std::collections::HashMap;

use serde_json::value::Value;
use errors::Result;

pub mod string;
pub mod number;
pub mod array;
pub mod common;
pub mod object;

/// The filter function type definition
pub type FilterFn = fn(Value, HashMap<String, Value>) -> Result<Value>;
