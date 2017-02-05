#[macro_use]
extern crate serde_derive;
extern crate tera;
extern crate glob;


use std::io::prelude::*;
use std::fs::File;

use tera::{Tera, Context};

mod common;
use common::{Product, Review, read_file};


fn assert_template_ok(path: &str, others: Vec<&str>) {
    let mut tera = Tera::default();
    tera.autoescape_on(vec!["html"]);

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
    context.add("a_tuple", &(1, 2, 3));
    context.add("an_array_of_tuple", &vec![(1, 2, 3), (1, 2, 3)]);
    let empty: Vec<Review> = Vec::new();
    context.add("empty", &empty);

    let rendered = tera.render("tpl.html", &context).unwrap();
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
