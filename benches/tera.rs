#![feature(test)]
extern crate test;
extern crate tera;
extern crate serde;
extern crate serde_json;

use tera::{Tera, Template, Context};


static VARIABLE_ONLY: &'static str = "{{product.name}}";

static SIMPLE_TEMPLATE: &'static str = "
<html>
  <head>
    <title>{{ product.name }}</title>
  </head>
  <body>
    <h1>{{ product.name }} - {{ product.manufacturer | upper }}</h1>
    <p>{{ product.summary }}</p>
    <p>£{{ product.price * 1.20 }} (VAT inc.)</p>
    <p>Look at reviews from your friends {{ username }}</p>
    <button>Buy!</button>
  </body>
</html>
";

static PARENT_TEMPLATE: &'static str = "
<html>
  <head>
    <title>{% block title %}Hello{% endblock title%}</title>
  </head>
  <body>
    {% block body %}{% endblock body %}
  </body>
</html>
";

static MACRO_TEMPLATE: &'static str = "
{% macro render_product(product) %}
    <h1>{{ product.name }} - {{ product.manufacturer | upper }}</h1>
    <p>{{ product.summary }}</p>
    <p>£{{ product.price * 1.20 }} (VAT inc.)</p>
    <button>Buy!</button>
{% endmacro render_product %}
";

static CHILD_TEMPLATE: &'static str = r#"{% extends "parent.html" %}
{% block title %}{{ super() }} - {{ username | lower }}{% endblock title %}

{% block body %}body{% endblock body %}
"#;

static CHILD_TEMPLATE_WITH_MACRO: &'static str = r#"{% extends "parent.html" %}
{% import "macros.html" as macros %}

{% block title %}{{ super() }} - {{ username | lower }}{% endblock title %}

{% block body %}
{{ macros::render_product(product=product) }}
{% endblock body %}
"#;

static USE_MACRO_TEMPLATE: &'static str = r#"
{% import "macros.html" as macros %}
{{ macros::render_product(product=product) }}
"#;


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
fn bench_parsing_basic_template(b: &mut test::Bencher) {
    b.iter(|| Template::new("bench", SIMPLE_TEMPLATE));
}

#[bench]
fn bench_parsing_with_inheritance_and_macros(b: &mut test::Bencher) {
    let mut tera = Tera::default();
    b.iter(|| tera.add_templates(vec![
        ("parent.html", PARENT_TEMPLATE),
        ("child.html", CHILD_TEMPLATE),
        ("macros.html", MACRO_TEMPLATE),
    ]));
}

#[bench]
fn bench_rendering_only_variable(b: &mut test::Bencher) {
    let mut tera = Tera::default();
    tera.add_template("test.html", VARIABLE_ONLY);
    let mut context = Context::new();
    context.add("product", &Product::new());
    context.add("username", &"bob");

    b.iter(|| tera.render("test.html", context.clone()));
}

#[bench]
fn bench_rendering_basic_template(b: &mut test::Bencher) {
    let mut tera = Tera::default();
    tera.add_template("bench.html", SIMPLE_TEMPLATE);
    let mut context = Context::new();
    context.add("product", &Product::new());
    context.add("username", &"bob");

    b.iter(|| tera.render("bench.html", context.clone()));
}

#[bench]
fn bench_rendering_only_parent(b: &mut test::Bencher) {
    let mut tera = Tera::default();
    tera.add_templates(vec![
        ("parent.html", PARENT_TEMPLATE),
    ]);
    let mut context = Context::new();
    context.add("product", &Product::new());
    context.add("username", &"bob");

    b.iter(|| tera.render("parent.html", context.clone()));
}

#[bench]
fn bench_rendering_only_macro_call(b: &mut test::Bencher) {
    let mut tera = Tera::default();
    tera.add_templates(vec![
        ("hey.html", USE_MACRO_TEMPLATE),
    ]);
    let mut context = Context::new();
    context.add("product", &Product::new());
    context.add("username", &"bob");

    b.iter(|| tera.render("hey.html", context.clone()));
}

#[bench]
fn bench_rendering_only_inheritance(b: &mut test::Bencher) {
    let mut tera = Tera::default();
    tera.add_templates(vec![
        ("parent.html", PARENT_TEMPLATE),
        ("child.html", CHILD_TEMPLATE),
    ]);
    let mut context = Context::new();
    context.add("product", &Product::new());
    context.add("username", &"bob");

    b.iter(|| tera.render("child.html", context.clone()));
}

#[bench]
fn bench_rendering_inheritance_and_macros(b: &mut test::Bencher) {
    let mut tera = Tera::default();
    tera.add_templates(vec![
        ("parent.html", PARENT_TEMPLATE),
        ("child.html", CHILD_TEMPLATE_WITH_MACRO),
        ("macros.html", MACRO_TEMPLATE),
    ]);
    let mut context = Context::new();
    context.add("product", &Product::new());
    context.add("username", &"bob");

    b.iter(|| tera.render("child.html", context.clone()));
}
