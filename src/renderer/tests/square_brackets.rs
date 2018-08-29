use std::collections::HashMap;

use context::Context;
use tera::Tera;


#[derive(Serialize)]
struct Test {
    a: String,
    b: String,
    c: Vec<String>,
}

#[test]
fn test_var_access_by_square_brackets() {
    let mut context = Context::new();
    context.add(
        "var",
        &Test { a: "hi".into(), b: "i_am_actually_b".into(), c: vec!["fred".into()] },
    );
    context.add("zero", &0);
    context.add("a", "b");

    let mut map = HashMap::new();
    map.insert("true", "yes");
    map.insert("false", "no");
    let mut deep_map = HashMap::new();
    deep_map.insert("inner_map", &map);
    context.add("deep_map", &deep_map);
    context.add("bool_vec", &vec!["true", "false"]);

    let inputs = vec![
        ("{{var.a}}", "hi"),
        ("{{var['a']}}", "hi"),
        ("{{var[\"a\"]}}", "hi"),
        ("{{var['c'][0]}}", "fred"),
        ("{{var['c'][zero]}}", "fred"),
        ("{{var[a]}}", "i_am_actually_b"),
        ("{{deep_map['inner_map'][bool_vec[zero]]}}", "yes"),
    ];

    for (input, expected) in inputs {
        println!("{:?} -> {:?}", input, expected);
        assert_eq!(Tera::one_off(input, &context, true).unwrap(), expected);
    }
}

#[test]
fn test_var_access_by_square_brackets_errors() {
    let mut context = Context::new();
    context.add("var", &Test { a: "hi".into(), b: "there".into(), c: vec![] });
    let t = Tera::one_off("{{var[csd]}}", &context, true);
    assert!(t.is_err(), "Access of csd should be impossible");
}
