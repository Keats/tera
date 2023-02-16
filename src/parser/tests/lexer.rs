use pest::Parser;

use crate::parser::{Rule, TeraParser};

macro_rules! assert_lex_rule {
    ($rule: expr, $input: expr) => {
        let res = TeraParser::parse($rule, $input);
        println!("{:?}", $input);
        println!("{:#?}", res);
        if res.is_err() {
            println!("{}", res.unwrap_err());
            panic!();
        }
        assert!(res.is_ok());
        assert_eq!(res.unwrap().last().unwrap().as_span().end(), $input.len());
    };
}

#[test]
fn lex_boolean() {
    let inputs = vec!["true", "false", "True", "False"];
    for i in inputs {
        assert_lex_rule!(Rule::boolean, i);
    }
}

#[test]
fn lex_int() {
    let inputs = vec!["-10", "0", "100", "250000"];
    for i in inputs {
        assert_lex_rule!(Rule::int, i);
    }
}

#[test]
fn lex_float() {
    let inputs = vec!["123.5", "123.5", "0.1", "-1.1"];
    for i in inputs {
        assert_lex_rule!(Rule::float, i);
    }
}

#[test]
fn lex_string() {
    let inputs = vec![
        "\"Blabla\"",
        "\"123\"",
        "\'123\'",
        "\'This is still a string\'",
        "`this is backquted`",
        "`and this too`",
    ];
    for i in inputs {
        assert_lex_rule!(Rule::string, i);
    }
}

#[test]
fn lex_ident() {
    let inputs = vec!["hello", "hello_", "hello_1", "HELLO", "_1"];
    for i in inputs {
        assert_lex_rule!(Rule::ident, i);
    }

    assert!(TeraParser::parse(Rule::ident, "909").is_err());
}

#[test]
fn lex_dotted_ident() {
    let inputs = vec![
        "hello",
        "hello_",
        "hello_1",
        "HELLO",
        "_1",
        "hey.ho",
        "h",
        "ho",
        "hey.ho.hu",
        "hey.0",
        "h.u",
    ];
    for i in inputs {
        assert_lex_rule!(Rule::dotted_ident, i);
    }

    let invalid_inputs = vec![".", "9.w"];
    for i in invalid_inputs {
        assert!(TeraParser::parse(Rule::dotted_ident, i).is_err());
    }
}

#[test]
fn lex_dotted_square_bracket_ident() {
    let inputs = vec![
        "hey.ho.hu",
        "hey.0",
        "h.u.x.0",
        "hey['ho'][\"hu\"]",
        "hey[0]",
        "h['u'].x[0]",
        "hey[a[0]]",
    ];
    for i in inputs {
        assert_lex_rule!(Rule::dotted_square_bracket_ident, i);
    }

    let invalid_inputs = vec![".", "9.w"];
    for i in invalid_inputs {
        assert!(TeraParser::parse(Rule::dotted_square_bracket_ident, i).is_err());
    }
}

#[test]
fn lex_string_concat() {
    let inputs = vec![
        "'hello' ~ `hey`",
        "'hello' ~ 1",
        "'hello' ~ 3.18",
        "1 ~ 'hello'",
        "3.18 ~ 'hello'",
        "'hello' ~ ident",
        "ident ~ 'hello'",
        "'hello' ~ ident[0]",
        "'hello' ~ a_function()",
        "a_function() ~ 'hello'",
        r#"'hello' ~ "hey""#,
        r#"a_string ~ " world""#,
        "'hello' ~ ident ~ `ho`",
    ];

    for i in inputs {
        assert_lex_rule!(Rule::string_concat, i);
    }
}

#[test]
fn lex_array() {
    let inputs = vec![
        "[]",
        "[1,2,3]",
        "[1, 2,3,]",
        "[1 + 1, 2,3 * 2,]",
        "[\"foo\", \"bar\"]",
        "[1,true,'string', 0.5, hello(), macros::hey(arg=1)]",
    ];

    for i in inputs {
        assert_lex_rule!(Rule::array, i);
    }
}

