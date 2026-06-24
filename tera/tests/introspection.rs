use std::collections::{HashMap, HashSet};

use tera::{ComponentArgType, Tera, Value};

#[test]
fn test_get_component_definition() {
    let mut tera = Tera::default();
    tera.add_raw_template(
        "components.html",
        r#"{% component Button(label: String, size: Integer = 1, variant = "primary", message, ...restant) {"doc": "An alert box", "deprecated": true} %} %}<button>{{ label }}</button>{% endcomponent Button %}"#,
    )
        .unwrap();

    let info = tera.get_component_definition("Button").unwrap();
    assert_eq!(info.name(), "Button");
    assert_eq!(info.args().len(), 4);
    assert_eq!(info.rest_param(), Some("restant"));

    let args: HashMap<_, _> = info.args().iter().map(|x| (x.name(), x)).collect();

    let label = args.get("label").unwrap();
    assert_eq!(label.name(), "label");
    assert!(label.is_required());
    assert_eq!(label.arg_type().unwrap(), ComponentArgType::String);
    assert!(label.default().is_none());

    let size = args.get("size").unwrap();
    assert!(!size.is_required());
    assert_eq!(size.arg_type().unwrap(), ComponentArgType::Integer);
    assert_eq!(size.default().unwrap(), &Value::from(1));

    let variant = args.get("variant").unwrap();
    assert!(!variant.is_required());
    // Type is inferred from the default value
    assert_eq!(variant.arg_type().unwrap(), ComponentArgType::String);
    assert_eq!(variant.default().unwrap(), &Value::from("primary"));

    let meta = info.metadata();
    assert_eq!(meta.get("doc").unwrap(), &Value::from("An alert box"));
    assert_eq!(meta.get("deprecated").unwrap(), &Value::from(true));

    assert!(tera.get_component_definition("DoesNotExist").is_none());
}

#[test]
fn test_get_template_variables() {
    let mut tera = Tera::new();
    tera.add_raw_templates(vec![
        (
            "page.html",
            r#"{{ name }} {{ user.name }}
{% set x = 1 %}{{ x }}
{% set a = some_value %}
{{ y }}{% set y = 2 %}
{% if cond %}{% set maybe = 1 %}{% endif %}{{ maybe }}
{% if cond2 %}{% set z = 1 %}{% else %}{% set z = 2 %}{% endif %}{{ z }}
{{ item }}{% for item in items %}{% set_global g = 1 %}{{ item }} {{ loop.index }}{% endfor %}
{{ g }}"#,
        ),
        (
            "parent.html",
            r#"{% set nav = "main" %}{{ title }} {{ nav }}{% block content %}{% endblock %}"#,
        ),
        (
            "child.html",
            r#"{% extends "parent.html" %}{% block content %}{{ body }}{% include "inc.html" %}{% endblock %}"#,
        ),
        (
            "inc.html",
            r#"{{ inc_var }}{% include "inc_nested.html" %}"#,
        ),
        ("inc_nested.html", r#"{{ inc_nested_var }}"#),
    ])
        .unwrap();

    let expected_cases: Vec<(&str, HashSet<&str>)> = vec![
        (
            "page.html",
            HashSet::from([
                "name",
                "user",
                "some_value",
                "y",
                "cond",
                "cond2",
                "item",
                "items",
            ]),
        ),
        (
            "child.html",
            HashSet::from(["title", "body", "inc_var", "inc_nested_var"]),
        ),
    ];

    for (template_name, expected) in expected_cases {
        assert_eq!(
            tera.get_template_variables(template_name).unwrap(),
            expected,
            "unexpected variables for template `{template_name}`",
        );
    }
}
