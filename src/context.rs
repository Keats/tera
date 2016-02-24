use std::collections::BTreeMap;

use serde::ser::Serialize;
use serde_json::value::{Value as Json, to_value};


pub type TemplateContext = BTreeMap<String, Json>;

#[derive(Debug)]
pub struct Context {
    data: Json
}

impl Context {
    pub fn new<T: Serialize>(d: &T) -> Context {
        Context {
            data: to_value(d)
        }
    }

    pub fn get(&self, path: &str) -> Option<&Json> {
        self.data.lookup(path)
    }
}

impl Default for Context {
    fn default() -> Context {
        Context {
            data: Json::Null
        }
    }
}

pub trait JsonRender {
    fn render(&self) -> String;
}

// Needed to render variables
impl JsonRender for Json {
    fn render(&self) -> String {
        match *self {
            Json::String(ref s) => s.to_string(),
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
            Json::String (ref i) => i.len() > 0,
            Json::Array (ref i) => i.len() > 0,
            Json::Object (ref i) => i.len() > 0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Context};
    use std::collections::BTreeMap;

    #[derive(Debug, Serialize, Clone)]
    pub struct Score {
        rank: i64,
        username: String,
    }

    impl Default for Score {
        fn default() -> Score {
            Score {
                rank: 42,
                username: "Billy".to_owned()
            }
        }
    }

    #[test]
    fn test_get_top_level() {
        let mut d = BTreeMap::new();
        d.insert("url".to_owned(), "https://wearewizards.io");
        let context = Context::new(&d);

        assert_eq!(context.get("url").unwrap().as_string().unwrap(), "https://wearewizards.io".to_owned());
    }

    #[test]
    fn test_get_in_deep() {
        let mut d = BTreeMap::new();
        let score = Score::default();
        d.insert("user".to_owned(), score.clone());
        let context = Context::new(&d);
        let score_rank = context.get("user.rank").unwrap().as_i64();

        assert_eq!(score_rank, Some(score.rank));
    }


    #[test]
    fn test_get_inexistent() {
        let mut d = BTreeMap::new();
        let score = Score::default();
        d.insert("user".to_owned(), score.clone());
        let context = Context::new(&d);
        let score_rank = context.get("user.position");

        assert_eq!(score_rank, None);
    }
}
