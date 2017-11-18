use pest::Parser;

use parser::parse;


#[test]
fn invalid_number() {
    let res = parse("{{ 1.2.2 }}");
    assert!(res.is_err());
    let err = res.unwrap_err();
    assert!(err.description().contains("line 1, col 7"));
}


#[test]
fn unterminated() {
    let res = parse("{{ hey");
    assert!(res.is_err());
    let err = res.unwrap_err();
    assert!(err.description().contains("line 1, col 7"));
}

#[test]
fn invalid_macro_content() {
    let res = parse(r#"
{% macro input(label, type) %}
    {% macro nested() %}
    {% endmacro nested %}
{% endmacro input %}
    "#);
    assert!(res.is_err());
    let err = res.unwrap_err();
    println!("{}", err.description());
    assert!(err.description().contains("line 3, col 8"));
}

#[test]
fn invalid_elif() {
    let res = parse(r#"
{% if true %}
{% else %}
{% elif false %}
{% endif %}
    "#);
    assert!(res.is_err());
    let err = res.unwrap_err();
    println!("{}", err.description());
    assert!(err.description().contains("line 4, col 4"));
}

#[test]
fn invalid_else() {
    let res = parse(r#"
{% if true %}
{% else %}
{% else %}
{% endif %}
    "#);
    assert!(res.is_err());
    let err = res.unwrap_err();
    println!("{}", err.description());
    assert!(err.description().contains("line 4, col 4"));
}

#[test]
fn invalid_extends() {
    let res = parse(r#"
Hello
{% extends "something.html" %}
    "#);
    assert!(res.is_err());
    let err = res.unwrap_err();
    println!("{}", err.description());
    assert!(err.description().contains("line 3, col 4"));
}

#[test]
fn invalid_operator() {
    let res = parse("{{ hey =! }}");
    assert!(res.is_err());
    let err = res.unwrap_err();
    println!("{}", err.description());
    assert!(err.description().contains("line 1, col 8"));
}

#[test]
fn missing_expression_with_not() {
    let res = parse("{% if not %}");
    assert!(res.is_err());
    let err = res.unwrap_err();
    println!("{}", err.description());
    assert!(err.description().contains("line 1, col 11"));
}
