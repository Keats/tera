use std::collections::BTreeMap;

use serde::ser::Serialize;
use serde_json::value::{Value, to_value};

pub type TemplateContext = BTreeMap<String, Value>;

/// The struct that holds the context of a template rendering.
///
/// Light wrapper around a `BTreeMap` for easier insertions of Serializable
/// values
#[derive(Debug, Clone)]
pub struct Context {
    data: BTreeMap<String, Value>,
}

impl Context {
    /// Initializes an empty context
    pub fn new() -> Context {
        Context {
            data: BTreeMap::new()
        }
    }

    /// Converts the `val` parameter to `Value` and insert it into the context
    ///
    /// ```rust,ignore
    /// let mut context = Context::new();
    /// // user is an instance of a struct implementing `Serialize`
    /// context.add("number_users", 42);
    /// ```
    pub fn add<T: Serialize>(&mut self, key: &str, val: &T) {
        self.data.insert(key.to_owned(), to_value(val));
    }

    #[doc(hidden)]
    pub fn as_json(&self) -> Value {
        to_value(&self.data)
    }
}

impl Default for Context {
    fn default() -> Context {
        Context::new()
    }
}

pub trait ValueRender {
    fn render(&self) -> String;
}

// Needed to render variables
// From handlebars-rust
impl ValueRender for Value {
    fn render(&self) -> String {
        match *self {
            Value::String(ref s) => s.clone(),
            Value::I64(i) => i.to_string(),
            Value::U64(i) => i.to_string(),
            Value::F64(f) => f.to_string(),
            Value::Bool(i) => i.to_string(),
            Value::Null => "".to_owned(),
            Value::Array(ref a) => {
                let mut buf = String::new();
                buf.push('[');
                for i in a.iter() {
                    buf.push_str(i.render().as_ref());
                    buf.push_str(", ");
                }
                buf.push(']');
                buf
            },
            Value::Object(_) => "[object]".to_owned()
        }
    }
}


pub trait ValueNumber {
    fn to_number(&self) -> Result<f64, ()>;
}
// Needed for all the maths
// Convert everything to f64, seems like a terrible idea
impl ValueNumber for Value {
    fn to_number(&self) -> Result<f64, ()> {
        match *self {
            Value::I64(i) => Ok(i as f64),
            Value::U64(i) => Ok(i as f64),
            Value::F64(f) => Ok(f as f64),
            _ => Err(())
        }
    }
}

// From handlebars-rust
pub trait ValueTruthy {
    fn is_truthy(&self) -> bool;
}

impl ValueTruthy for Value {
    fn is_truthy(&self) -> bool {
        match *self {
            Value::I64(i) => i != 0,
            Value::U64(i) => i != 0,
            Value::F64(i) => i != 0.0 || !i.is_nan(),
            Value::Bool(ref i) => *i,
            Value::Null => false,
            Value::String(ref i) => !i.is_empty(),
            Value::Array(ref i) => !i.is_empty(),
            Value::Object(ref i) => !i.is_empty()
        }
    }
}


/// Converts a dotted path to a json pointer one
pub fn get_json_pointer(key: &str) -> String {
    ["/", &key.replace(".", "/")].join("")
}
