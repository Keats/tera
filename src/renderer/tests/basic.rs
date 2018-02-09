use std::collections::BTreeMap;
use context::Context;
use errors::Result;
use tera::Tera;

use super::Review;

fn render_template(content: &str, context: &Context) -> Result<String> {
    let mut tera = Tera::default();
    tera.add_raw_template("hello.html", content).unwrap();

    tera.render("hello.html", context)
}

#[test]
fn render_simple_string() {
    let result = render_template("<h1>Hello world</h1>", &Context::new());
    assert_eq!(result.unwrap(), "<h1>Hello world</h1>".to_owned());
}

#[test]
fn render_variable_block_lit_expr() {
    let inputs = vec![
        ("{{ 1 }}", "1"),
        ("{{ 3.14 }}", "3.14"),
        ("{{ \"hey\" }}", "hey"),
        (r#"{{ "{{ hey }}" }}"#, "{{ hey }}"),
        ("{{ true }}", "true"),
        ("{{ false }}", "false"),
        ("{{ 1 + 1 }}", "2"),
        ("{{ 1 + 1.1 }}", "2.1"),
        ("{{ 3 - 1 }}", "2"),
        ("{{ 3 - 1.1 }}", "1.9"),
        ("{{ 2 * 5 }}", "10"),
        ("{{ 10 / 5 }}", "2"),
        ("{{ 2.1 * 5 }}", "10.5"),
        ("{{ 2.1 * 5.05 }}", "10.605"),
        ("{{ 2 / 0.5 }}", "4"),
        ("{{ 2.1 / 0.5 }}", "4.2"),
        ("{{ 2 + 1 * 2 }}", "4"),
        ("{{ (2 + 1) * 2 }}", "6"),
        ("{{ 2 * 4 % 8 }}", "0"),
        ("{{ 2.8 * 2 | round }}", "6"),
        ("{{ 0 / 0 }}", "NaN"),
        ("{{ true and 10 }}", "true"),
        ("{{ true and not 10 }}", "false"),
        ("{{ not true }}", "false"),
    ];

    for (input, expected) in inputs {
        println!("{:?} -> {:?}", input, expected);
        assert_eq!(render_template(input, &Context::new()).unwrap(), expected);
    }
}

#[test]
fn render_variable_block_ident() {
    let mut context = Context::new();
    context.add("name", &"john");
    context.add("malicious", &"<html>");
    context.add("a", &2);
    context.add("b", &3);
    context.add("numbers", &vec![1, 2, 3]);
    context.add("tuple_list", &vec![(1, 2, 3), (1, 2, 3)]);
    context.add("review", &Review::new());

    let inputs = vec![
        ("{{ name }}", "john"),
        ("{{ malicious }}", "&lt;html&gt;"),
        ("{{ \"<html>\" }}", "&lt;html&gt;"),
        ("{{ \" html \" | upper | trim }}", "HTML"),
        ("{{ malicious | safe }}", "<html>"),
        ("{{ malicious | upper }}", "&LT;HTML&GT;"), // everything upper eh
        ("{{ malicious | upper | safe }}", "&LT;HTML&GT;"),
        ("{{ malicious | safe | upper }}", "<HTML>"),
        ("{{ review.paragraphs.1 }}", "B"),
        ("{{ numbers }}", "[1, 2, 3]"),
        ("{{ numbers.0 }}", "1"),
        ("{{ tuple_list.1.1 }}", "2"),
        ("{{ name and true }}", "true"),
        ("{{ name | length }}", "4"),
        ("{{ name is defined }}", "true"),
        ("{{ not name is defined }}", "false"),
        ("{{ a is odd }}", "false"),
        ("{{ a is odd or b is odd  }}", "true"),
        ("{{ range(start=1, end=4) }}", "[1, 2, 3]"),
        ("{{ a + b }}", "5"),
        ("{{ a + 1.5 }}", "3.5"),
        ("{{ 1 + 1 + 1 }}", "3"),
        ("{{ 2 - 2 - 1 }}", "-1"),
        ("{{ 1 - 1 + 1 }}", "1"),
        ("{{ (1.9 + a) | round }}", "4"),
        ("{{ 1.9 + a | round }}", "4"),
        ("{{ numbers | length - 1 }}", "2"),
        ("{{ 1.9 + a | round - 1 }}", "3"),
        ("{{ 1.9 + a | round - 1.8 + a | round }}", "0"),
        ("{{ 1.9 + a | round - 1.8 + a | round - 1 }}", "-1"),
    ];

    for (input, expected) in inputs {
        println!("{:?} -> {:?}", input, expected);
        assert_eq!(render_template(input, &context).unwrap(), expected);
    }
}

#[test]
fn render_variable_block_logic_expr() {
    let mut context = Context::new();
    context.add("name", &"john");
    context.add("malicious", &"<html>");
    context.add("a", &2);
    context.add("b", &3);
    context.add("numbers", &vec![1, 2, 3]);
    context.add("tuple_list", &vec![(1, 2, 3), (1, 2, 3)]);

    let inputs = vec![
        ("{{ (1.9 + a) | round > 10 }}", "false"),
        ("{{ (1.9 + a) | round > 10 or b > a }}", "true"),
        (
            "{{ 1.9 + a | round == 4 and numbers | length == 3}}",
            "true",
        ),
        ("{{ numbers | length > 1 }}", "true"),
        ("{{ numbers | length == 1 }}", "false"),
        ("{{ numbers | length - 2 == 1 }}", "true"),
        ("{{ not name }}", "false"),
        ("{{ not true }}", "false"),
        ("{{ not undefined }}", "true"),
    ];

    for (input, expected) in inputs {
        println!("{:?} -> {:?}", input, expected);
        assert_eq!(render_template(input, &context).unwrap(), expected);
    }
}

#[test]
fn render_variable_block_autoescaping_disabled() {
    let mut context = Context::new();
    context.add("name", &"john");
    context.add("malicious", &"<html>");

    let inputs = vec![
        ("{{ name }}", "john"),
        ("{{ malicious }}", "<html>"),
        ("{{ malicious | safe }}", "<html>"),
        ("{{ malicious | upper }}", "<HTML>"),
        ("{{ malicious | upper | safe }}", "<HTML>"),
        ("{{ malicious | safe | upper }}", "<HTML>"),
    ];

    for (input, expected) in inputs {
        let mut tera = Tera::default();
        tera.add_raw_template("hello.sql", input).unwrap();
        assert_eq!(tera.render("hello.sql", &context).unwrap(), expected);
    }
}

#[test]
fn comments_are_ignored() {
    let inputs = vec![
        ("Hello {# comment #}world", "Hello world"),
        ("Hello {# comment {# nested #}world", "Hello world"),
        (
            "My name {# was {{ name }} #}is No One.",
            "My name is No One.",
        ),
    ];

    for (input, expected) in inputs {
        println!("{:?} -> {:?}", input, expected);
        assert_eq!(render_template(input, &Context::new()).unwrap(), expected);
    }
}

#[test]
fn filter_args_are_not_escaped() {
    let mut context = Context::new();
    context.add("my_var", &"hey");
    context.add("to", &"&");
    let input = r#"{{ my_var | replace(from="h", to=to) }}"#;

    assert_eq!(render_template(input, &context).unwrap(), "&ey");
}

#[test]
fn render_include_tag() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("world", "world"),
        ("hello", "<h1>Hello {% include \"world\" %}</h1>"),
    ]).unwrap();
    let result = tera.render("hello", &Context::new()).unwrap();
    assert_eq!(result, "<h1>Hello world</h1>".to_owned());
}

