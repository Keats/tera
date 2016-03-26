#![feature(test, custom_derive, plugin)]
#![plugin(serde_macros)]
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
    let mut context = Context::new();
    context.add("product", &Product::new());
    context.add("username", &"bob");

    b.iter(|| template.render(context.clone(), HashMap::new()));
}
