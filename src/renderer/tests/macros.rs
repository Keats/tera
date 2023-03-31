use crate::context::Context;
use crate::tera::Tera;

use super::NestedObject;

#[test]
fn render_macros() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("macros", "{% macro hello()%}Hello{% endmacro hello %}"),
        (
            "tpl",
            "{% import \"macros\" as macros %}{% block hey %}{{macros::hello()}}{% endblock hey %}",
        ),
    ])
    .unwrap();

    let result = tera.render("tpl", &Context::new());

    assert_eq!(result.unwrap(), "Hello".to_string());
}

#[test]
fn render_macros_defined_in_template() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("tpl", "{% macro hello()%}Hello{% endmacro hello %}{% block hey %}{{self::hello()}}{% endblock hey %}"),
    ])
        .unwrap();

    let result = tera.render("tpl", &Context::new());

    assert_eq!(result.unwrap(), "Hello".to_string());
}

#[test]
fn render_macros_expression_arg() {
    let mut context = Context::new();
    context.insert("pages", &vec![1, 2, 3, 4, 5]);
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("macros", "{% macro hello(val)%}{{val}}{% endmacro hello %}"),
        ("tpl", "{% import \"macros\" as macros %}{{macros::hello(val=pages|last)}}"),
    ])
    .unwrap();

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
    ])
    .unwrap();
    let mut context = Context::new();
    context.insert("my_var", &"&");
    let result = tera.render("hello.html", &context);

    assert_eq!(result.unwrap(), "&".to_string());
}

#[test]
fn render_set_tag_macro() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("macros", "{% macro hello()%}Hello{% endmacro hello %}"),
        (
            "hello.html",
            "{% import \"macros\" as macros %}{% set my_var = macros::hello() %}{{my_var}}",
        ),
    ])
    .unwrap();
    let result = tera.render("hello.html", &Context::new());

    assert_eq!(result.unwrap(), "Hello".to_string());
}

#[test]
fn render_macros_with_default_args() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("macros", "{% macro hello(val=1) %}{{val}}{% endmacro hello %}"),
        ("hello.html", "{% import \"macros\" as macros %}{{macros::hello()}}"),
    ])
    .unwrap();
    let result = tera.render("hello.html", &Context::new());

    assert_eq!(result.unwrap(), "1".to_string());
}

#[test]
fn render_macros_override_default_args() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("macros", "{% macro hello(val=1) %}{{val}}{% endmacro hello %}"),
        ("hello.html", "{% import \"macros\" as macros %}{{macros::hello(val=2)}}"),
    ])
    .unwrap();
    let result = tera.render("hello.html", &Context::new());

    assert_eq!(result.unwrap(), "2".to_string());
}

#[test]
fn render_recursive_macro() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        (
            "macros",
            "{% macro factorial(n) %}{% if n > 1 %}{{ n }} - {{ self::factorial(n=n-1) }}{% else %}1{% endif %}{{ n }}{% endmacro factorial %}",
        ),
        ("hello.html", "{% import \"macros\" as macros %}{{macros::factorial(n=7)}}"),
    ]).unwrap();
    let result = tera.render("hello.html", &Context::new());

    assert_eq!(result.unwrap(), "7 - 6 - 5 - 4 - 3 - 2 - 11234567".to_string());
}

// https://github.com/Keats/tera/issues/202
#[test]
fn recursive_macro_with_loops() {
    let parent = NestedObject { label: "Parent".to_string(), parent: None, numbers: vec![1, 2, 3] };
    let child = NestedObject {
        label: "Child".to_string(),
        parent: Some(Box::new(parent)),
        numbers: vec![1, 2, 3],
    };
    let mut context = Context::new();
    context.insert("objects", &vec![child]);
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
            "#,
        ),
        (
            "recursive",
            r#"
{%- import "macros.html" as macros -%}
{%- for obj in objects -%}
    {{ macros::label_for(obj=obj, sep="|") }}
{%- endfor -%}
"#,
        ),
    ])
    .unwrap();

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
    ])
    .unwrap();
    let result = tera.render("example", &Context::new());

    assert_eq!(result.unwrap(), "my macro".to_string());
}

// https://github.com/Keats/tera/issues/255
#[test]
fn import_macros_into_other_macro_files() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("submacros", "{% macro test() %}Success!{% endmacro %}"),
        (
            "macros",
            r#"{% import "submacros" as sub %}{% macro test() %}{{ sub::test() }}{% endmacro %}"#,
        ),
        ("index", r#"{% import "macros" as macros %}{{ macros::test() }}"#),
    ])
    .unwrap();
    let result = tera.render("index", &Context::new());

    assert_eq!(result.unwrap(), "Success!".to_string());
}

#[test]
fn can_load_parent_macro_in_child() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("macros", "{% macro hello()%}{{ 1 }}{% endmacro hello %}"),
        ("parent", "{% import \"macros\" as macros %}{{ macros::hello() }}{% block bob %}{% endblock bob %}"),
        ("child", "{% extends \"parent\" %}{% block bob %}{{ super() }}Hey{% endblock bob %}"),
    ]).unwrap();

    let result = tera.render("child", &Context::new());

    assert_eq!(result.unwrap(), "1Hey".to_string());
}

