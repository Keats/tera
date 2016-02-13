use std::collections::{BTreeMap};

use serde::ser::Serialize;
use serde_json::value::{Value as Json, to_value};


pub type TemplateContext = BTreeMap<String, Json>;

#[derive(Debug)]
pub struct Context {
    data: Json
}


impl Context {
    pub fn null() -> Context {
        Context {
            data: Json::Null
        }
    }

    pub fn new<T: Serialize>(d: &T) -> Context {
        Context {
            data: to_value(d)
        }
    }

    pub fn get(&self, path: &str) -> Option<&Json> {
        return self.data.lookup(path);
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