#[test]
fn lex_basic_expr() {
    let inputs = vec![
        "admin",
        "true",
        "macros::something()",
        "something()",
        "a is defined",
        "a is defined(2)",
        "1 + 1",
        "1 + counts",
        "1 + counts.first",
        "1 + 2 + 3 * 9/2 + 2.1",
        "(1 + 2 + 3) * 9/2 + 2.1",
        "10 * 2 % 5",
    ];

    for i in inputs {
        assert_lex_rule!(Rule::basic_expr, i);
    }
}

#[test]
fn lex_basic_expr_with_filter() {
    let inputs = vec![
        "admin | hello",
        "true | ho",
        "macros::something() | hey",
        "something() | hey",
        "a is defined | ho",
        "a is defined(2) | ho",
        "1 + 1 | round",
        "1 + counts | round",
        "1 + counts.first | round",
        "1 + 2 + 3 * 9/2 + 2.1 | round",
        "(1 + 2 + 3) * 9/2 + 2.1 | round",
        "10 * 2 % 5 | round",
    ];

    for i in inputs {
        assert_lex_rule!(Rule::basic_expr_filter, i);
    }
}

#[test]
fn lex_string_expr_with_filter() {
    let inputs = vec![
        r#""hey" | capitalize"#,
        r#""hey""#,
        r#""hey" ~ 'ho' | capitalize"#,
        r#""hey" ~ ho | capitalize"#,
        r#"ho ~ ho ~ ho | capitalize"#,
        r#"ho ~ 'ho' ~ ho | capitalize"#,
        r#"ho ~ 'ho' ~ ho"#,
    ];

    for i in inputs {
        assert_lex_rule!(Rule::string_expr_filter, i);
    }
}

#[test]
fn lex_comparison_val() {
    let inputs = vec![
        // all the basic expr still work
        "admin",
        "true",
        "macros::something()",
        "something()",
        "a is defined",
        "a is defined(2)",
        "1 + 1",
        "1 + counts",
        "1 + counts.first",
        "1 + 2 + 3 * 9/2 + 2.1",
        "(1 + 2 + 3) * 9/2 + 2.1",
        "10 * 2 % 5",
        // but now ones with filters also work
        "admin | upper",
        "admin | upper | round",
        "admin | upper | round(var=2)",
        "1.5 + a | round(var=2)",
        // and maths after filters is ok
        "a | length - 1",
        "1.5 + a | round - 1",
        "1.5 + a | round - (1 + 1.5) | round",
        "1.5 + a | round - (1 + 1.5) | round",
    ];

    for i in inputs {
        assert_lex_rule!(Rule::comparison_val, i);
    }
}

#[test]
fn lex_in_cond() {
    let inputs = vec![
        "a in b",
        "1 in b",
        "'b' in b",
        "'b' in b",
        "a in request.path",
        "'index.html' in request.build_absolute_uri",
        "a in [1, 2, 3]",
        "a | capitalize in [1, 2, 3]",
        "a | capitalize in [1, 'hey']",
        "a | capitalize in [ho, 1, 'hey']",
        "'e' in 'hello'",
        "'e' in 'hello' | capitalize",
        "e in 'hello'",
    ];

    for i in inputs {
        assert_lex_rule!(Rule::in_cond, i);
    }
}

#[test]
fn lex_comparison_expr() {
    let inputs = vec![
        "1.5 + a | round(var=2) > 10",
        "1.5 + a | round(var=2) > a | round",
        "a == b",
        "a + 1 == b",
        "a != b",
        "a % 2 == 0",
        "a == 'admin'",
        "a != 'admin'",
        "a == 'admin' | capitalize",
        "a != 'admin' | capitalize",
        "a > b",
        "a >= b",
        "a < b",
        "a <= b",
        "true > false",
    ];

    for i in inputs {
        assert_lex_rule!(Rule::comparison_expr, i);
    }
}

