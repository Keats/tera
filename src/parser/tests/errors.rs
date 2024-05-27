use crate::parser::parse;

fn assert_err_msg(input: &str, needles: &[&str]) {
    let res = parse(input);
    assert!(res.is_err());
    let err = res.unwrap_err();
    let err_msg = err.to_string();
    println!("{}", err_msg);
    println!("Looking for:");
    for needle in needles {
        println!("{}", needle);
        assert!(err_msg.contains(needle));
    }
}

#[test]
fn invalid_number() {
    assert_err_msg(
        "{{ 1.2.2 }}",
        &[
            "1:7",
            "expected `or`, `and`, `not`, `<=`, `>=`, `<`, `>`, `==`, `!=`, `+`, `-`, `*`, `/`, `%`, a filter, or a variable end (`}}`)"
        ],
    );
}

#[test]
fn invalid_op() {
    assert_err_msg("{{ 1.2 >+ 3 }}", &["1:9", "expected an expression"]);
}

#[test]
fn wrong_start_block() {
    assert_err_msg(
        "{{ if true %}",
        &[
            "1:7",
            "expected `or`, `and`, `not`, `<=`, `>=`, `<`, `>`, `==`, `!=`, `+`, `-`, `*`, `/`, `%`, a filter, or a variable end (`}}`)"
        ],
    );
}

#[test]
fn wrong_end_block() {
    assert_err_msg(
        "{{ hey %}",
        &[
            "1:9",
            "expected an integer, a float, `true` or `false`, an identifier (must start with a-z), a square bracketed identifier (identifiers separated by `.` or `[]`s), or an expression"
        ],
    );
}

#[test]
fn unterminated_variable_block() {
    assert_err_msg(
        "{{ hey",
        &[
            "1:7",
            "expected `or`, `and`, `not`, `<=`, `>=`, `<`, `>`, `==`, `!=`, `+`, `-`, `*`, `/`, `%`, a filter, or a variable end (`}}`)"
        ],
    );
}

