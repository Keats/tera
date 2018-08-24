use std::collections::BTreeMap;
use context::Context;
use errors::Result;
use tera::Tera;
use std::fmt::Debug;
use error_chain::ChainedError;

#[test]
fn error_location_basic() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("tpl", "{{ 1 + true }}"),
    ]).unwrap();

    let result = tera.render("tpl", &Context::new());

    assert_eq!(
        result.unwrap_err().iter().nth(0).unwrap().description(),
        "Failed to render \'tpl\'"
    );
}

#[test]
fn error_location_inside_macro() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("macros", "{% macro hello()%}{{ 1 + true }}{% endmacro hello %}"),
        ("tpl", "{% import \"macros\" as macros %}{{ macro::hello() }}"),
    ]).unwrap();

    let result_text = result_text(tera.render("tpl", &Context::new()));
    assert!(result_text.contains("Error: Failed to render 'tpl'"));
    assert!(result_text.contains("Caused by: Macro `(macro:hello)` not found in template `tpl`"));
}

#[test]
fn error_location_base_template() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("parent", "Hello {{ greeting + 1}} {% block bob %}{% endblock bob %}"),
        ("child", "{% extends \"parent\" %}{% block bob %}Hey{% endblock bob %}"),
    ]).unwrap();

    let result = tera.render("child", &Context::new());

    assert_eq!(
        result.unwrap_err().iter().nth(0).unwrap().description(),
        "Failed to render \'child\' (error happened in 'parent')."
    );
}

#[test]
fn error_location_in_parent_block() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("parent", "Hello {{ greeting }} {% block bob %}{{ 1 + true }}{% endblock bob %}"),
        ("child", "{% extends \"parent\" %}{% block bob %}{{ super() }}Hey{% endblock bob %}"),
    ]).unwrap();

    let result = tera.render("child", &Context::new());

    assert_eq!(
        result.unwrap_err().iter().nth(0).unwrap().description(),
        "Failed to render \'child\' (error happened in 'parent')."
    );
}

#[test]
fn error_location_in_parent_in_macro() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("macros", "{% macro hello()%}{{ 1 + true }}{% endmacro hello %}"),
        ("parent", "{% import \"macros\" as macros %}{{ macro::hello() }}{% block bob %}{% endblock bob %}"),
        ("child", "{% extends \"parent\" %}{% block bob %}{{ super() }}Hey{% endblock bob %}"),
    ]).unwrap();

    let result_text = result_text(tera.render("child", &Context::new()));
    assert!(result_text.contains("Error: Failed to render 'child' (error happened in 'parent')."));
    assert!(result_text.contains("Caused by: Macro `(macro:hello)` not found in template `child`"));
}

#[test]
fn error_out_of_range_index() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("tpl", "{{ arr[10] }}"),
    ]).unwrap();
    let mut context = Context::new();
    context.add("arr", &[1, 2, 3]);

    let result_text = result_text(tera.render("tpl", &Context::new()));
    assert!(result_text.contains("Error: Failed to render 'tpl'"));
    assert!(result_text.contains("Caused by: Unable to find variable `arr`"));
}

#[test]
fn error_unknown_index_variable() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("tpl", "{{ arr[a] }}"),
    ]).unwrap();
    let mut context = Context::new();
    context.add("arr", &[1, 2, 3]);

    let result_text = result_text(tera.render("tpl", &Context::new()));
    assert!(result_text.contains("Error: Failed to render 'tpl'"));
    assert!(result_text.contains("Unable to find variable `arr`"));
}

#[test]
fn error_invalid_type_index_variable() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("tpl", "{{ arr[a] }}"),
    ]).unwrap();

    let mut context = Context::new();
    context.add("arr", &[1, 2, 3]);
    context.add("a", &true);

    let result_text = result_text(tera.render("tpl", &context));
    assert!(result_text.contains("Only variables evaluating to String or Number can be used as index -> [a] which is Bool"));
}

fn result_text<T>(result: Result<T>) -> String
where
    T: Debug,
{
    result
        .unwrap_err()
        .display_chain()
        .to_string()
        .trim()
        .into()
}
