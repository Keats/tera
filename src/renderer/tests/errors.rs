use std::collections::HashMap;
use std::error::Error;

use crate::context::Context;
use crate::tera::Tera;

#[test]
fn error_location_basic() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![("tpl", "{{ 1 + true }}")]).unwrap();

    let result = tera.render("tpl", &Context::new());

    assert_eq!(result.unwrap_err().to_string(), "Failed to render \'tpl\'");
}

#[test]
fn error_location_inside_macro() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("macros", "{% macro hello()%}{{ 1 + true }}{% endmacro hello %}"),
        ("tpl", "{% import \"macros\" as macros %}{{ macros::hello() }}"),
    ])
    .unwrap();

    let result = tera.render("tpl", &Context::new());

    assert_eq!(
        result.unwrap_err().to_string(),
        "Failed to render \'tpl\': error while rendering macro `macros::hello`"
    );
}

#[test]
fn error_loading_macro_from_unloaded_namespace() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("macros", "{% macro hello()%}{{ 1 + true }}{% endmacro hello %}"),
        ("tpl", "{% import \"macros\" as macros %}{{ macro::hello() }}"),
    ])
    .unwrap();

    let result = tera.render("tpl", &Context::new());
    println!("{:#?}", result);
    assert_eq!(
        result.unwrap_err().source().unwrap().to_string(),
        "Macro namespace `macro` was not found in template `tpl`. Have you maybe forgotten to import it, or misspelled it?"
    );
}

#[test]
fn error_location_base_template() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("parent", "Hello {{ greeting + 1}} {% block bob %}{% endblock bob %}"),
        ("child", "{% extends \"parent\" %}{% block bob %}Hey{% endblock bob %}"),
    ])
    .unwrap();

    let result = tera.render("child", &Context::new());

    assert_eq!(
        result.unwrap_err().to_string(),
        "Failed to render \'child\' (error happened in 'parent')."
    );
}

#[test]
fn error_location_in_parent_block() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("parent", "Hello {{ greeting }} {% block bob %}{{ 1 + true }}{% endblock bob %}"),
        ("child", "{% extends \"parent\" %}{% block bob %}{{ super() }}Hey{% endblock bob %}"),
    ])
    .unwrap();

    let result = tera.render("child", &Context::new());

    assert_eq!(
        result.unwrap_err().to_string(),
        "Failed to render \'child\' (error happened in 'parent')."
    );
}

#[test]
fn error_location_in_parent_in_macro() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("macros", "{% macro hello()%}{{ 1 + true }}{% endmacro hello %}"),
        ("parent", "{% import \"macros\" as macros %}{{ macros::hello() }}{% block bob %}{% endblock bob %}"),
        ("child", "{% extends \"parent\" %}{% block bob %}{{ super() }}Hey{% endblock bob %}"),
    ]).unwrap();

    let result = tera.render("child", &Context::new());

    assert_eq!(
        result.unwrap_err().to_string(),
        "Failed to render \'child\': error while rendering macro `macros::hello` (error happened in \'parent\')."
    );
}

#[test]
fn error_out_of_range_index() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![("tpl", "{{ arr[10] }}")]).unwrap();
    let mut context = Context::new();
    context.insert("arr", &[1, 2, 3]);

    let result = tera.render("tpl", &Context::new());

    assert_eq!(
        result.unwrap_err().source().unwrap().to_string(),
        "Variable `arr[10]` not found in context while rendering \'tpl\': the evaluated version was `arr.10`. Maybe the index is out of bounds?"
    );
}

#[test]
fn error_unknown_index_variable() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![("tpl", "{{ arr[a] }}")]).unwrap();
    let mut context = Context::new();
    context.insert("arr", &[1, 2, 3]);

    let result = tera.render("tpl", &context);

    assert_eq!(
        result.unwrap_err().source().unwrap().to_string(),
        "Variable arr[a] can not be evaluated because: Variable `a` not found in context while rendering \'tpl\'"
    );
}

