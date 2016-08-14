#![feature(test)]
extern crate test;
extern crate tera;
extern crate serde;
extern crate serde_json;

use tera::{Tera, Template, Context};


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
        let mut state = try!(serializer.serialize_struct("Product", 4));
        try!(serializer.serialize_struct_elt(&mut state, "name", &self.name));
        try!(serializer.serialize_struct_elt(&mut state, "manufacturer", &self.manufacturer));
        try!(serializer.serialize_struct_elt(&mut state, "summary", &self.summary));
        try!(serializer.serialize_struct_elt(&mut state, "price", &self.price));
        serializer.serialize_struct_end(state)
    }
}

#[bench]
fn bench_parsing(b: &mut test::Bencher) {
    b.iter(|| Template::new("bench", TEMPLATE));
}

#[bench]
fn bench_rendering(b: &mut test::Bencher) {
    let mut tera = Tera::default();
    tera.add_template("bench", TEMPLATE);
    let mut context = Context::new();
    context.add("product", &Product::new());
    context.add("username", &"bob");

    b.iter(|| tera.render("bench", context.clone()));
}
