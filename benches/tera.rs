#![feature(test)]
extern crate test;
extern crate tera;
#[macro_use]
extern crate serde_derive;

use tera::{Tera, Template, Context, escape_html};


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
fn bench_parsing_basic_template(b: &mut test::Bencher) {
    b.iter(|| Template::new("bench", None, SIMPLE_TEMPLATE));
}

#[bench]
fn bench_parsing_with_inheritance_and_macros(b: &mut test::Bencher) {
    let mut tera = Tera::default();
    b.iter(|| tera.add_raw_templates(vec![
        ("parent.html", PARENT_TEMPLATE),
        ("child.html", CHILD_TEMPLATE),
        ("macros.html", MACRO_TEMPLATE),
    ]));
}

#[bench]
fn bench_rendering_only_variable(b: &mut test::Bencher) {
    let mut tera = Tera::default();
    tera.add_raw_template("test.html", VARIABLE_ONLY).unwrap();
    let mut context = Context::new();
    context.add("product", &Product::new());
    context.add("username", &"bob");

    b.iter(|| tera.render("test.html", &context));
}

#[bench]
fn bench_rendering_basic_template(b: &mut test::Bencher) {
    let mut tera = Tera::default();
    tera.add_raw_template("bench.html", SIMPLE_TEMPLATE).unwrap();
    let mut context = Context::new();
    context.add("product", &Product::new());
    context.add("username", &"bob");

    b.iter(|| tera.render("bench.html", &context));
}

#[bench]
fn bench_rendering_only_parent(b: &mut test::Bencher) {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("parent.html", PARENT_TEMPLATE),
    ]).unwrap();
    let mut context = Context::new();
    context.add("product", &Product::new());
    context.add("username", &"bob");

    b.iter(|| tera.render("parent.html", &context));
}

#[bench]
fn bench_rendering_only_macro_call(b: &mut test::Bencher) {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("hey.html", USE_MACRO_TEMPLATE),
    ]).unwrap();
    let mut context = Context::new();
    context.add("product", &Product::new());
    context.add("username", &"bob");

    b.iter(|| tera.render("hey.html", &context));
}

#[bench]
fn bench_rendering_only_inheritance(b: &mut test::Bencher) {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("parent.html", PARENT_TEMPLATE),
        ("child.html", CHILD_TEMPLATE),
    ]).unwrap();
    let mut context = Context::new();
    context.add("product", &Product::new());
    context.add("username", &"bob");

    b.iter(|| tera.render("child.html", &context));
}

#[bench]
fn bench_rendering_inheritance_and_macros(b: &mut test::Bencher) {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("parent.html", PARENT_TEMPLATE),
        ("child.html", CHILD_TEMPLATE_WITH_MACRO),
        ("macros.html", MACRO_TEMPLATE),
    ]).unwrap();
    let mut context = Context::new();
    context.add("product", &Product::new());
    context.add("username", &"bob");

    b.iter(|| tera.render("child.html", &context));
}

#[bench]
fn bench_build_inheritance_chains(b: &mut test::Bencher) {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("parent.html", PARENT_TEMPLATE),
        ("child.html", CHILD_TEMPLATE_WITH_MACRO),
        ("macros.html", MACRO_TEMPLATE),
    ]).unwrap();
    b.iter(|| tera.build_inheritance_chains());
}


#[bench]
fn bench_escape_html(b: &mut test::Bencher) {
    b.iter(|| escape_html(r#"Hello word <script></script>"#));
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
    let real: Vec<DataWrapper> = (1..100)
        .into_iter()
        .map(|i| DataWrapper { v: format!("n={}", i) })
        .collect();
    let dummy: Vec<DataWrapper> = (1..100)
        .into_iter()
        .map(|i| DataWrapper { v: format!("n={}", i) })
        .collect();
    let rows = RowWrapper { real, dummy };

    let mut tera = Tera::default();
    let loop_control: Vec<i32> = (0..500).collect();


    tera.add_raw_templates(vec![
        ("huge.html", "
{% for i in loop %}      
{% for j in rows.real %}  
{{j.v}}
{% endfor %}
{% endfor %}
"),
    ]).unwrap();
    let mut context = Context::new();
    context.add("rows", &rows);
    context.add("loop", &loop_control);

    b.iter(|| tera.render("huge.html", &context));
}

#[bench]
fn bench_huge_object(b: &mut test::Bencher) {
    #[derive(Serialize, Clone)]
    struct DataWrapper {
        v: String,
    }

    #[derive(Serialize, Clone)]    
    struct BigRow {
        v: Vec<DataWrapper>,
    }   

    #[derive(Serialize, Clone)]    
    struct BigObject {
        a: BigRow,
        b: BigRow,
        c: BigRow,
        d: BigRow,
        e: BigRow,
    }

    #[derive(Serialize)]
    struct RowWrapper {
        real: Vec<BigObject>,
    }

    let v: Vec<DataWrapper> = (1..5000)
        .into_iter()
        .map(|i| DataWrapper { v: format!("n={}", i) })
        .collect();

    let big_row = BigRow { v };

    let big_object = BigObject {
        a: big_row.clone(),
        b: big_row.clone(),
        c: big_row.clone(),
        d: big_row.clone(),
        e: big_row.clone(),
    };

    let mut tera = Tera::default();

    let loop_control: Vec<i32> = (0..10).collect();

    tera.add_raw_templates(vec![
        ("huge.html", "
{% for i in loop %}
    {% for v in big_object.c.v %}
{{v.v}}
    {% endfor %}
{% endfor %}
"),
    ]).unwrap();
    let mut context = Context::new();
    context.add("big_object", &big_object);
    context.add("loop", &loop_control);

    b.iter(|| tera.render("huge.html", &context));
}

