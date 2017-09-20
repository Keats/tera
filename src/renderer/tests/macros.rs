use context::Context;
use errors::Result;
use tera::Tera;

#[test]
fn render_macros() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("macros", "{% macro hello()%}Hello{% endmacro hello %}"),
        ("tpl", "{% import \"macros\" as macros %}{% block hey %}{{macros::hello()}}{% endblock hey %}"),
    ]).unwrap();

    let result = tera.render("tpl", &Context::new());

    assert_eq!(result.unwrap(), "Hello".to_string());
}

#[test]
fn render_macros_expression_arg() {
    let mut context = Context::new();
    context.add("pages", &vec![1, 2, 3, 4, 5]);
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("macros", "{% macro hello(val)%}{{val}}{% endmacro hello %}"),
        ("tpl", "{% import \"macros\" as macros %}{{macros::hello(val=pages|last)}}"),
    ]).unwrap();

    let result = tera.render("tpl", &context);

    assert_eq!(result.unwrap(), "5".to_string());
}

#[test]
fn render_macros_in_child_templates_same_namespace() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("grandparent", "{% block hey %}hello{% endblock hey %}"),
        ("macros", "{% macro hello()%}Hello{% endmacro hello %}"),
        ("macros2", "{% macro hi()%}Hi{% endmacro hi %}"),
        ("parent", "{% extends \"grandparent\" %}{% import \"macros\" as macros %}{% block hey %}{{macros::hello()}}{% endblock hey %}"),
        ("child", "{% extends \"parent\" %}{% import \"macros2\" as macros %}{% block hey %}{{super()}}/{{macros::hi()}}{% endblock hey %}"),
    ]).unwrap();

    let result = tera.render("child", &Context::new());

    assert_eq!(result.unwrap(), "Hello/Hi".to_string());
}

#[test]
fn render_macros_in_child_templates_different_namespace() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("grandparent", "{% block hey %}hello{% endblock hey %}"),
        ("macros", "{% macro hello()%}Hello{% endmacro hello %}"),
        ("macros2", "{% macro hi()%}Hi{% endmacro hi %}"),
        ("parent", "{% extends \"grandparent\" %}{% import \"macros\" as macros %}{% block hey %}{{macros::hello()}}{% endblock hey %}"),
        ("child", "{% extends \"parent\" %}{% import \"macros2\" as macros2 %}{% block hey %}{{super()}}/{{macros2::hi()}}{% endblock hey %}"),
    ]).unwrap();

    let result = tera.render("child", &Context::new());

    assert_eq!(result.unwrap(), "Hello/Hi".to_string());
}

#[test]
fn render_macros_in_parent_template_with_inheritance() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("macros", "{% macro hello()%}Hello{% endmacro hello %}"),
        ("grandparent", "{% import \"macros\" as macros %}{% block hey %}{{macros::hello()}}{% endblock hey %}"),
        ("child", "{% extends \"grandparent\" %}{% import \"macros\" as macros %}{% block hey %}{{super()}}/{{macros::hello()}}{% endblock hey %}"),
    ]).unwrap();

    let result = tera.render("child", &Context::new());

    assert_eq!(result.unwrap(), "Hello/Hello".to_string());
}

#[test]
fn macro_param_arent_escaped() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("macros.html", r#"{% macro print(val) %}{{val|safe}}{% endmacro print %}"#),
        ("hello.html", r#"{% import "macros.html" as macros %}{{ macros::print(val=my_var)}}"#),
    ]).unwrap();
    let mut context = Context::new();
    context.add("my_var", &"&");
    let result = tera.render("hello.html", &context);

    assert_eq!(result.unwrap(), "&".to_string());
}

#[test]
fn render_set_tag_macro() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("macros", "{% macro hello()%}Hello{% endmacro hello %}"),
        ("hello.html", "{% import \"macros\" as macros %}{% set my_var = macros::hello() %}{{my_var}}"),
    ]).unwrap();
    let result = tera.render("hello.html", &Context::new());

    assert_eq!(result.unwrap(), "Hello".to_string());
}

#[test]
fn render_macros_with_default_args() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("macros", "{% macro hello(val=1) %}{{val}}{% endmacro hello %}"),
        ("hello.html", "{% import \"macros\" as macros %}{{macros::hello()}}"),
    ]).unwrap();
    let result = tera.render("hello.html", &Context::new());

    assert_eq!(result.unwrap(), "1".to_string());
}

#[test]
fn render_macros_override_default_args() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("macros", "{% macro hello(val=1) %}{{val}}{% endmacro hello %}"),
        ("hello.html", "{% import \"macros\" as macros %}{{macros::hello(val=2)}}"),
    ]).unwrap();
    let result = tera.render("hello.html", &Context::new());

    assert_eq!(result.unwrap(), "2".to_string());
}
