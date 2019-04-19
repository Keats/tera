use std::borrow::Cow;
use std::collections::BTreeMap;

use serde::ser::Serialize;
use serde_json::value::{to_value, Map, Value};

/// The struct that holds the context of a template rendering.
///
/// Light wrapper around a `BTreeMap` for easier insertions of Serializable
/// values
#[derive(Debug, Clone, PartialEq)]
pub struct Context {
    data: BTreeMap<String, Value>,
}

impl Context {
    /// Initializes an empty context
    pub fn new() -> Context {
        Context { data: BTreeMap::new() }
    }

    /// Converts the `val` parameter to `Value` and insert it into the context
    ///
    /// ```rust,ignore
    /// let mut context = Context::new();
    /// // user is an instance of a struct implementing `Serialize`
    /// context.insert("number_users", 42);
    /// ```
    pub fn insert<T: Serialize + ?Sized>(&mut self, key: &str, val: &T) {
        self.data.insert(key.to_owned(), to_value(val).unwrap());
    }

    /// Appends the data of the `source` parameter to `self`, overwriting existing keys.
    /// The source context will be dropped.
    ///
    /// ```rust,ignore
    /// let mut target = Context::new();
    /// target.insert("a", 1);
    /// target.insert("b", 2);
    /// let mut source = Context::new();
    /// source.insert("b", 3);
    /// source.insert("d", 4);
    /// target.extend(source);
    /// ```
    pub fn extend(&mut self, mut source: Context) {
        self.data.append(&mut source.data);
    }

    /// Converts the context to a `serde_json::Value` consuming the context
    pub fn into_json(self) -> Value {
        self.into()
    }
}

impl Default for Context {
    fn default() -> Context {
        Context::new()
    }
}

pub trait ValueRender {
    fn render(&self) -> Cow<str>;
}

// Convert serde Value to String
impl ValueRender for Value {
    fn render(&self) -> Cow<str> {
        match *self {
            Value::String(ref s) => Cow::Borrowed(s),
            Value::Number(ref i) => Cow::Owned(i.to_string()),
            Value::Bool(i) => Cow::Owned(i.to_string()),
            Value::Null => Cow::Owned(String::new()),
            Value::Array(ref a) => {
                let mut buf = String::new();
                buf.push('[');
                for i in a.iter() {
                    if buf.len() > 1 {
                        buf.push_str(", ");
                    }
                    buf.push_str(i.render().as_ref());
                }
                buf.push(']');
                Cow::Owned(buf)
            }
            Value::Object(_) => Cow::Owned("[object]".to_owned()),
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
            Value::Number(ref i) => Ok(i.as_f64().unwrap()),
            _ => Err(()),
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
            Value::Number(ref i) => {
                if i.is_i64() {
                    return i.as_i64().unwrap() != 0;
                }
                if i.is_u64() {
                    return i.as_u64().unwrap() != 0;
                }
                let f = i.as_f64().unwrap();
                f != 0.0 && !f.is_nan()
            }
            Value::Bool(ref i) => *i,
            Value::Null => false,
            Value::String(ref i) => !i.is_empty(),
            Value::Array(ref i) => !i.is_empty(),
            Value::Object(ref i) => !i.is_empty(),
        }
    }
}

impl From<Context> for Value {
    fn from(ctx: Context) -> Self {
        let mut m = Map::new();
        for (key, value) in ctx.data {
            m.insert(key, value);
        }
        Value::Object(m)
    }
}

/// Converts a dotted path to a json pointer one
#[inline]
pub fn get_json_pointer(key: &str) -> String {
    ["/", &key.replace(".", "/")].join("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extend() {
        let mut target = Context::new();
        target.insert("a", &1);
        target.insert("b", &2);
        let mut source = Context::new();
        source.insert("b", &3);
        source.insert("c", &4);
        target.extend(source);
        assert_eq!(*target.data.get("a").unwrap(), to_value(1).unwrap());
        assert_eq!(*target.data.get("b").unwrap(), to_value(3).unwrap());
        assert_eq!(*target.data.get("c").unwrap(), to_value(4).unwrap());
    }
}
