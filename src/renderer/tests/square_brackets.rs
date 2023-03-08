use std::collections::HashMap;

use crate::context::Context;
use crate::tera::Tera;
use serde_derive::Serialize;

#[derive(Serialize)]
struct Test {
    a: String,
    b: String,
    c: Vec<String>,
}

#[test]
fn var_access_by_square_brackets() {
    let mut context = Context::new();
    context.insert(
        "var",
        &Test { a: "hi".into(), b: "i_am_actually_b".into(), c: vec!["fred".into()] },
    );
    context.insert("zero", &0);
    context.insert("a", "b");

    let mut map = HashMap::new();
    map.insert("true", "yes");
    map.insert("false", "no");
    map.insert("with space", "works");
    map.insert("with/slash", "works");
    let mut deep_map = HashMap::new();
    deep_map.insert("inner_map", &map);
    context.insert("map", &map);
    context.insert("deep_map", &deep_map);
    context.insert("bool_vec", &vec!["true", "false"]);

    let inputs = vec![
        ("{{var.a}}", "hi"),
        ("{{var['a']}}", "hi"),
        ("{{var[\"a\"]}}", "hi"),
        ("{{var['c'][0]}}", "fred"),
        ("{{var['c'][zero]}}", "fred"),
        ("{{var[a]}}", "i_am_actually_b"),
        ("{{map['with space']}}", "works"),
        ("{{map['with/slash']}}", "works"),
        ("{{deep_map['inner_map'][bool_vec[zero]]}}", "yes"),
    ];

    for (input, expected) in inputs {
        let result = Tera::one_off(input, &context, true).unwrap();
        println!("{:?} -> {:?} = {:?}", input, expected, result);
        assert_eq!(result, expected);
    }
}

#[test]
fn var_access_by_square_brackets_errors() {
    let mut context = Context::new();
    context.insert("var", &Test { a: "hi".into(), b: "there".into(), c: vec![] });
    let t = Tera::one_off("{{var[csd]}}", &context, true);
    assert!(t.is_err(), "Access of csd should be impossible");
}

// https://github.com/Keats/tera/issues/334
#[test]
fn var_access_by_loop_index() {
    let context = Context::new();
    let res = Tera::one_off(
        r#"
{% set ics = ["fa-rocket","fa-paper-plane","fa-diamond","fa-signal"] %}
{% for a in ics %}
{{ ics[loop.index0] }}
{% endfor %}
    "#,
        &context,
        true,
    );
    assert!(res.is_ok());
}

// https://github.com/Keats/tera/issues/334
#[test]
fn var_access_by_loop_index_with_set() {
    let context = Context::new();
    let res = Tera::one_off(
        r#"
{% set ics = ["fa-rocket","fa-paper-plane","fa-diamond","fa-signal"] %}
{% for a in ics %}
    {% set i = loop.index - 1 %}
    {{ ics[i] }}
{% endfor %}
    "#,
        &context,
        true,
    );
    assert!(res.is_ok());
}

// https://github.com/Keats/tera/issues/754
#[test]
fn can_get_value_if_key_contains_period() {
    let mut context = Context::new();
    context.insert("name", "Mt. Robson Provincial Park");
    let mut map = HashMap::new();
    map.insert("Mt. Robson Provincial Park".to_string(), "hello".to_string());
    context.insert("tag_info", &map);

    let res = Tera::one_off(r#"{{ tag_info[name] }}"#, &context, true);
    assert!(res.is_ok());
    let res = res.unwrap();
    assert_eq!(res, "hello");
}
