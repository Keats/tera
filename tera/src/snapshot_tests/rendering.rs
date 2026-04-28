use serde::Serialize;
use std::collections::HashMap;

use crate::delimiters::Delimiters;
use crate::snapshot_tests::utils::{create_multi_templates_tera, normalize_line_endings};
use crate::tera::Tera;

#[cfg(not(feature = "preserve_order"))]
use crate::args::Kwargs;

#[cfg(not(feature = "preserve_order"))]
use crate::vm::state::State;
use crate::{Context, Value};

#[derive(Debug, Serialize)]
pub struct Product {
    name: String,
}
impl Product {
    pub fn new() -> Product {
        Product {
            name: "Moto G".to_owned(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Review {
    title: String,
    paragraphs: Vec<String>,
}
impl Review {
    pub fn new() -> Review {
        Review {
            title: "My review".to_owned(),
            paragraphs: vec!["A".to_owned(), "B".to_owned(), "C".to_owned()],
        }
    }
}

#[derive(Debug, Serialize)]
pub struct NestedObject {
    pub label: String,
    pub parent: Option<Box<NestedObject>>,
    pub numbers: Vec<usize>,
}

#[derive(Debug, Serialize)]
pub struct YearData {
    id: usize,
    year: Option<usize>,
}

fn get_context() -> Context {
    let mut context = Context::new();
    context.insert("name", &"Bob");
    context.insert("description", &"<p>I should be escaped by default</p>");
    context.insert("some_html", &"<p>Some HTML chars & more</p>");
    context.insert("age", &18);
    context.insert("some_bool", &true);
    context.insert("one", &1);
    context.insert("product", &Product::new());
    context.insert("vectors", &vec![vec![0, 3, 6], vec![1, 4, 7]]);
    context.insert("numbers", &vec![1, 2, 3]);
    context.insert("empty", &Vec::<usize>::new());
    let parent = NestedObject {
        label: "Parent".to_string(),
        parent: None,
        numbers: vec![1, 2, 3],
    };
    let child = NestedObject {
        label: "Child".to_string(),
        parent: Some(Box::new(parent)),
        numbers: vec![1, 2, 3],
    };
    context.insert("objects", &vec![child]);
    let mut data: HashMap<String, Value> = HashMap::new();
    data.insert(
        "names".to_string(),
        vec![
            "Tchoupi".to_string(),
            "Pilou".to_string(),
            "Fanny".to_string(),
        ]
        .into(),
    );
    data.insert("weights".to_string(), vec![50.6, 70.1].into());
    context.insert("data", &data);
    context.insert("reviews", &vec![Review::new(), Review::new()]);
    context.insert("to", &"&");
    context.insert("malicious", &"<html>");
    context.insert(
        "year_data",
        &vec![
            YearData {
                id: 1,
                year: Some(2015),
            },
            YearData {
                id: 2,
                year: Some(2015),
            },
            YearData {
                id: 3,
                year: Some(2016),
            },
            YearData {
                id: 4,
                year: Some(2017),
            },
            YearData {
                id: 5,
                year: Some(2018),
            },
            YearData { id: 6, year: None },
            YearData {
                id: 7,
                year: Some(2018),
            },
        ],
    );
    context
}

// Disable those tests with preserve_order since the order of printed maps would change
// and fail
#[cfg(not(feature = "preserve_order"))]
#[test]
fn rendering_ok() {
    insta::glob!("rendering_inputs/success/*.txt*", |path| {
        let contents = std::fs::read_to_string(path).unwrap();
        let normalized_contents = normalize_line_endings(&contents);
        let p = format!("{}", path.file_name().unwrap().to_string_lossy());
        let mut tera = Tera::default();
        tera.autoescape_on(vec![".txt"]);
        // Register filter before adding templates that use it
        // Test filter using State::get<T> to read from context with path support
        tera.register_filter("read_ctx", |x: &str, _: Kwargs, state: &State| {
            if let Some((start, rest)) = x.split_once('.') {
                let base: Value = state.get(start)?.unwrap_or(Value::undefined());
                Ok(base.get_from_path(rest))
            } else {
                Ok(state.get::<Value>(x)?.unwrap_or(Value::undefined()))
            }
        });
        tera.add_raw_templates(vec![(&p, normalized_contents)])
            .unwrap();
        let out = tera.render(&p, &get_context()).unwrap();
        let normalized_out = normalize_line_endings(&out);
        insta::assert_snapshot!(&normalized_out);
    });
}

#[test]
fn rendering_components_ok() {
    insta::glob!("rendering_inputs/success/components/*.txt", |path| {
        println!("{path:?}");
        let contents = std::fs::read_to_string(path).unwrap();
        let (tera, tpl_name) = create_multi_templates_tera(&contents);
        let out = tera.render(&tpl_name, &get_context()).unwrap();
        insta::assert_snapshot!(&out);
    });
}

#[test]
fn rendering_inheritance_ok() {
    insta::glob!("rendering_inputs/success/inheritance/*.txt", |path| {
        println!("{path:?}");
        let contents = std::fs::read_to_string(path).unwrap();
        let normalized_contents = normalize_line_endings(&contents);
        let (tera, tpl_name) = create_multi_templates_tera(&normalized_contents);
        let out = tera.render(&tpl_name, &get_context()).unwrap();
        let normalized_out = normalize_line_endings(&out);
        insta::assert_snapshot!(&normalized_out);
    });
}

#[test]
fn rendering_errors() {
    insta::glob!("rendering_inputs/errors/*.txt", |path| {
        let contents = std::fs::read_to_string(path).unwrap();
        let normalized_contents = normalize_line_endings(&contents);
        let p = format!("{}", path.file_name().unwrap().to_string_lossy());
        let mut tera = Tera::default();
        tera.add_raw_templates(vec![(&p, normalized_contents)])
            .unwrap();
        let err = tera.render(&p, &get_context()).unwrap_err();
        insta::assert_snapshot!(&err);
    });
}

#[test]
fn rendering_inheritance_errors() {
    insta::glob!("rendering_inputs/errors/inheritance/*.txt", |path| {
        let contents = std::fs::read_to_string(path).unwrap();
        let normalized_contents = normalize_line_endings(&contents);
        let (tera, tpl_name) = create_multi_templates_tera(&normalized_contents);
        let err = tera.render(&tpl_name, &get_context()).unwrap_err();
        insta::assert_snapshot!(&err);
    });
}

#[test]
fn rendering_components_errors() {
    insta::glob!("rendering_inputs/errors/components/*.txt", |path| {
        let contents = std::fs::read_to_string(path).unwrap();
        let (tera, tpl_name) = create_multi_templates_tera(&contents);
        let err = tera.render(&tpl_name, &get_context()).unwrap_err();
        insta::assert_snapshot!(&err);
    });
}

#[test]
fn rendering_include_errors() {
    insta::glob!("rendering_inputs/errors/include/*.txt", |path| {
        let contents = std::fs::read_to_string(path).unwrap();
        let (tera, tpl_name) = create_multi_templates_tera(&contents);
        let err = tera.render(&tpl_name, &get_context()).unwrap_err();
        insta::assert_snapshot!(&err);
    });
}

#[cfg(feature = "unicode")]
#[test]
fn can_iterate_on_graphemes() {
    let tpl = r#"{% for c in string -%}
{{loop.index}}.{{c}}
{% endfor %}"#;
    let mut tera = Tera::default();
    tera.add_raw_template("tpl", tpl).unwrap();
    let mut context = Context::default();
    // s.chars() would give ['न', 'म', 'स', '्', 'त', 'े']
    // graphemes per UAX #29 are ["न", "म", "स्ते"]
    context.insert("string", "नमस्ते");
    let out = tera.render("tpl", &context).unwrap();
    let normalized_out = normalize_line_endings(&out);

    insta::assert_snapshot!(&normalized_out);
}

#[cfg(feature = "unicode")]
#[test]
fn can_slice_on_graphemes() {
    let tpl = r#"
{{ string[::-1] }}
{{ string[1:] }}
{{ string[:-1] }}
"#;
    let mut tera = Tera::default();
    tera.add_raw_template("tpl", tpl).unwrap();
    let mut context = Context::default();
    context.insert("string", "नमस्ते");
    let out = tera.render("tpl", &context).unwrap();
    let normalized_out = normalize_line_endings(&out);

    insta::assert_snapshot!(&normalized_out);
}

#[cfg(feature = "preserve_order")]
#[test]
fn inline_map_preserve_order() {
    let tpl = r#"
{% set m = {"name": "Alex", "age": 42, "vip": true, } -%}
{{ m }}
{% for k, v in m -%}
{{ k }} = {{ v }}
{% endfor -%}
"#;
    let mut tera = Tera::default();
    tera.add_raw_template("tpl", tpl).unwrap();
    let context = Context::default();
    let out = tera.render("tpl", &context).unwrap();
    let normalized_out = normalize_line_endings(&out);

    insta::assert_snapshot!(&normalized_out);
}

#[test]
fn rendering_custom_delimiters() {
    let tpl = r#"Hello, << name >>!
<% if some_bool %>Bool is true<% endif %>
<% for num in numbers %>[<< num >>]<% endfor %>
<# This comment should not appear #>
Age: << age >>
<<- " trimmed " ->>
<% raw %><<not a variable>><% endraw %>"#;

    let mut tera = Tera::default();
    tera.set_delimiters(Delimiters {
        block_start: "<%".into(),
        block_end: "%>".into(),
        variable_start: "<<".into(),
        variable_end: ">>".into(),
        comment_start: "<#".into(),
        comment_end: "#>".into(),
    })
    .unwrap();
    tera.add_raw_template("custom_delimiters.txt", tpl).unwrap();

    let out = tera
        .render("custom_delimiters.txt", &get_context())
        .unwrap();
    let normalized_out = normalize_line_endings(&out);
    insta::assert_snapshot!(&normalized_out);
}

#[test]
fn render_str_errors() {
    let tera = Tera::default();
    let mut ctx = Context::new();
    ctx.insert("x", &1);

    let out = tera.render_str(r#"{{ youtube() }}"#, &ctx, false);
    assert!(out.is_err());

    let out = tera.render_str(r#"{{ "hello" | unknown_filter }}"#, &ctx, false);
    assert!(out.is_err());

    let out = tera.render_str(r#"{% if x is unknown_test %}yes{% endif %}"#, &ctx, false);
    assert!(out.is_err());

    let out = tera.render_str(r#"{{<Unknown />}}"#, &ctx, false);
    assert!(out.is_err());

    let out = tera.render_str(r#"{% include "missing.html" %}"#, &ctx, false);
    assert!(out.is_err());
}
