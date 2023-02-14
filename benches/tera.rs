#![feature(test)]
extern crate tera;
extern crate test;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use tera::{Context, Template, Tera, Value};

static VARIABLE_ONLY: &str = "{{product.name}}";

static SIMPLE_TEMPLATE: &str = "
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

static PARENT_TEMPLATE: &str = "
<html>
  <head>
    <title>{% block title %}Hello{% endblock title%}</title>
  </head>
  <body>
    {% block body %}{% endblock body %}
  </body>
</html>
";

static MACRO_TEMPLATE: &str = "
{% macro render_product(product) %}
    <h1>{{ product.name }} - {{ product.manufacturer | upper }}</h1>
    <p>{{ product.summary }}</p>
    <p>£{{ product.price * 1.20 }} (VAT inc.)</p>
    <button>Buy!</button>
{% endmacro render_product %}
";

static CHILD_TEMPLATE: &str = r#"{% extends "parent.html" %}
{% block title %}{{ super() }} - {{ username | lower }}{% endblock title %}

{% block body %}body{% endblock body %}
"#;

static CHILD_TEMPLATE_WITH_MACRO: &str = r#"{% extends "parent.html" %}
{% import "macros.html" as macros %}

{% block title %}{{ super() }} - {{ username | lower }}{% endblock title %}

{% block body %}
{{ macros::render_product(product=product) }}
{% endblock body %}
"#;

static USE_MACRO_TEMPLATE: &str = r#"
{% import "macros.html" as macros %}
{{ macros::render_product(product=product) }}
"#;

#[derive(Debug, Serialize)]
struct Product {
    name: String,
    manufacturer: String,
    price: i32,
    summary: String,
}
impl Product {
    pub fn new() -> Product {
        Product {
            name: "Moto G".to_owned(),
            manufacturer: "Motorala".to_owned(),
            summary: "A phone".to_owned(),
            price: 100,
        }
    }
}

#[bench]
fn bench_parsing_basic_template(b: &mut test::Bencher) {
    b.iter(|| Template::new("bench", None, SIMPLE_TEMPLATE));
}

#[bench]
fn bench_parsing_with_inheritance_and_macros(b: &mut test::Bencher) {
    let mut tera = Tera::default();
    b.iter(|| {
        tera.add_raw_templates(vec![
            ("parent.html", PARENT_TEMPLATE),
            ("child.html", CHILD_TEMPLATE),
            ("macros.html", MACRO_TEMPLATE),
        ])
    });
}

#[bench]
fn bench_rendering_only_variable(b: &mut test::Bencher) {
    let mut tera = Tera::default();
    tera.add_raw_template("test.html", VARIABLE_ONLY).unwrap();
    let mut context = Context::new();
    context.insert("product", &Product::new());
    context.insert("username", &"bob");

    b.iter(|| tera.render("test.html", &context));
}

#[bench]
fn bench_rendering_basic_template(b: &mut test::Bencher) {
    let mut tera = Tera::default();
    tera.add_raw_template("bench.html", SIMPLE_TEMPLATE).unwrap();
    let mut context = Context::new();
    context.insert("product", &Product::new());
    context.insert("username", &"bob");

    b.iter(|| tera.render("bench.html", &context));
}

#[bench]
fn bench_rendering_only_parent(b: &mut test::Bencher) {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![("parent.html", PARENT_TEMPLATE)]).unwrap();
    let mut context = Context::new();
    context.insert("product", &Product::new());
    context.insert("username", &"bob");

    b.iter(|| tera.render("parent.html", &context));
}

#[bench]
fn bench_rendering_only_macro_call(b: &mut test::Bencher) {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![("hey.html", USE_MACRO_TEMPLATE), ("macros.html", MACRO_TEMPLATE)])
        .unwrap();
    let mut context = Context::new();
    context.insert("product", &Product::new());
    context.insert("username", &"bob");

    b.iter(|| tera.render("hey.html", &context));
}

