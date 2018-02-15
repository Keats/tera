use context::Context;
use errors::Result;
use tera::Tera;

use super::NestedObject;

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

#[test]
fn render_recursive_macro() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("macros", "{% macro factorial(n) %}{% if n > 1 %}{{ n }} - {{ self::factorial(n=n-1) }}{% else %}1{% endif %}{{ n }}{% endmacro factorial %}"),
        ("hello.html", "{% import \"macros\" as macros %}{{macros::factorial(n=7)}}"),
    ]).unwrap();
    let result = tera.render("hello.html", &Context::new());

    assert_eq!(result.unwrap(), "7 - 6 - 5 - 4 - 3 - 2 - 11234567".to_string());
}

// https://github.com/Keats/tera/issues/202
#[test]
fn recursive_macro_with_loops() {
    let parent = NestedObject { label: "Parent".to_string(), parent: None, numbers: vec![1, 2, 3] };
    let child = NestedObject { label: "Child".to_string(), parent: Some(Box::new(parent)), numbers: vec![1, 2, 3] };
    let mut context = Context::new();
    context.add("objects", &vec![child]);
    let mut tera = Tera::default();

    tera.add_raw_templates(vec![
        (
            "macros.html",
            r#"
{% macro label_for(obj, sep) -%}
  {%- if obj.parent -%}
    {{ self::label_for(obj=obj.parent, sep=sep) }}{{sep}}
  {%- endif -%}
  {{obj.label}}
  {%- for i in obj.numbers -%}{{ i }}{%- endfor -%}
{%- endmacro label_for %}
            "#
        ),
        (
            "recursive",
            r#"
{%- import "macros.html" as macros -%}
{%- for obj in objects -%}
    {{ macros::label_for(obj=obj, sep="|") }}
{%- endfor -%}
"#
        ),
    ]).unwrap();

    let result = tera.render("recursive", &context);

    assert_eq!(result.unwrap(), "Parent123|Child123".to_string());
}

// https://github.com/Keats/tera/issues/250
#[test]
fn render_macros_in_included() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("macros", "{% macro my_macro() %}my macro{% endmacro %}"),
        ("includeme", r#"{% import "macros" as macros %}{{ macros::my_macro() }}"#),
        ("example", r#"{% include "includeme" %}"#),
    ]).unwrap();
    let result = tera.render("example", &Context::new());

    assert_eq!(result.unwrap(), "my macro".to_string());
}

// https://github.com/Keats/tera/issues/255
#[test]
fn import_macros_into_other_macro_files() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("submacros", "{% macro test() %}Success!{% endmacro %}"),
        ("macros", r#"{% import "submacros" as sub %}{% macro test() %}{{ sub::test() }}{% endmacro %}"#),
        ("index", r#"{% import "macros" as macros %}{{ macros::test() }}"#),
    ]).unwrap();
    let result = tera.render("index", &Context::new());

    assert_eq!(result.unwrap(), "Success!".to_string());

}
