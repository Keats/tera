extern crate tera;
extern crate glob;

use std::io::prelude::*;
use std::fs::File;

use tera::{Tera, Context};

mod common;
use common::{Product, Review, read_file};


fn assert_template_ok(path: &str, base_path: Option<&str>) {
    let mut tera = Tera::default();
    tera.add_template("tpl", &read_file(path));
    if base_path.is_some() {
        let base = base_path.unwrap().to_string();
        let split = base.split("/").collect::<Vec<&str>>();
        let name = split.last().unwrap();
        tera.add_template(name, &read_file(&base));
    }
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

    let rendered = tera.render("tpl", context).unwrap();
    if rendered != expected {
        println!("Template {:?} was rendered incorrectly", path);
        println!("Got: \n {:#?}", rendered);
        println!("Expected: \n {:#?}", expected);
        // Uncomment below to save ouput to html file since
        // we don't ignore whitespace right now it's a bit tricky to get
        // the exact \n and spacing
        let mut file = File::create("out.html").unwrap();
        file.write_all(rendered.as_bytes()).unwrap();
        assert!(false);
    }
}

#[test]
fn test_ok_basic_template() {
    assert_template_ok("tests/templates/basic.html", None);
}

#[test]
fn test_ok_comment_template() {
    assert_template_ok("tests/templates/comment.html", None);
}

#[test]
fn test_ok_comment_alignment_template() {
    assert_template_ok("tests/templates/comment_alignment.html", None);
}

#[test]
fn test_ok_variables_template() {
    assert_template_ok("tests/templates/variables.html", None);
}

#[test]
fn test_ok_conditions_template() {
    assert_template_ok("tests/templates/conditions.html", None);
}

#[test]
fn test_ok_loops_template() {
    assert_template_ok("tests/templates/loops.html", None);
}

#[test]
fn test_ok_empty_loop_template() {
    assert_template_ok("tests/templates/empty_loop.html", None);
}

#[test]
fn test_ok_basic_inheritance_template() {
    assert_template_ok(
        "tests/templates/basic_inheritance.html",
        Some("tests/templates/base.html")
    );
}

#[test]
fn test_ok_raw_template() {
    assert_template_ok("tests/templates/raw.html", None);
}

#[test]
fn test_ok_filters_template() {
    assert_template_ok("tests/templates/filters.html", None);
}

#[test]
fn test_ok_variable_tests() {
    assert_template_ok("tests/templates/variable_tests.html", None);
}

#[test]
fn test_ok_indexing() {
    assert_template_ok("tests/templates/indexing.html", None);
}
