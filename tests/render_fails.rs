extern crate tera;
#[macro_use]
extern crate serde_derive;

use tera::{Tera, Context, Result};

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
        "Field `hey` not found in context while rendering \'field_unknown.html\'"
    );
}

#[test]
fn test_error_render_field_unknown_in_forloop() {
    let result = render_tpl("field_unknown_forloop.html");

    assert_eq!(result.is_err(), true);
    assert_eq!(
        result.unwrap_err().iter().nth(1).unwrap().description(),
        "Field `random` not found in context while rendering \'field_unknown_forloop.html\'"
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
    println!("{:?}", result);
    assert_eq!(
        result.unwrap_err().iter().nth(1).unwrap().description(),
        "Filter `round` was called on an incorrect value: got `\"\\nhello\\n\"` but expected a f64"
    );
}

#[test]
fn test_error_render_iterate_non_array() {
    let result = render_tpl("iterate_on_non_array.html");

    assert_eq!(result.is_err(), true);
    assert_eq!(
        result.unwrap_err().iter().nth(1).unwrap().description(),
        "Tried to iterate on variable `username`, but it isn\'t an array"
    );
}

#[test]
fn test_error_render_serialize_non_object() {
    let tera = Tera::new("tests/render-failures/**/*").unwrap();
    let result = tera.render("value_render_non_object.html", &[1,2,3]);

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
    assert_eq!(
        result.unwrap_err().iter().nth(1).unwrap().description(),
        "Macro `input` got `[\"label\", \"type\"]` for args but was expecting `[\"greeting\"]` (order does not matter)"
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
