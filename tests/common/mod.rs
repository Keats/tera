extern crate serde;
extern crate serde_json;
extern crate tera;

use std::fs::File;
use std::io::prelude::*;

use self::tera::Template;

#[derive(Debug, Serialize)]
pub struct Product {
    name: String,
    manufacturer: String,
    price: i32,
    summary: String,
}
impl Product {
    #[allow(dead_code)]
    pub fn new() -> Product {
        Product {
            name: "Moto G".to_owned(),
            manufacturer: "Motorala".to_owned(),
            summary: "A phone".to_owned(),
            price: 100,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Review {
    title: String,
    paragraphs: Vec<String>,
}
impl Review {
    #[allow(dead_code)]
    pub fn new() -> Review {
        Review {
            title: "My review".to_owned(),
            paragraphs: vec!["A".to_owned(), "B".to_owned(), "C".to_owned()],
        }
    }
}

#[allow(dead_code)]
pub fn load_template(path: &str) -> Template {
    Template::new("tpl", None, &read_file(path)).unwrap()
}

#[allow(dead_code)]
pub fn read_file(path: &str) -> String {
    let mut f = File::open(path).unwrap();
    let mut input = String::new();
    f.read_to_string(&mut input).unwrap();

    input
}

#[allow(dead_code)]
#[derive(Debug, Serialize)]
pub struct NestedObject {
    pub label: String,
    pub parent: Option<Box<NestedObject>>,
    pub numbers: Vec<usize>,
}