#[bench]
fn bench_rendering_only_inheritance(b: &mut test::Bencher) {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![("parent.html", PARENT_TEMPLATE), ("child.html", CHILD_TEMPLATE)])
        .unwrap();
    let mut context = Context::new();
    context.insert("product", &Product::new());
    context.insert("username", &"bob");

    b.iter(|| tera.render("child.html", &context));
}

#[bench]
fn bench_rendering_inheritance_and_macros(b: &mut test::Bencher) {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("parent.html", PARENT_TEMPLATE),
        ("child.html", CHILD_TEMPLATE_WITH_MACRO),
        ("macros.html", MACRO_TEMPLATE),
    ])
    .unwrap();
    let mut context = Context::new();
    context.insert("product", &Product::new());
    context.insert("username", &"bob");

    b.iter(|| tera.render("child.html", &context));
}

#[bench]
fn bench_build_inheritance_chains(b: &mut test::Bencher) {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("parent.html", PARENT_TEMPLATE),
        ("child.html", CHILD_TEMPLATE_WITH_MACRO),
        ("macros.html", MACRO_TEMPLATE),
    ])
    .unwrap();
    b.iter(|| tera.build_inheritance_chains());
}

#[bench]
fn bench_huge_loop(b: &mut test::Bencher) {
    #[derive(Serialize)]
    struct DataWrapper {
        v: String,
    }

    #[derive(Serialize)]
    struct RowWrapper {
        real: Vec<DataWrapper>,
        dummy: Vec<DataWrapper>,
    }
    let real: Vec<DataWrapper> = (1..1000).map(|i| DataWrapper { v: format!("n={}", i) }).collect();
    let dummy: Vec<DataWrapper> =
        (1..1000).map(|i| DataWrapper { v: format!("n={}", i) }).collect();
    let rows = RowWrapper { real, dummy };

    let mut tera = Tera::default();
    tera.add_raw_templates(vec![(
        "huge.html",
        "{% for real in rows.real %}{{real.v}}{% endfor %}",
    )])
    .unwrap();
    let mut context = Context::new();
    context.insert("rows", &rows);

    b.iter(|| tera.render("huge.html", &context.clone()));
}

fn deep_object() -> Value {
    let data = r#"{
                    "foo": {
                        "bar": {
                            "goo": {
                                "moo": {
                                    "cows": [
                                        {
                                            "name": "betsy",
                                            "age" : 2,
                                            "temperament": "calm"
                                        },
                                        {
                                            "name": "elsie",
                                            "age": 3,
                                            "temperament": "calm"
                                        },
                                        {
                                            "name": "veal",
                                            "age": 1,
                                            "temperament": "ornery"
                                        }
                                    ]
                                }
                            }
                        }
                    }
                  }"#;

    serde_json::from_str(data).unwrap()
}

#[bench]
fn access_deep_object(b: &mut test::Bencher) {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![(
        "deep_object.html",
        "{% for cow in deep_object.foo.bar.goo.moo.cows %}{{cow.temperament}}{% endfor %}",
    )])
    .unwrap();
    let mut context = Context::new();
    println!("{:?}", deep_object());
    context.insert("deep_object", &deep_object());
    assert!(tera.render("deep_object.html", &context).unwrap().contains("ornery"));

    b.iter(|| tera.render("deep_object.html", &context));
}

#[bench]
fn access_deep_object_with_literal(b: &mut test::Bencher) {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![(
        "deep_object.html",
        "
{% set goo = deep_object.foo['bar'][\"goo\"] %}
{% for cow in goo.moo.cows %}{{cow.temperament}}
{% endfor %}",
    )])
    .unwrap();
    let mut context = Context::new();
    context.insert("deep_object", &deep_object());
    assert!(tera.render("deep_object.html", &context).unwrap().contains("ornery"));

    b.iter(|| tera.render("deep_object.html", &context));
}
