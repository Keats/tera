#[macro_use]
extern crate serde_derive;
extern crate tera;

use tera::{Context, Result, Tera};

mod common;
use common::{Product, Review};

fn render_tpl(tpl_name: &str) -> Result<String> {
    let tera = Tera::new("tests/render-failures/**/*").unwrap();
    let mut context = Context::new();
    context.add("product", &Product::new());
    context.add("username", &"bob");
    context.add("friend_reviewed", &true);
    context.add("number_reviews", &2);
    context.add("show_more", &true);
    context.add("reviews", &vec![Review::new(), Review::new()]);

    tera.render(tpl_name, &context)
}

#[test]
fn test_error_render_field_unknown() {
    let result = render_tpl("field_unknown.html");

    assert_eq!(result.is_err(), true);
    assert_eq!(
        result.unwrap_err().iter().nth(1).unwrap().description(),
        "Variable `hey` not found in context while rendering \'field_unknown.html\'"
    );
}

#[test]
fn test_error_render_field_unknown_in_forloop() {
    let result = render_tpl("field_unknown_forloop.html");

    assert_eq!(result.is_err(), true);
    let err = result.unwrap_err();
    assert_eq!(
        err.iter().nth(1).unwrap().description(),
        "Variable lookup failed in forloop for `r.random`"
    );
    assert_eq!(
        err.iter().nth(2).unwrap().description(),
        "Variable `random` not found in context while rendering \'field_unknown_forloop.html\'"
    );
}

#[test]
fn test_error_render_non_math() {
    let result = render_tpl("non_math_operation.html");

    assert_eq!(result.is_err(), true);
    assert_eq!(
        result.unwrap_err().iter().nth(1).unwrap().description(),
        "Variable `username` was used in a math operation but is not a number"
    );
}

#[test]
fn test_error_render_filter_section_invalid() {
    let result = render_tpl("filter_section_invalid.html");

    assert_eq!(result.is_err(), true);
    assert_eq!(
        result.unwrap_err().iter().nth(1).unwrap().description(),
        "Filter `round` was called on an incorrect value: got `\"hello\"` but expected a f64"
    );
}

#[test]
fn test_error_render_iterate_non_array() {
    let result = render_tpl("iterate_on_non_array.html");

    assert_eq!(result.is_err(), true);
    assert_eq!(
        result.unwrap_err().iter().nth(1).unwrap().description(),
        "Tried to iterate on a container (`username`) that has a unsupported type"
    );
}

#[test]
fn test_error_render_serialize_non_object() {
    let tera = Tera::new("tests/render-failures/**/*").unwrap();
    let result = tera.render("value_render_non_object.html", &[1, 2, 3]);

    assert_eq!(result.is_err(), true);
    assert_eq!(
        result.unwrap_err().iter().nth(0).unwrap().description(),
        "Failed to render \'value_render_non_object.html\': context isn\'t a JSON object. \
         The value passed needs to be a key-value object: context, struct, hashmap for example."
    );
}

#[test]
fn test_error_wrong_args_macros() {
    let result = render_tpl("macro_wrong_args.html");

    assert_eq!(result.is_err(), true);
    assert!(
        result
            .unwrap_err()
            .iter()
            .nth(1)
            .unwrap()
            .description()
            .contains("Macro `input` is missing the argument")
    );
}

#[test]
fn test_error_macros_self_inexisting() {
    let result = render_tpl("macro_self_inexisting.html");

    assert_eq!(result.is_err(), true);
    assert_eq!(
        result.unwrap_err().iter().nth(1).unwrap().description(),
        "Macro `inexisting` was not found in the namespace `macros`"
    );
}

#[test]
fn test_error_in_child_template_location() {
    let result = render_tpl("error-location/error_in_child.html");

    assert_eq!(result.is_err(), true);
    let errs = result.unwrap_err();
    assert_eq!(
        errs.iter().nth(0).unwrap().description(),
        "Failed to render 'error-location/error_in_child.html'"
    );
}

#[test]
fn test_error_in_grandchild_template_location() {
    let result = render_tpl("error-location/error_in_grand_child.html");

    assert_eq!(result.is_err(), true);
    let errs = result.unwrap_err();
    assert_eq!(
        errs.iter().nth(0).unwrap().description(),
        "Failed to render 'error-location/error_in_grand_child.html'"
    );
}

#[test]
fn test_error_in_parent_template_location() {
    let result = render_tpl("error-location/error_in_parent.html");

    assert_eq!(result.is_err(), true);
    let errs = result.unwrap_err();
    assert_eq!(
        errs.iter().nth(0).unwrap().description(),
        "Failed to render 'error-location/error_in_parent.html' (error happened in a parent template)"
    );
}

#[test]
fn test_error_in_macro_location() {
    let result = render_tpl("error-location/error_in_macro.html");

    assert_eq!(result.is_err(), true);
    let errs = result.unwrap_err();
    assert_eq!(
        errs.iter().nth(0).unwrap().description(),
        "Failed to render 'error-location/error_in_macro.html': error while rendering a macro from the `macros` namespace"
    );
}
