use std::collections::BTreeMap;

use serde::ser::Serialize;
use serde_json::value::{Value as Json, to_value};


pub type TemplateContext = BTreeMap<String, Json>;

#[derive(Debug, Clone)]
pub struct Context {
    data: BTreeMap<String, Json>,
}

impl Context {
    pub fn new() -> Context {
        Context {
            data: BTreeMap::new()
        }
    }

    pub fn add<T: Serialize>(&mut self, key: &str, d: &T) {
        self.data.insert(key.to_owned(), to_value(d));
    }

    pub fn as_json(&self) -> Json {
        to_value(&self.data)
    }
}

pub trait JsonRender {
    fn render(&self) -> String;
}
// Needed to render variables
// From handlebars-rust
impl JsonRender for Json {
    fn render(&self) -> String {
        match *self {
            Json::String(ref s) => s.clone(),
            Json::I64(i) => i.to_string(),
            Json::U64(i) => i.to_string(),
            Json::F64(f) => f.to_string(),
            Json::Bool (i) => i.to_string(),
            Json::Null => "".to_owned(),
            Json::Array (ref a) => {
                let mut buf = String::new();
                buf.push('[');
                for i in a.iter() {
                    buf.push_str(i.render().as_ref());
                    buf.push_str(", ");
                }
                buf.push(']');
                buf
            },
            Json::Object (_) => "[object]".to_owned()
        }
    }
}


pub trait JsonNumber {
    fn to_number(&self) -> Result<f32, ()>;
}
// Needed for all the maths
// Convert everything to f32, seems like a terrible idea
impl JsonNumber for Json {
    fn to_number(&self) -> Result<f32, ()> {
        match *self {
            Json::I64(i) => Ok(i as f32),
            Json::U64(i) => Ok(i as f32),
            Json::F64(f) => Ok(f as f32),
            _ => Err(())
        }
    }
}

// From handlebars-rust
pub trait JsonTruthy {
    fn is_truthy(&self) -> bool;
}
impl JsonTruthy for Json {
    fn is_truthy(&self) -> bool {
        match *self {
            Json::I64(i) => i != 0,
            Json::U64(i) => i != 0,
            Json::F64(i) => i != 0.0 || ! i.is_nan(),
            Json::Bool (ref i) => *i,
            Json::Null => false,
            Json::String (ref i) => !i.is_empty(),
            Json::Array (ref i) => !i.is_empty(),
            Json::Object (ref i) => !i.is_empty()
        }
    }
}
