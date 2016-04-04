#![feature(test)]
extern crate test;
extern crate tera;
extern crate serde;
extern crate serde_json;

use std::collections::HashMap;
use tera::{Template, Context};


static TEMPLATE: &'static str = "
<html>
  <head>
    <title>{{ product.name }}</title>
  </head>
  <body>
    <h1>{{ product.name }} - {{ product.manufacturer }}</h1>
    <p>{{ product.summary }}</p>
    <p>£{{ product.price * 1.20 }} (VAT inc.)</p>
    <p>Look at reviews from your friends {{ username }}</p>
    <button>Buy!</button>
  </body>
</html>
";

#[derive(Debug)]
struct Product {
    name: String,
    manufacturer: String,
    price: i32,
    summary: String
}
impl Product {
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
        serializer.serialize_struct("Product", ProductMapVisitor {
            value: self,
            state: 0,
        })
    }
}

struct ProductMapVisitor<'a> {
    value: &'a Product,
    state: u8,
}

impl<'a> serde::ser::MapVisitor for ProductMapVisitor<'a> {
    fn visit<S>(&mut self, serializer: &mut S) -> Result<Option<()>, S::Error>
        where S: serde::Serializer
    {
        match self.state {
            0 => {
                self.state += 1;
                Ok(Some(try!(serializer.serialize_struct_elt("name", &self.value.name))))
            },
            1 => {
                self.state += 1;
                Ok(Some(try!(serializer.serialize_struct_elt("manufacturer", &self.value.manufacturer))))
            },
            2 => {
                self.state += 1;
                Ok(Some(try!(serializer.serialize_struct_elt("price", &self.value.price))))
            },
            3 => {
                self.state += 1;
                Ok(Some(try!(serializer.serialize_struct_elt("summary", &self.value.summary))))
            },
            _ => {
                Ok(None)
            }
        }
    }
}

#[bench]
fn bench_parsing(b: &mut test::Bencher) {
    b.iter(|| Template::new("bench", TEMPLATE));
}

#[bench]
fn bench_rendering(b: &mut test::Bencher) {
    let template = Template::new("bench", TEMPLATE);
    let mut context = Context::new();
    context.add("product", &Product::new());
    context.add("username", &"bob");

    b.iter(|| template.render(context.clone(), HashMap::new()));
}
