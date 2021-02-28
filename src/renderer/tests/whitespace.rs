use crate::context::Context;
use crate::tera::Tera;

#[test]
fn can_remove_whitespace_basic() {
    let mut context = Context::new();
    context.insert("numbers", &vec![1, 2, 3]);

    let inputs = vec![
        ("  {%- for n in numbers %}{{n}}{% endfor -%} ", "123"),
        ("{%- for n in numbers %} {{n}}{%- endfor -%} ", " 1 2 3"),
        ("{%- for n in numbers -%}\n {{n}}\n {%- endfor -%} ", "123"),
        ("{%- if true -%}\n {{numbers}}\n {%- endif -%} ", "[1, 2, 3]"),
        ("{%- if false -%}\n {{numbers}}\n {% else %} Nope{%- endif -%} ", " Nope"),
        ("  {%- if false -%}\n {{numbers}}\n {% else -%} Nope {%- endif -%} ", "Nope"),
        ("  {%- if false -%}\n {{numbers}}\n {% elif true -%} Nope {%- endif -%} ", "Nope"),
        ("  {%- if false -%}\n {{numbers}}\n {% elif false -%} Nope {% else %} else {%- endif -%} ", " else"),
        ("  {%- set var = 2 -%} {{var}}", "2"),
        ("  {% set var = 2 -%} {{var}}", "  2"),
        (" {% raw -%} {{2}} {% endraw -%} ", " {{2}} "),
        ("  {% filter upper -%} hey {%- endfilter -%} ", "  HEY"),
        ("  {{ \"hello\" -}} ", "  hello"),
        ("  {{- \"hello\" }} ", "hello "),
        ("  {{- \"hello\" -}} ", "hello"),
        // Comments are not rendered so it should be just whitespace if anything
        ("  {#- \"hello\" -#} ", ""),
        ("  {# \"hello\" -#} ", "  "),
        ("  {#- \"hello\" #} ", " "),
    ];

    for (input, expected) in inputs {
        let mut tera = Tera::default();
        tera.add_raw_template("tpl", input).unwrap();
        println!("{} -> {:?}", input, expected);
        assert_eq!(tera.render("tpl", &context).unwrap(), expected);
    }
}

#[test]
fn can_remove_whitespace_include() {
    let mut context = Context::new();
    context.insert("numbers", &vec![1, 2, 3]);

    let inputs = vec![
        (r#"Hi {%- include "include" -%} "#, "HiIncluded"),
        (r#"Hi {% include "include" -%} "#, "Hi Included"),
        (r#"Hi {% include "include" %} "#, "Hi Included "),
    ];

    for (input, expected) in inputs {
        let mut tera = Tera::default();
        tera.add_raw_templates(vec![("include", "Included"), ("tpl", input)]).unwrap();
        assert_eq!(tera.render("tpl", &context).unwrap(), expected);
    }
}

#[test]
fn can_remove_whitespace_macros() {
    let mut context = Context::new();
    context.insert("numbers", &vec![1, 2, 3]);

    let inputs = vec![
        (r#" {%- import "macros" as macros -%} {{macros::hey()}}"#, "Hey!"),
        (r#" {% import "macros" as macros %} {{macros::hey()}}"#, "Hey!"),
        (r#" {%- import "macros" as macros %} {%- set hey = macros::hey() -%} {{hey}}"#, "Hey!"),
    ];

    for (input, expected) in inputs {
        let mut tera = Tera::default();
        tera.add_raw_templates(vec![
            ("macros", "{% macro hey() -%} Hey! {%- endmacro %}"),
            ("tpl", input),
        ])
        .unwrap();
        assert_eq!(tera.render("tpl", &context).unwrap(), expected);
    }
}

#[test]
fn can_remove_whitespace_inheritance() {
    let mut context = Context::new();
    context.insert("numbers", &vec![1, 2, 3]);

    let inputs = vec![
        (r#"{%- extends "base" -%} {% block content %}{{super()}}{% endblock %}"#, " Hey! "),
        (r#"{%- extends "base" -%} {% block content -%}{{super()}}{%- endblock %}"#, " Hey! "),
        (r#"{%- extends "base" %} {%- block content -%}{{super()}}{%- endblock -%} "#, " Hey! "),
    ];

    for (input, expected) in inputs {
        let mut tera = Tera::default();
        tera.add_raw_templates(vec![
            ("base", "{% block content %} Hey! {% endblock %}"),
            ("tpl", input),
        ])
        .unwrap();
        assert_eq!(tera.render("tpl", &context).unwrap(), expected);
    }
}

// https://github.com/Keats/tera/issues/475
#[test]
fn works_with_filter_section() {
    let mut context = Context::new();
    context.insert("d", "d");
    let input = r#"{% filter upper %}  {{ "c" }}   d{% endfilter %}"#;
    let res = Tera::one_off(input, &context, true).unwrap();
    assert_eq!(res, "  C   D");
}

#[test]
fn make_sure_not_to_delete_whitespaces() {
    let mut context = Context::new();
    context.insert("d", "d");
    let input = r#"{% raw %}    yaml_test:     {% endraw %}"#;
    let res = Tera::one_off(input, &context, true).unwrap();
    assert_eq!(res, "    yaml_test:     ");
}