#[test]
fn lex_logic_val() {
    let inputs = vec![
        // all the basic expr still work
        "admin",
        "true",
        "macros::something()",
        "something()",
        r#""hey""#,
        "a is defined",
        "a is defined(2)",
        "a is not defined",
        "1 + 1",
        "1 + counts",
        "1 + counts.first",
        "1 + 2 + 3 * 9/2 + 2.1",
        "(1 + 2 + 3) * 9/2 + 2.1",
        "10 * 2 % 5",
        // filters still work
        "admin | upper",
        "admin | upper | round",
        "admin | upper | round(var=2)",
        "1.5 + a | round(var=2)",
        // but now we can negate things
        "not true",
        "not admin",
        "not num + 1 == 0",
    ];

    for i in inputs {
        assert_lex_rule!(Rule::logic_val, i);
    }
}

#[test]
fn lex_logic_expr() {
    let inputs = vec![
        "1.5 + a | round(var=2) > 10 and admin",
        "1.5 + a | round(var=2) > a | round or true",
        "1 > 0 and 2 < 3",
    ];

    for i in inputs {
        assert_lex_rule!(Rule::logic_expr, i);
    }
}

#[test]
fn lex_kwarg() {
    let inputs = vec![
        "hello=1",
        "hello=1+1",
        "hello=[]",
        "hello=[true, false]",
        "hello1=true",
        "hello=name",
        "hello=name|filter",
        "hello=name|filter(with_arg=true)",
    ];
    for i in inputs {
        assert_lex_rule!(Rule::kwarg, i);
    }
}

#[test]
fn lex_kwargs() {
    let inputs = vec![
        "hello=1",
        "hello=1+1,hey=1",
        "hello1=true,name=name,admin=true",
        "hello=name",
        "hello=name|filter,id=1",
        "hello=name|filter(with_arg=true),id=1",
    ];
    for i in inputs {
        assert_lex_rule!(Rule::kwargs, i);
    }
}

#[test]
fn lex_fn_call() {
    let inputs = vec![
        "fn(hello=1)",
        "fn(hello=1+1,hey=1)",
        "fn(hello1=true,name=name,admin=true)",
        "fn(hello=name)",
        "fn(hello=name,)",
        "fn(\n  hello=name,\n)",
        "fn(hello=name|filter,id=1)",
    ];
    for i in inputs {
        assert_lex_rule!(Rule::fn_call, i);
    }
}

#[test]
fn lex_filter() {
    let inputs = vec![
        "|attr",
        "|attr()",
        "|attr(key=1)",
        "|attr(key=1, more=true)",
        "|attr(key=1,more=true)",
    ];
    for i in inputs {
        assert_lex_rule!(Rule::filter, i);
    }
}

#[test]
fn lex_macro_definition() {
    let inputs = vec![
        "hello()",
        "hello(name, admin)",
        "hello(name, admin=1)",
        "hello(name=\"bob\", admin)",
        "hello(name=\"bob\",admin=true)",
    ];
    for i in inputs {
        // The () are not counted as tokens for some reasons so can't use the macro
        assert!(TeraParser::parse(Rule::macro_fn, i).is_ok());
    }
}

#[test]
fn lex_test() {
    let inputs =
        vec!["a is defined", "a is defined()", "a is divisibleby(2)", "a is in([1, 2, something])"];
    for i in inputs {
        // The () are not counted as tokens for some reasons so can't use the macro
        assert!(TeraParser::parse(Rule::test, i).is_ok());
    }
}

#[test]
fn lex_include_tag() {
    assert!(TeraParser::parse(Rule::include_tag, "{% include \"index.html\" %}").is_ok());
    assert!(TeraParser::parse(Rule::include_tag, "{% include [\"index.html\"] %}").is_ok());
    assert!(TeraParser::parse(Rule::include_tag, "{% include [\"index.html\"] ignore missing %}")
        .is_ok());
}

