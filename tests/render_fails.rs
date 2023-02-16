extern crate tera;
#[macro_use]
extern crate serde_derive;

use std::error::Error;
use tera::{Context, Result, Tera};

mod common;
use crate::common::{Product, Review};

fn render_tpl(tpl_name: &str) -> Result<String> {
    let tera = Tera::new("tests/render-failures/**/*").unwrap();
    let mut context = Context::new();
    context.insert("product", &Product::new());
    context.insert("username", &"bob");
    context.insert("friend_reviewed", &true);
    context.insert("number_reviews", &2);
    context.insert("show_more", &true);
    context.insert("reviews", &vec![Review::new(), Review::new()]);

    tera.render(tpl_name, &context)
}

#[test]
fn test_error_render_field_unknown() {
    let result = render_tpl("field_unknown.html");

    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().source().unwrap().to_string(),
        "Variable `hey` not found in context while rendering \'field_unknown.html\'"
    );
}

#[test]
fn test_error_render_field_unknown_in_forloop() {
    let result = render_tpl("field_unknown_forloop.html");

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(
        err.source().unwrap().to_string(),
        "Variable `r.random` not found in context while rendering \'field_unknown_forloop.html\'"
    );
}

#[test]
fn test_error_render_non_math() {
    let result = render_tpl("non_math_operation.html");

    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().source().unwrap().to_string(),
        "Variable `username` was used in a math operation but is not a number"
    );
}

#[test]
fn test_error_render_filter_section_invalid() {
    let result = render_tpl("filter_section_invalid.html");
    assert!(result.is_err());
    let err = result.unwrap_err();
    let source = err.source().unwrap();

    assert_eq!(source.to_string(), "Filter call \'round\' failed");
    let source2 = source.source().unwrap();
    assert_eq!(
        source2.to_string(),
        "Filter `round` was called on an incorrect value: got `\"hello\"` but expected a f64"
    );
}

#[test]
fn test_error_render_iterate_non_array() {
    let result = render_tpl("iterate_on_non_array.html");

    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().source().unwrap().to_string(),
        "Tried to iterate on a container (`friend_reviewed`) that has a unsupported type"
    );
}

#[test]
fn test_error_wrong_args_macros() {
    let result = render_tpl("macro_wrong_args.html");

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .source()
        .unwrap()
        .to_string()
        .contains("Macro `input` is missing the argument"));
}

#[test]
fn test_error_macros_self_inexisting() {
    let result = render_tpl("macro_self_inexisting.html");

    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().source().unwrap().to_string(),
        "Macro `self::inexisting` not found in template `macros.html`"
    );
}

#[test]
fn test_error_in_child_template_location() {
    let result = render_tpl("error-location/error_in_child.html");

    assert!(result.is_err());
    let errs = result.unwrap_err();
    assert_eq!(errs.to_string(), "Failed to render 'error-location/error_in_child.html'");
}

#[test]
fn test_error_in_grandchild_template_location() {
    let result = render_tpl("error-location/error_in_grand_child.html");

    assert!(result.is_err());
    let errs = result.unwrap_err();
    assert_eq!(errs.to_string(), "Failed to render 'error-location/error_in_grand_child.html'");
}

#[test]
fn test_error_in_parent_template_location() {
    let result = render_tpl("error-location/error_in_parent.html");

    assert!(result.is_err());
    let errs = result.unwrap_err();
    assert_eq!(
        errs.to_string(),
        "Failed to render 'error-location/error_in_parent.html' (error happened in a parent template)"
    );
}

#[test]
fn test_error_in_macro_location() {
    let result = render_tpl("error-location/error_in_macro.html");

    assert!(result.is_err());
    let errs = result.unwrap_err();
    assert_eq!(
        errs.to_string(),
        "Failed to render 'error-location/error_in_macro.html': error while rendering macro `macros::cause_error`"
    );
}