#[test]
fn error_invalid_type_index_variable() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![("tpl", "{{ arr[a] }}")]).unwrap();

    let mut context = Context::new();
    context.insert("arr", &[1, 2, 3]);
    context.insert("a", &true);

    let result = tera.render("tpl", &context);

    assert_eq!(
        result.unwrap_err().source().unwrap().to_string(),
        "Only variables evaluating to String or Number can be used as index (`a` of `arr[a]`)"
    );
}

#[test]
fn error_when_missing_macro_templates() {
    let mut tera = Tera::default();
    let result = tera.add_raw_templates(vec![(
        "parent",
        "{% import \"macros\" as macros %}{{ macros::hello() }}{% block bob %}{% endblock bob %}",
    )]);
    assert_eq!(
        result.unwrap_err().to_string(),
        "Template `parent` loads macros from `macros` which isn\'t present in Tera"
    );
}

#[test]
fn error_when_using_variable_set_in_included_templates_outside() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("included", r#"{{a}}{% set b = "hi" %}-{{b}}"#),
        ("base", r#"{{a}}{% include "included" %}{{b}}"#),
    ])
    .unwrap();
    let mut context = Context::new();
    context.insert("a", &10);
    let result = tera.render("base", &context);

    assert_eq!(
        result.unwrap_err().source().unwrap().to_string(),
        "Variable `b` not found in context while rendering \'base\'"
    );
}

// https://github.com/Keats/tera/issues/344
// Yes it is as silly as it sounds
#[test]
fn right_variable_name_is_needed_in_for_loop() {
    let mut data = HashMap::new();
    data.insert("content", "hello");
    let mut context = Context::new();
    context.insert("comments", &vec![data]);
    let mut tera = Tera::default();
    tera.add_raw_template(
        "tpl",
        r#"
{%- for comment in comments -%}
<p>{{ comment.content }}</p>
<p>{{ whocares.content }}</p>
<p>{{ doesntmatter.content }}</p>
{% endfor -%}"#,
    )
    .unwrap();
    let result = tera.render("tpl", &context);

    assert_eq!(
        result.unwrap_err().source().unwrap().to_string(),
        "Variable `whocares.content` not found in context while rendering \'tpl\'"
    );
}

// https://github.com/Keats/tera/issues/385
// https://github.com/Keats/tera/issues/370
#[test]
fn errors_with_inheritance_in_included_template() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("base", "Base - {% include \"child\" %}"),
        ("parent", "{% block title %}Parent{% endblock %}"),
        ("child", "{% extends \"parent\" %}{% block title %}{{ super() }} - Child{% endblock %}"),
    ])
    .unwrap();

    let result = tera.render("base", &Context::new());

    assert_eq!(
        result.unwrap_err().source().unwrap().to_string(),
        "Inheritance in included templates is currently not supported: extended `parent`"
    );
}

#[test]
fn error_string_concat_math_logic() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![("tpl", "{{ 'ho' ~ name < 10 }}")]).unwrap();
    let mut context = Context::new();
    context.insert("name", &"john");

    let result = tera.render("tpl", &context);

    assert_eq!(
        result.unwrap_err().source().unwrap().to_string(),
        "Tried to do math with a string concatenation: 'ho' ~ name"
    );
}

#[test]
fn error_gives_source_on_tests() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![("tpl", "{% if a is undefined(1) %}-{% endif %}")]).unwrap();
    let result = tera.render("tpl", &Context::new());
    println!("{:?}", result);
    let err = result.unwrap_err();

    let source = err.source().unwrap();
    assert_eq!(source.to_string(), "Test call \'undefined\' failed");
    let source2 = source.source().unwrap();

    assert_eq!(
        source2.to_string(),
        "Tester `undefined` was called with some args but this test doesn\'t take args"
    );
}
