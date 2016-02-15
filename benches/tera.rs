#![feature(test, custom_derive, plugin)]
#![plugin(serde_macros)]
extern crate test;
extern crate tera;
extern crate serde;
extern crate serde_json;


use std::collections::BTreeMap;
use serde_json::value::{Value as Json, to_value};
use tera::Template;


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

#[derive(Debug, Serialize)]
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

#[bench]
fn bench_parsing(b: &mut test::Bencher) {
    b.iter(|| Template::new("bench", TEMPLATE));
}

#[bench]
fn bench_rendering(b: &mut test::Bencher) {
    let template = Template::new("bench", TEMPLATE);
    let mut data: BTreeMap<String, Json> = BTreeMap::new();
    data.insert("product".to_owned(), to_value(&Product::new()));
    data.insert("username".to_owned(), to_value(&"bob"));

    b.iter(|| template.render(&data));
}
