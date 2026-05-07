use std::collections::BTreeMap;

use serde_derive::Serialize;

use tera::{Context, Tera, Value};

#[derive(Debug, Serialize, Default)]
pub struct SomeStruct {
    pub label: String,
}

// - `{{ hey }}` should error if hey is undefined
// - `{{ existing.hey }}` sould error if hey is undefined but existing is
// - `{{ hey or 1 }}` should print 1
// - `{% if hey or true %}` should be truthy
// - `{% if hey.other or true %}` should error if `hey` is not defined (currently truthy)
// - `{{ hey.other or 1 }}` should error if `hey` is not defined (currently prints "true")
#[test]
fn handles_undefined_correctly() {
    let mut data = BTreeMap::new();
    data.insert("key".to_string(), Value::undefined());
    let mut context = Context::new();
    context.insert_value("m", Value::from(data));
    let existing = SomeStruct::default();
    context.insert("existing", &existing);

    let tests = vec![
        ("{{ hey }}", None),
        ("{{ existing.hey }}", None),
        ("{{ hey or 1 }}", Some("1")),
        ("{{ hey.other or 1 }}", None),
        ("{% if hey or true %}truthy{% endif %}", Some("truthy")),
        ("{% if hey.other or true %}truthy{% endif %}", None),
        ("{{ m.key | default(value='fallback') }}", Some("fallback")),
        ("{{ m.key.foo | default(value='fallback') }}", None),
    ];

    for (input, expected) in tests {
        println!("{input:?}");
        let mut tera = Tera::default();
        tera.add_raw_template("tpl", input).unwrap();
        let res = tera.render("tpl", &context);
        if let Some(expected_output) = expected {
            assert_eq!(expected_output, res.unwrap());
        } else {
            println!("{:?}", res);
            assert!(res.is_err());
        }
    }
}
