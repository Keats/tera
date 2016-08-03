extern crate serde;
extern crate serde_json;
extern crate tera;

use std::io::prelude::*;
use std::fs::File;

use self::tera::Template;


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
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: serde::Serializer
    {
        let mut state = try!(serializer.serialize_struct("Product", 4));
        try!(serializer.serialize_struct_elt(&mut state, "name", &self.name));
        try!(serializer.serialize_struct_elt(&mut state, "manufacturer", &self.manufacturer));
        try!(serializer.serialize_struct_elt(&mut state, "summary", &self.summary));
        try!(serializer.serialize_struct_elt(&mut state, "price", &self.price));
        serializer.serialize_struct_end(state)
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
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: serde::Serializer
    {
        let mut state = try!(serializer.serialize_struct("Review", 2));
        try!(serializer.serialize_struct_elt(&mut state, "title", &self.title));
        try!(serializer.serialize_struct_elt(&mut state, "paragraphs", &self.paragraphs));
        serializer.serialize_struct_end(state)
    }
}

#[allow(dead_code)]
pub fn load_template(path: &str) -> Template {
    Template::new("tpl", &read_file(path))
}

#[allow(dead_code)]
pub fn read_file(path: &str) -> String {
    let mut f = File::open(path).unwrap();
    let mut input = String::new();
    f.read_to_string(&mut input).unwrap();

    input
}