#[test]
fn render_raw_tag() {
    let inputs = vec![
        ("{% raw %}hey{% endraw %}", "hey"),
        ("{% raw %}{{hey}}{% endraw %}", "{{hey}}"),
        ("{% raw %}{% if true %}{% endraw %}", "{% if true %}"),
    ];

    for (input, expected) in inputs {
        println!("{:?} -> {:?}", input, expected);
        assert_eq!(render_template(input, &Context::new()).unwrap(), expected);
    }
}

#[test]
fn add_set_values_in_context() {
    let mut context = Context::new();
    context.add("my_var", &"hey");
    context.add("malicious", &"<html>");
    context.add("admin", &true);
    context.add("num", &1);

    let inputs = vec![
        ("{% set i = 1 %}{{ i }}", "1"),
        ("{% set i = 1 + 2 %}{{ i }}", "3"),
        (r#"{% set i = "hey" %}{{ i }}"#, "hey"),
        (r#"{% set i = "<html>" %}{{ i | safe }}"#, "<html>"),
        (r#"{% set i = "<html>" %}{{ i }}"#, "&lt;html&gt;"),
        ("{% set i = my_var %}{{ i }}", "hey"),
        ("{% set i = malicious %}{{ i | safe }}", "<html>"),
        ("{% set i = malicious %}{{ i }}", "&lt;html&gt;"),
        ("{% set i = my_var | upper %}{{ i }}", "HEY"),
        ("{% set i = range(end=3) %}{{ i }}", "[0, 1, 2]"),
        ("{% set i = admin or true %}{{ i }}", "true"),
        ("{% set i = admin and num > 0 %}{{ i }}", "true"),
        ("{% set i = 0 / 0 %}{{ i }}", "NaN"),
    ];

    for (input, expected) in inputs {
        println!("{:?} -> {:?}", input, expected);
        assert_eq!(render_template(input, &context).unwrap(), expected);
    }
}

#[test]
fn render_filter_section() {
    let result = render_template("{% filter upper %}Hello{% endfilter %}", &Context::new());

    assert_eq!(result.unwrap(), "HELLO".to_owned());
}

#[test]
fn render_if_elif_else() {
    let mut context = Context::new();
    context.add("is_true", &true);
    context.add("is_false", &false);
    context.add("age", &18);
    context.add("numbers", &vec![1, 2, 3]);

    let inputs = vec![
        ("{% if is_true %}Admin{% endif %}", "Admin"),
        ("{% if is_true or age + 1 > 18 %}Adult{% endif %}", "Adult"),
        ("{% if is_true and age == 18 %}Adult{% endif %}", "Adult"),
        // https://github.com/Keats/tera/issues/187
        ("{% if 1 <= 2 %}a{% endif %}", "a"),
        ("{% if 2 >= 1 %}a{% endif %}", "a"),
        ("{% if 1 < 2 %}a{% endif %}", "a"),
        ("{% if 2 > 1 %}a{% endif %}", "a"),
        ("{% if 1 == 1 %}a{% endif %}", "a"),
        ("{% if 1 != 2 %}a{% endif %}", "a"),
        // some not conditions
        ("{% if not is_false %}a{% endif %}", "a"),
        ("{% if not is_true %}a{% endif %}", ""),
        ("{% if undefined %}a{% endif %}", ""),
        ("{% if not undefined %}a{% endif %}", "a"),
        ("{% if not is_false and is_true %}a{% endif %}", "a"),
        (
            "{% if not is_false or numbers | length > 0 %}a{% endif %}",
            "a",
        ),
        // doesn't panic with NaN results
        ("{% if 0 / 0 %}a{% endif %}", ""),
        // if and else
        ("{% if is_true %}Admin{% else %}User{% endif %}", "Admin"),
        ("{% if is_false %}Admin{% else %}User{% endif %}", "User"),
        // if and elifs
        (
            "{% if is_true %}Admin{% elif is_false %}User{% endif %}",
            "Admin",
        ),
        (
            "{% if is_true %}Admin{% elif is_true %}User{% endif %}",
            "Admin",
        ),
        (
            "{% if is_true %}Admin{% elif numbers | length > 0 %}User{% endif %}",
            "Admin",
        ),
        // if, elifs and else
        (
            "{% if is_true %}Admin{% elif is_false %}User{% else %}Hmm{% endif %}",
            "Admin",
        ),
        (
            "{% if false %}Admin{% elif is_false %}User{% else %}Hmm{% endif %}",
            "Hmm",
        ),
        // doesn't fallthrough elifs
        // https://github.com/Keats/tera/issues/188
        (
            "{% if 1 < 4 %}a{% elif 2 < 4 %}b{% elif 3 < 4 %}c{% else %}d{% endif %}",
            "a",
        ),
    ];

    for (input, expected) in inputs {
        println!("{:?} -> {:?}", input, expected);
        assert_eq!(render_template(input, &context).unwrap(), expected);
    }
}

#[test]
fn render_for() {
    let mut context = Context::new();
    let mut map = BTreeMap::new();
    map.insert("name", "bob");
    map.insert("age", "18");

    context.add("data", &vec![1, 2, 3]);
    context.add("notes", &vec![1, 2, 3]);
    context.add("vectors", &vec![vec![0, 3, 6], vec![1, 4, 7]]);
    context.add(
        "vectors_some_empty",
        &vec![vec![0, 3, 6], vec![], vec![1, 4, 7]],
    );
    context.add("map", &map);
    context.add("truthy", &2);

    let inputs = vec![
        ("{% for i in data %}{{i}}{% endfor %}", "123"),
        ("{% for key, val in map %}{{key}}:{{val}} {% endfor %}", "age:18 name:bob "),
        (
            "{% for i in data %}{{loop.index}}{{loop.index0}}{{loop.first}}{{loop.last}}{% endfor %}",
            "10truefalse21falsefalse32falsetrue"
        ),
        (
            "{% for vector in vectors %}{% for j in vector %}{{ j }}{% endfor %}{% endfor %}",
            "036147"
        ),
        (
            "{% for vector in vectors_some_empty %}{% for j in vector %}{{ j }}{% endfor %}{% endfor %}",
            "036147"
        ),
        (
            "{% for val in data %}{% if val == truthy %}on{% else %}off{% endif %}{% endfor %}",
            "offonoff"
        ),
        ("{% for i in range(end=5) %}{{i}}{% endfor %}", "01234"),
        ("{% for i in range(end=5) | reverse %}{{i}}{% endfor %}", "43210"),
        (
            "{% set looped = 0 %}{% for i in range(end=5) %}{% set looped = i %}{{looped}}{% endfor%}{{looped}}",
            "012340"
        ),
        // https://github.com/Keats/tera/issues/184
        ("{% for note in notes %}{{ note }}{% endfor %}", "123"),
        ("{% for note in notes | reverse %}{{ note }}{% endfor %}", "321"),
        ("{% for v in vectors %}{{ v.0 }}{% endfor %}", "01"),
    ];

    for (input, expected) in inputs {
        println!("{:?} -> {:?}", input, expected);
        assert_eq!(render_template(input, &context).unwrap(), expected);
    }
}

#[test]
fn render_magic_variable_isnt_escaped() {
    let mut context = Context::new();
    context.add("html", &"<html>");

    let result = render_template("{{ __tera_context }}", &context);

    assert_eq!(
        result.unwrap(),
        r#"{
  "html": "<html>"
}"#.to_owned()
    );
}

// https://github.com/Keats/tera/issues/185
#[test]
fn ok_many_variable_blocks() {
    let mut context = Context::new();
    context.add("username", &"bob");

    let mut tpl = String::new();
    for i in 0..200 {
        tpl.push_str("{{ username }}")
    }
    let mut expected = String::new();
    for i in 0..200 {
        expected.push_str("bob")
    }
    assert_eq!(render_template(&tpl, &context).unwrap(), expected);
}

#[test]
fn can_set_variable_in_global_context_in_forloop() {
    let mut context = Context::new();
    context.add("tags", &vec![1, 2, 3]);
    context.add("default", &"default");

    let result = render_template(
        r#"
{%- for i in tags -%}
{%- set default = 1 -%}
{%- set_global global_val = i -%}
{%- endfor -%}
{{ default }}{{ global_val }}"#,
        &context,
    );

    assert_eq!(result.unwrap(), "default3");
}

#[test]
fn default_filter_works() {
    let mut context = Context::new();
    context.add("existing", "hello");

    let inputs = vec![
        (r#"{{ existing | default(value="hey") }}"#, "hello"),
        (r#"{{ val | default(value=1) }}"#, "1"),
        (r#"{{ val | default(value="hey") | capitalize }}"#, "Hey"),
        (
            r#"{{ obj.val | default(value="hey") | capitalize }}"#,
            "Hey",
        ),
        (
            r#"{{ obj.val | default(value="hey") | capitalize }}"#,
            "Hey",
        ),
        (r#"{{ not admin | default(value=false) }}"#, "true"),
        (r#"{{ not admin | default(value=true) }}"#, "false"),
    ];

    for (input, expected) in inputs {
        println!("{:?} -> {:?}", input, expected);
        assert_eq!(render_template(input, &context).unwrap(), expected);
    }
}
