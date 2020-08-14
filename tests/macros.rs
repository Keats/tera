extern crate tera;

use tera::{Context, Result, Tera};

fn render_tpl(tpl_name: &str) -> Result<String> {
    let tera = Tera::new("tests/macros/**/*.html").unwrap();

    tera.render(tpl_name, &Context::new())
}

#[test]
fn test_extend_and_include1() {
    // trivial case: inheritance, only one macro import
    let result = render_tpl("extend_and_include1.html");

    assert_eq!(result.is_err(), false);
    assert_eq!(result.unwrap(), "hello");
}

#[test]
fn test_extend_and_include2() {
    // inheritance, two macro imports, a and b, a is used, a uses self
    let result = render_tpl("extend_and_include2.html");

    assert_eq!(result.is_err(), false);
    assert_eq!(result.unwrap(), "hello");
}

#[test]
fn test_extend_and_include3() {
    // inheritance, two macro imports, b and a, a is used, a uses self
    let result = render_tpl("extend_and_include3.html");

    assert_eq!(result.is_err(), false);
    assert_eq!(result.unwrap(), "hello");
}

#[test]
fn test_extend_and_include4() {
    // like test_extend_and_include3 but no inheritance for completeness
    let result = render_tpl("extend_and_include4.html");

    assert_eq!(result.is_err(), false);
    assert_eq!(result.unwrap(), "hello");
}

#[test]
fn test_extend_and_include_nested1() {
    // inheritance, a macro import, another nested import, nested uses self
    let result = render_tpl("extend_and_include_nested1.html");

    assert_eq!(result.is_err(), false);
    assert_eq!(result.unwrap(), "hello");
}

#[test]
fn test_extend_and_include_nested2() {
    // like test_extend_and_include_nested1 but no inheritance
    let result = render_tpl("extend_and_include_nested2.html");

    assert_eq!(result.is_err(), false);
    assert_eq!(result.unwrap(), "hello");
}