#[test]
fn lex_import_macro_tag() {
    assert!(TeraParser::parse(Rule::import_macro_tag, "{% import \"macros.html\" as macros %}",)
        .is_ok());
}

#[test]
fn lex_extends_tag() {
    assert!(TeraParser::parse(Rule::extends_tag, "{% extends \"index.html\" %}").is_ok());
}

#[test]
fn lex_comment_tag() {
    let inputs = vec![
        "{# #comment# {{}} {%%} #}",
        "{# #comment# {{}} {%%} #}",
        "{#- #comment# {{}} {%%} #}",
        "{# #comment# {{}} {%%} -#}",
        "{#- #comment# {{}} {%%} -#}",
    ];
    for i in inputs {
        assert_lex_rule!(Rule::comment_tag, i);
    }
}

#[test]
fn lex_block_tag() {
    let inputs = vec!["{% block tag %}", "{% block my_block %}"];
    for i in inputs {
        assert_lex_rule!(Rule::block_tag, i);
    }
}

#[test]
fn lex_filter_tag() {
    let inputs = vec![
        "{%- filter tag() %}",
        "{% filter foo(bar=baz) -%}",
        "{% filter foo(bar=42) %}",
        "{% filter foo(bar=baz,qux=quz) %}",
        "{% filter foo(bar=baz, qux=quz) %}",
        "{% filter foo ( bar=\"baz\", qux=42 ) %}",
    ];
    for i in inputs {
        assert_lex_rule!(Rule::filter_tag, i);
    }
}

#[test]
fn lex_macro_tag() {
    let inputs = vec![
        "{%- macro tag() %}",
        "{% macro my_block(name) -%}",
        "{% macro my_block(name=42) %}",
        "{% macro foo ( bar=\"baz\", qux=42 ) %}",
    ];
    for i in inputs {
        assert_lex_rule!(Rule::macro_tag, i);
    }
}

#[test]
fn lex_if_tag() {
    let inputs = vec![
        "{%- if name %}",
        "{% if true -%}",
        "{% if admin or show %}",
        "{% if 1 + 2 == 2 and true %}",
        "{% if 1 + 2 == 2 and admin is defined %}",
    ];
    for i in inputs {
        assert_lex_rule!(Rule::if_tag, i);
    }
}

#[test]
fn lex_elif_tag() {
    let inputs = vec![
        "{%- elif name %}",
        "{% elif true -%}",
        "{% elif admin or show %}",
        "{% elif 1 + 2 == 2 and true %}",
        "{% elif 1 + 2 == 2 and admin is defined %}",
    ];
    for i in inputs {
        assert_lex_rule!(Rule::elif_tag, i);
    }
}

#[test]
fn lex_else_tag() {
    assert!(TeraParser::parse(Rule::else_tag, "{% else %}").is_ok());
}

#[test]
fn lex_for_tag() {
    let inputs = vec![
        "{%- for a in array %}",
        "{% for a, b in object -%}",
        "{% for a, b in fn_call() %}",
        "{% for a in fn_call() %}",
        "{% for a in [] %}",
        "{% for a in [1,2,3,] %}",
        "{% for a,b in fn_call(with_args=true, name=name) %}",
        "{% for client in clients | slice(start=1, end=9) %}",
    ];

    for i in inputs {
        assert_lex_rule!(Rule::for_tag, i);
    }
}

#[test]
fn lex_break_tag() {
    assert!(TeraParser::parse(Rule::break_tag, "{% break %}").is_ok());
}

#[test]
fn lex_continue_tag() {
    assert!(TeraParser::parse(Rule::continue_tag, "{% continue %}").is_ok());
}

#[test]
fn lex_set_tag() {
    let inputs = vec![
        "{%- set a = true %}",
        "{% set a = object -%}",
        "{% set a = [1,2,3, 'hey'] -%}",
        "{% set a = fn_call() %}",
        "{% set a = fn_call(with_args=true, name=name) %}",
        "{% set a = macros::fn_call(with_args=true, name=name) %}",
        "{% set a = var | caps %}",
        "{% set a = var +1 >= 2%}",
    ];

    for i in inputs {
        assert_lex_rule!(Rule::set_tag, i);
    }
}