#[test]
fn unterminated_string() {
    assert_err_msg(r#"{{ "hey }}"#, &["1:4", "expected a value that can be negated"]);
}

#[test]
fn unterminated_if_tag() {
    assert_err_msg(r#"{% if true %}sd"#, &["1:16", r#"expected tag or some content"#]);
}

#[test]
fn unterminated_filter_section() {
    assert_err_msg(
        r#"{% filter uppercase %}sd"#,
        &["1:25", r#"expected tag or the filter section content"#],
    );
}

#[test]
fn invalid_filter_section_missing_name() {
    assert_err_msg(
        r#"{% filter %}sd{% endfilter %}"#,
        &["1:11", "expected an identifier (must start with a-z)"],
    );
}

#[test]
fn invalid_macro_content() {
    assert_err_msg(
        r#"
{% macro input(label, type) %}
    {% macro nested() %}
    {% endmacro nested %}
{% endmacro input %}
    "#,
        &["3:5", "unexpected tag; expected `{% endmacro %}` or the macro content"],
    );
}

#[test]
fn invalid_macro_not_toplevel() {
    assert_err_msg(
        r#"
{% if val %}
    {% macro input(label, type) %}
    {% endmacro input %}
{% endif %}
    "#,
        &["3:5", "unexpected tag; expected an `elif` tag, an `else` tag, an endif tag (`{% endif %}`), or some content"],
    );
}

#[test]
fn invalid_macro_default_arg_value() {
    assert_err_msg(
        r#"
{% macro input(label=something) %}
{% endmacro input %}
    "#,
        &["2:22", "expected an integer, a float, a string, or `true` or `false`"],
    );
}

#[test]
fn invalid_elif() {
    assert_err_msg(
        r#"
{% if true %}
{% else %}
{% elif false %}
{% endif %}
    "#,
        &["4:1", "unexpected tag; expected an endif tag (`{% endif %}`) or some content"],
    );
}

#[test]
fn invalid_else() {
    assert_err_msg(
        r#"
{% if true %}
{% else %}
{% else %}
{% endif %}
    "#,
        &["4:1", "unexpected tag; expected an endif tag (`{% endif %}`) or some content"],
    );
}

#[test]
fn invalid_extends_position() {
    assert_err_msg(
        r#"
hello
{% extends "hey.html" %}
    "#,
        &["3:1", "unexpected tag; expected end of input, a macro definition tag (`{% macro my_macro() %}`, or some content"],
    );
}

#[test]
fn invalid_operator() {
    assert_err_msg(
        "{{ hey =! }}",
        &[
            "1:8",
            "expected `or`, `and`, `not`, `<=`, `>=`, `<`, `>`, `==`, `!=`, `+`, `-`, `*`, `/`, `%`, a filter, or a variable end (`}}`)"
        ],
    );
}

#[test]
fn missing_expression_with_not() {
    assert_err_msg("{% if not %}", &["1:11", "expected an expression"]);
}

#[test]
fn missing_expression_in_if() {
    assert_err_msg("{% if %}", &["1:7", "expected a value that can be negated"]);
}

#[test]
fn missing_container_name_in_forloop() {
    assert_err_msg("{% for i in %}", &["1:13", "expected an expression or an array of values"]);
}

#[test]
fn missing_variable_name_in_set() {
    assert_err_msg("{% set = 1 %}", &["1:8", "expected an identifier (must start with a-z)"]);
}

#[test]
fn missing_value_in_set() {
    assert_err_msg(
        "{% set a =  %}",
        &["1:13", "expected a value that can be negated or an array of values"],
    );
}

#[test]
fn unterminated_fn_call() {
    assert_err_msg("{{ a | slice( }}", &["1:15", "expected an identifier (must start with a-z)"]);
}

#[test]
fn invalid_fn_call_missing_value() {
    assert_err_msg(
        "{{ a | slice(start=) }}",
        &["1:20", "expected a value that can be negated or an array of values"],
    );
}

#[test]
fn unterminated_macro_call() {
    assert_err_msg("{{ my::macro( }}", &["1:15", "expected an identifier (must start with a-z)"]);
}

#[test]
fn invalid_macro_call() {
    assert_err_msg(
        "{{ my:macro() }}",
        &[
            "1:6",
            "expected `or`, `and`, `not`, `<=`, `>=`, `<`, `>`, `==`, `!=`, `+`, `-`, `*`, `/`, `%`, a filter, or a variable end (`}}`)"
        ],
    );
}

#[test]
fn unterminated_include() {
    assert_err_msg("{% include %}", &["1:12", "expected a string"]);
}

#[test]
fn invalid_include_no_string() {
    assert_err_msg("{% include 1 %}", &["1:12", "expected a string"]);
}

#[test]
fn unterminated_extends() {
    assert_err_msg("{% extends %}", &["1:12", "expected a string"]);
}

#[test]
fn invalid_extends_no_string() {
    assert_err_msg("{% extends 1 %}", &["1:12", "expected a string"]);
}

#[test]
fn invalid_import_macros_missing_filename() {
    assert_err_msg("{% import as macros %}", &["1:11", "expected a string"]);
}

#[test]
fn invalid_import_macros_missing_namespace() {
    assert_err_msg(
        r#"{% import "hello" as %}"#,
        &["1:22", "expected an identifier (must start with a-z)"],
    );
}

#[test]
fn invalid_block_missing_name() {
    assert_err_msg(r#"{% block %}"#, &["1:10", "expected an identifier (must start with a-z)"]);
}

#[test]
fn unterminated_test() {
    assert_err_msg(
        r#"{% if a is odd( %}"#,
        &["1:17", "a test argument (any expressions including arrays)"],
    );
}

#[test]
fn invalid_test_argument() {
    assert_err_msg(
        r#"{% if a is odd(key=1) %}"#,
        &[
            "1:19",
            "expected `or`, `and`, `not`, `<=`, `>=`, `<`, `>`, `==`, `!=`, `+`, `-`, `*`, `/`, `%`, or a filter"
        ],
    );
}

#[test]
fn unterminated_raw_tag() {
    assert_err_msg(r#"{% raw %}sd"#, &["1:12", "expected tag"]);
}

#[test]
fn invalid_break_outside_loop() {
    assert_err_msg(r#"{% break %}"#, &["1:1", "{% break %}", "expected a template"]);
}

#[test]
fn invalid_continue_outside_loop() {
    assert_err_msg(r#"{% continue %}"#, &["1:1", "{% continue %}", "expected a template"]);
}
