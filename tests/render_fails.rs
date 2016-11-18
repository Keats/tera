extern crate tera;

use std::error::Error;

use tera::{Tera, Context, TeraResult};

mod common;
use common::{Product, Review};

fn render_tpl(tpl_name: &str) -> TeraResult<String> {
    let tera = Tera::new("tests/render-failures/**/*");
    let mut context = Context::new();
    context.add("product", &Product::new());
    context.add("username", &"bob");
    context.add("friend_reviewed", &true);
    context.add("number_reviews", &2);
    context.add("show_more", &true);
    context.add("reviews", &vec![Review::new(), Review::new()]);

    tera.render(tpl_name, context)
}

#[test]
fn test_error_render_parent_inexistent() {
    let result = render_tpl("inexisting_parent.html");

    assert_eq!(result.is_err(), true);
    assert_eq!(result.unwrap_err().description(), "template not found".to_string());
}

#[test]
fn test_error_render_field_unknown() {
    let result = render_tpl("field_unknown.html");

    assert_eq!(result.is_err(), true);
    assert_eq!(result.unwrap_err().description(), "field not found".to_string());
}

#[test]
fn test_error_render_field_unknown_in_forloop() {
    let result = render_tpl("field_unknown_forloop.html");

    assert_eq!(result.is_err(), true);
    assert_eq!(result.unwrap_err().description(), "field not found".to_string());
}

#[test]
fn test_error_render_non_math() {
    let result = render_tpl("non_math_operation.html");

    assert_eq!(result.is_err(), true);
    assert_eq!(result.unwrap_err().description(), "field is not a number".to_string());
}

#[test]
fn test_error_render_iterate_non_array() {
    let result = render_tpl("iterate_on_non_array.html");

    assert_eq!(result.is_err(), true);
    assert_eq!(result.unwrap_err().description(), "field is not an array".to_string());
}

#[test]
fn test_error_render_include_inexistent() {
    let result = render_tpl("inexisting_include.html");

    assert_eq!(result.is_err(), true);
    assert_eq!(result.unwrap_err().description(), "template not found".to_string());
}

#[test]
fn test_error_value_render_non_object() {
    let tera = Tera::new("tests/render-failures/**/*");
    let result = tera.value_render("value_render_non_object.html", &[1,2,3]);

    assert_eq!(result.is_err(), true);
    assert_eq!(result.unwrap_err().description(), "invalid value".to_string());
}

#[test]
fn test_error_wrong_args_macros() {
    let result = render_tpl("macro_wrong_args.html");

    assert_eq!(result.is_err(), true);
    assert_eq!(result.unwrap_err().description(), "macro wrong args".to_string());
}


#[test]
fn test_error_macros_self_inexisting() {
    let result = render_tpl("macro_self_inexisting.html");

    assert_eq!(result.is_err(), true);
    assert_eq!(result.unwrap_err().description(), "macro not found".to_string());
}
