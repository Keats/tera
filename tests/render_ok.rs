#[macro_use]
extern crate serde_derive;
extern crate tera;
extern crate glob;


use std::io::prelude::*;
use std::fs::File;
use std::collections::BTreeMap;

use tera::{Tera, Result, Context, GlobalFn, Value, to_value, from_value};

mod common;
use common::{Product, Review, read_file};


fn make_url_for(urls: BTreeMap<String, String>) -> GlobalFn {
    Box::new(move |args| -> Result<Value> {
        match args.get("name") {
            Some(val) => match from_value::<String>(val.clone()) {
                Ok(v) =>  Ok(to_value(urls.get(&v).unwrap()).unwrap()),
                Err(_) => Err("oops".into()),
            },
            None => Err("oops".into()),
        }
    })
}

fn assert_template_ok(path: &str, others: Vec<&str>) {
    let mut tera = Tera::default();
    tera.autoescape_on(vec!["html"]);
    let mut urls = BTreeMap::new();
    urls.insert("home".to_string(), "vincent.is".to_string());
    tera.register_global_function("url_for", make_url_for(urls));

    for p in others {
        let base = p.to_string();
        let split = base.split('/').collect::<Vec<&str>>();
        let name = split.last().unwrap();
        tera.add_raw_template(name, &read_file(&base)).unwrap();
    }
    tera.add_raw_template("tpl.html", &read_file(path)).unwrap();
    let expected = read_file(&path.replace("templates", "expected"));

    let mut context = Context::new();
    context.add("product", &Product::new());
    context.add("username", &"bob");
    context.add("friend_reviewed", &true);
    context.add("number_reviews", &2);
    context.add("show_more", &true);
    context.add("reviews", &vec![Review::new(), Review::new()]);
    let mut map = BTreeMap::new();
    map.insert("bob", "comment 1");
    map.insert("jane", "comment 2");
    context.add("comments", &map);
    context.add("a_tuple", &(1, 2, 3));
    context.add("an_array_of_tuple", &vec![(1, 2, 3), (1, 2, 3)]);
    let empty: Vec<Review> = Vec::new();
    context.add("empty", &empty);

    let rendered = tera.render("tpl.html", &context).unwrap();
    // replace to make tests pass in windows
    if rendered.replace("\r\n", "\n") != expected.replace("\r\n", "\n") {
        println!("Template {:?} was rendered incorrectly", path);
        println!("Got: \n {:#?}", rendered);
        println!("Expected: \n {:#?}", expected);
        let mut file = File::create("out.html").unwrap();
        file.write_all(rendered.as_bytes()).unwrap();
        assert!(false);
    }
}

#[test]
fn test_ok_basic_template() {
    assert_template_ok("tests/templates/basic.html", vec![]);
}

#[test]
fn test_ok_comment_template() {
    assert_template_ok("tests/templates/comment.html", vec![]);
}

#[test]
fn test_ok_comment_alignment_template() {
    assert_template_ok("tests/templates/comment_alignment.html", vec![]);
}

#[test]
fn test_ok_variables_template() {
    assert_template_ok("tests/templates/variables.html", vec![]);
}

#[test]
fn test_ok_conditions_template() {
    assert_template_ok("tests/templates/conditions.html", vec![]);
}

#[test]
fn test_ok_loops_template() {
    assert_template_ok("tests/templates/loops.html", vec![]);
}

#[test]
fn test_ok_empty_loop_template() {
    assert_template_ok("tests/templates/empty_loop.html", vec![]);
}

#[test]
fn test_ok_loop_with_filter_template() {
    assert_template_ok("tests/templates/loop_with_filters.html", vec![]);
}

#[test]
fn test_ok_basic_inheritance_template() {
    assert_template_ok(
        "tests/templates/basic_inheritance.html",
        vec!["tests/templates/base.html"]
    );
}

#[test]
fn test_ok_raw_template() {
    assert_template_ok("tests/templates/raw.html", vec![]);
}

#[test]
fn test_ok_filters_template() {
    assert_template_ok("tests/templates/filters.html", vec![]);
}

#[test]
fn test_ok_variable_tests() {
    assert_template_ok("tests/templates/variable_tests.html", vec![]);
}

#[test]
fn test_ok_indexing() {
    assert_template_ok("tests/templates/indexing.html", vec![]);
}

#[test]
fn test_ok_include_template() {
    assert_template_ok("tests/templates/include.html", vec!["tests/templates/included.html"]);
}

#[test]
fn test_ok_render_struct_data() {
    let path = "tests/templates/value_render.html";
    let mut tera = Tera::default();
    tera.add_raw_template("tpl", &read_file(path)).unwrap();
    let expected = read_file(&path.replace("templates", "expected"));
    let rendered = tera.render("tpl", &Product::new()).unwrap();
    if rendered != expected {
        println!("Template {:?} was rendered incorrectly", path);
        println!("Got: \n {:#?}", rendered);
        println!("Expected: \n {:#?}", expected);
        let mut file = File::create("out.html").unwrap();
        file.write_all(rendered.as_bytes()).unwrap();
        assert!(false);
    }
}

#[test]
fn test_ok_macros() {
    assert_template_ok(
        "tests/templates/use_macros.html",
        vec!["tests/templates/macros.html", "tests/templates/macro_included.html"]
    );
}

#[test]
fn test_magical_variable_dumps_context() {
    assert_template_ok(
        "tests/templates/magical_variable.html",
        vec!["tests/templates/macros.html"]
    );
}

#[test]
fn test_ok_closure_global_fn() {
    assert_template_ok("tests/templates/global_fn.html", vec![]);
}