#[test]
fn can_load_macro_in_child() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("macros", "{% macro hello()%}{{ 1 }}{% endmacro hello %}"),
        ("parent", "{% block bob %}{% endblock bob %}"),
        ("child", "{% extends \"parent\" %}{% import \"macros\" as macros %}{% block bob %}{{ macros::hello() }}{% endblock bob %}"),
    ]).unwrap();

    let result = tera.render("child", &Context::new());

    assert_eq!(result.unwrap(), "1".to_string());
}

// https://github.com/Keats/tera/issues/333
// this test fails in 0.11.14, worked in 0.11.10
#[test]
fn can_inherit_macro_import_from_parent() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("macros", "{% macro hello()%}HELLO{% endmacro hello %}"),
        ("parent", "{% import \"macros\" as macros %}{% block bob %}parent{% endblock bob %}"),
        ("child", "{% extends \"parent\" %}{% block bob %}{{macros::hello()}}{% endblock bob %}"),
    ])
    .unwrap();

    let result = tera.render("child", &Context::default());
    assert_eq!(result.unwrap(), "HELLO".to_string());
}

#[test]
fn can_inherit_macro_import_from_grandparent() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("macros", "{% macro hello()%}HELLO{% endmacro hello %}"),
        ("grandparent", "{% import \"macros\" as macros %}{% block bob %}grandparent{% endblock bob %}"),
        ("parent", "{% extends \"grandparent\" %}{% import \"macros\" as macros2 %}{% block bob %}parent{% endblock bob %}"),
        ("child", "{% extends \"parent\" %}{% block bob %}{{macros::hello()}}-{{macros2::hello()}}{% endblock bob %}"),
    ]).unwrap();

    let result = tera.render("child", &Context::default());
    assert_eq!(result.unwrap(), "HELLO-HELLO".to_string());
}

#[test]
fn can_load_macro_in_parent_with_grandparent() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("macros", "{% macro hello()%}{{ 1 }}{% endmacro hello %}"),
        ("grandparent", "{% block bob %}{% endblock bob %}"),
        ("parent", "{% extends \"grandparent\" %}{% import \"macros\" as macros %}{% block bob %}{{ macros::hello() }} - Hey{% endblock bob %}"),
        ("child", "{% extends \"parent\" %}{% block bob %}{{ super() }}{% endblock bob %}"),
    ]).unwrap();

    let result = tera.render("child", &Context::new());

    assert_eq!(result.unwrap(), "1 - Hey".to_string());
}

#[test]
fn macro_can_load_macro_from_macro_files() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("submacros", "{% macro emma() %}Emma{% endmacro emma %}"),
        ("macros", "{% import \"submacros\" as submacros %}{% macro hommage() %}{{ submacros::emma() }} was an amazing person!{% endmacro hommage %}"),
        ("parent", "{% block main %}Someone was a terrible person!{% endblock main %} Don't you think?"),
        ("child", "{% extends \"parent\" %}{% import \"macros\" as macros %}{% block main %}{{ macros::hommage() }}{% endblock main %}")
    ]).unwrap();

    let result = tera.render("child", &Context::new());
    //println!("{:#?}", result);
    assert_eq!(result.unwrap(), "Emma was an amazing person! Don't you think?".to_string());
}

#[test]
fn macro_can_access_global_context() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("parent", r#"{% import "macros" as macros %}{{ macros::test_global() }}"#),
        ("macros", r#"{% macro test_global() %}{% set_global value1 = "42" %}{% for i in range(end=1) %}{% set_global value2 = " is the truth." %}{% endfor %}{{ value1 }}{% endmacro test_global %}"#)
    ]).unwrap();

    let result = tera.render("parent", &Context::new());
    assert_eq!(result.unwrap(), "42".to_string());
}

#[test]
fn template_cant_access_macros_context() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("parent", r#"{% import "macros" as macros %}{{ macros::empty() }}{{ quote | default(value="I'd rather have roses on my table than diamonds on my neck.") }}"#),
        ("macros", r#"{% macro empty() %}{% set_global quote = "This should not reachable from the calling template!" %}{% endmacro empty %}"#)
    ]).unwrap();

    let result = tera.render("parent", &Context::new());
    assert_eq!(result.unwrap(), "I'd rather have roses on my table than diamonds on my neck.");
}

#[test]
fn parent_macro_cant_access_child_macro_context() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("parent", "{% import \"macros\" as macros %}{{ macros::test_global() }}"),
        ("macros", r#"{% import "moremacros" as moremacros %}{% macro test_global() %}{% set_global value1 = "ACAB" %}{{ moremacros::another_one() }}{{ value1 }}-{{ value2 | default(value="ACAB") }}{% endmacro test_global %}"#),
        ("moremacros", r#"{% macro another_one() %}{% set_global value2 = "1312" %}{% endmacro another_one %}"#)
    ]).unwrap();

    let result = tera.render("parent", &Context::new());
    assert_eq!(result.unwrap(), "ACAB-ACAB".to_string());
}
