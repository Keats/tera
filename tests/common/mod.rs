extern crate serde;
extern crate serde_json;
extern crate tera;

use std::io::prelude::*;
use std::fs::File;

use self::tera::Template;
use self::serde::ser::SerializeStruct;


#[derive(Debug)]
pub struct Product {
    name: String,
    manufacturer: String,
    price: i32,
    summary: String
}
impl Product {
    #[allow(dead_code)]
    pub fn new() -> Product {
        Product {
            name: "Moto G".to_owned(),
            manufacturer: "Motorala".to_owned(),
            summary: "A phone".to_owned(),
            price: 100
        }
    }
}
// Impl Serialize by hand so tests pass on stable and beta
impl serde::Serialize for Product {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: serde::Serializer
    {
        let mut state = serializer.serialize_struct("Product", 4)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("manufacturer", &self.manufacturer)?;
        state.serialize_field("summary", &self.summary)?;
        state.serialize_field("price", &self.price)?;
        state.end()
    }
}

#[derive(Debug)]
pub struct Review {
    title: String,
    paragraphs: Vec<String>
}
impl Review {
    #[allow(dead_code)]
    pub fn new() -> Review {
        Review {
            title: "My review".to_owned(),
            paragraphs: vec![
                "A".to_owned(), "B".to_owned(), "C".to_owned()
            ]
        }
    }
}
// Impl Serialize by hand so tests pass on stable and beta
impl serde::Serialize for Review {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: serde::Serializer
    {
        let mut state = serializer.serialize_struct("Review", 2)?;
        state.serialize_field("title", &self.title)?;
        state.serialize_field("paragraphs", &self.paragraphs)?;
        state.end()
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