#[test]
fn lex_set_global_tag() {
    let inputs = vec![
        "{% set_global a = 1 %}",
        "{% set_global a = [1,2,3, 'hey'] -%}",
        "{% set_global a = another_var %}",
        "{% set_global a = another_var | filter %}",
        "{% set_global a = var +1 >= 2%}",
        "{%- set_global a = var +1 >= 2 -%}",
    ];

    for i in inputs {
        assert_lex_rule!(Rule::set_global_tag, i);
    }
}

#[test]
fn lex_variable_tag() {
    let inputs = vec![
        "{{ a }}",
        "{{ a | caps }}",
        r#"{{ "hey" }}"#,
        r#"{{ 'hey' }}"#,
        r#"{{ `hey` }}"#,
        "{{ fn_call() }}",
        "{{ macros::fn() }}",
        "{{ name + 42 }}",
        "{{ loop.index + 1 }}",
        "{{ name is defined and name >= 42 }}",
        "{{ my_macros::macro1(hello=\"world\", foo=bar, hey=1+2) }}",
        "{{ 'hello' ~ `ho` }}",
        r#"{{ hello ~ `ho` }}"#,
    ];

    for i in inputs {
        assert_lex_rule!(Rule::variable_tag, i);
    }
}

#[test]
fn lex_content() {
    let inputs = vec![
        "some text",
        "{{ name }}",
        "{# comment #}",
        "{% filter upper %}hey{% endfilter %}",
        "{% filter upper() %}hey{% endfilter %}",
        "{% raw %}{{ hey }}{% endraw %}",
        "{% for a in b %}{{a}}{% endfor %}",
        "{% if i18n %}世界{% else %}world{% endif %}",
    ];

    for i in inputs {
        assert_lex_rule!(Rule::content, i);
    }
}

#[test]
fn lex_template() {
    assert!(TeraParser::parse(
        Rule::template,
        "{# Greeter template #}
            Hello {% if i18n %}世界{% else %}world{% endif %}
            {% for country in countries %}
                {{ loop.index }}.{{ country }}
            {% endfor %}",
    )
    .is_ok());
}

#[test]
fn lex_extends_with_imports() {
    let sample = r#"
{% extends "base.html" %}

{% import "macros/image.html" as image %}
{% import "macros/masonry.html" as masonry %}
{% import "macros/breadcrumb.html" as breadcrumb %}
{% import "macros/ul_links.html" as ul_links %}
{% import "macros/location.html" as location %}
         "#;
    assert_lex_rule!(Rule::template, sample);
}

// https://github.com/Keats/tera/issues/379
#[test]
fn lex_requires_whitespace_between_things() {
    // All the ones below should fail parsing
    let inputs = vec![
        "{% filterupper %}hey{% endfilter %}",
        "{% blockhey %}{%endblock%}",
        "{% macrohey() %}{%endmacro%}",
        "{% setident = 1 %}",
        "{% set_globalident = 1 %}",
        "{% extends'base.html' %}",
        "{% import 'macros/image.html' asimage %}",
        "{% import'macros/image.html' as image %}",
        "{% fora in b %}{{a}}{% endfor %}",
        "{% for a inb %}{{a}}{% endfor %}",
        "{% for a,bin c %}{{a}}{% endfor %}",
        "{% for a,b inc %}{{a}}{% endfor %}",
        "{% ifi18n %}世界{% else %}world{% endif %}",
        "{% if i18n %}世界{% eliftrue %}world{% endif %}",
        "{% include'base.html' %}",
    ];

    for i in inputs {
        let res = TeraParser::parse(Rule::template, i);
        println!("{:?}", i);
        assert!(res.is_err());
    }
}
