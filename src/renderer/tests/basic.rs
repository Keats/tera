use std::collections::BTreeMap;

use serde_json::Value;

use context::Context;
use errors::Result;
use tera::Tera;

use super::Review;

fn render_template(content: &str, context: &Context) -> Result<String> {
    let mut tera = Tera::default();
    tera.add_raw_template("hello.html", content).unwrap();
    tera.register_function("get_number", Box::new(|_| Ok(Value::Number(10.into()))));
    tera.register_function("get_string", Box::new(|_| Ok(Value::String("Hello".to_string()))));

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
    context.insert("name", &"john");
    context.insert("malicious", &"<html>");
    context.insert("a", &2);
    context.insert("b", &3);
    context.insert("numbers", &vec![1, 2, 3]);
    context.insert("tuple_list", &vec![(1, 2, 3), (1, 2, 3)]);
    context.insert("review", &Review::new());

    let inputs = vec![
        ("{{ name }}", "john"),
        ("{{ malicious }}", "&lt;html&gt;"),
        ("{{ \"<html>\" }}", "&lt;html&gt;"),
        ("{{ \" html \" | upper | trim }}", "HTML"),
        ("{{ 'html' }}", "html"),
        ("{{ `html` }}", "html"),
        // https://github.com/Keats/tera/issues/273
        (
            r#"{{ 'hangar new "Will Smoth <will_s@example.com>"' | safe }}"#,
            r#"hangar new "Will Smoth <will_s@example.com>""#,
        ),
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
        ("{{ 1 + get_number() }}", "11"),
        ("{{ get_number() + 1 }}", "11"),
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
    context.insert("name", &"john");
    context.insert("malicious", &"<html>");
    context.insert("a", &2);
    context.insert("b", &3);
    context.insert("numbers", &vec![1, 2, 3]);
    context.insert("tuple_list", &vec![(1, 2, 3), (1, 2, 3)]);

    let inputs = vec![
        ("{{ (1.9 + a) | round > 10 }}", "false"),
        ("{{ (1.9 + a) | round > 10 or b > a }}", "true"),
        ("{{ 1.9 + a | round == 4 and numbers | length == 3}}", "true"),
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
    context.insert("name", &"john");
    context.insert("malicious", &"<html>");

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
        ("My name {# was {{ name }} #}is No One.", "My name is No One."),
    ];

    for (input, expected) in inputs {
        println!("{:?} -> {:?}", input, expected);
        assert_eq!(render_template(input, &Context::new()).unwrap(), expected);
    }
}

#[test]
fn filter_args_are_not_escaped() {
    let mut context = Context::new();
    context.insert("my_var", &"hey");
    context.insert("to", &"&");
    let input = r#"{{ my_var | replace(from="h", to=to) }}"#;

    assert_eq!(render_template(input, &context).unwrap(), "&ey");
}

#[test]
fn render_include_tag() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("world", "world"),
        ("hello", "<h1>Hello {% include \"world\" %}</h1>"),
    ])
    .unwrap();
    let result = tera.render("hello", &Context::new()).unwrap();
    assert_eq!(result, "<h1>Hello world</h1>".to_owned());
}

#[test]
fn can_set_variables_in_included_templates() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("world", r#"{% set a = "world" %}{{a}}"#),
        ("hello", "<h1>Hello {% include \"world\" %}</h1>"),
    ])
    .unwrap();
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
    context.insert("my_var", &"hey");
    context.insert("malicious", &"<html>");
    context.insert("admin", &true);
    context.insert("num", &1);

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
        ("{% set i = [1,2] %}{{ i }}", "[1, 2]"),
    ];

    for (input, expected) in inputs {
        println!("{:?} -> {:?}", input, expected);
        assert_eq!(render_template(input, &context).unwrap(), expected);
    }
}

#[test]
fn render_filter_section() {
    let inputs = vec![
        ("{% filter upper %}Hello{% endfilter %}", "HELLO"),
        ("{% filter upper %}Hello{% if true %} world{% endif %}{% endfilter %}", "HELLO WORLD"),
        ("{% filter upper %}Hello {% for i in range(end=3) %}i{% endfor %}{% endfilter %}", "HELLO III"),
        (
            "{% filter upper %}Hello {% for i in range(end=3) %}{% if i == 1 %}{% break %} {% endif %}i{% endfor %}{% endfilter %}",
            "HELLO I",
        ),
        ("{% filter title %}Hello {% if true %}{{ 'world' | upper | safe }}{% endif %}{% endfilter %}", "Hello World"),
    ];

    let context = Context::new();
    for (input, expected) in inputs {
        println!("{:?} -> {:?}", input, expected);
        assert_eq!(render_template(input, &context).unwrap(), expected);
    }
}

#[test]
fn render_if_elif_else() {
    let mut context = Context::new();
    context.insert("is_true", &true);
    context.insert("is_false", &false);
    context.insert("age", &18);
    context.insert("numbers", &vec![1, 2, 3]);

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
        ("{% if not is_false or numbers | length > 0 %}a{% endif %}", "a"),
        // doesn't panic with NaN results
        ("{% if 0 / 0 %}a{% endif %}", ""),
        // if and else
        ("{% if is_true %}Admin{% else %}User{% endif %}", "Admin"),
        ("{% if is_false %}Admin{% else %}User{% endif %}", "User"),
        // if and elifs
        ("{% if is_true %}Admin{% elif is_false %}User{% endif %}", "Admin"),
        ("{% if is_true %}Admin{% elif is_true %}User{% endif %}", "Admin"),
        ("{% if is_true %}Admin{% elif numbers | length > 0 %}User{% endif %}", "Admin"),
        // if, elifs and else
        ("{% if is_true %}Admin{% elif is_false %}User{% else %}Hmm{% endif %}", "Admin"),
        ("{% if false %}Admin{% elif is_false %}User{% else %}Hmm{% endif %}", "Hmm"),
        // doesn't fallthrough elifs
        // https://github.com/Keats/tera/issues/188
        ("{% if 1 < 4 %}a{% elif 2 < 4 %}b{% elif 3 < 4 %}c{% else %}d{% endif %}", "a"),
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

    context.insert("data", &vec![1, 2, 3]);
    context.insert("notes", &vec![1, 2, 3]);
    context.insert("vectors", &vec![vec![0, 3, 6], vec![1, 4, 7]]);
    context.insert("vectors_some_empty", &vec![vec![0, 3, 6], vec![], vec![1, 4, 7]]);
    context.insert("map", &map);
    context.insert("truthy", &2);

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
        // Loop control (`break` and `continue`)
        // https://github.com/Keats/tera/issues/267
        (
            "{% for i in data %}{{ i }}{% if i == 2 %}{% break %}{% endif %}{% endfor %}",
            "12"
        ),
        (
            "{% for i in data %}{% if i == 2 %}{% continue %}{% endif %}{{ i }}{% endfor %}",
            "13"
        ),
        (
            "{% for v in vectors %}{% for i in v %}{% if i == 3 %}{% break %}{% endif %}{{ i }}{% endfor %}{% endfor %}",
            "0147"
        ),
        (
            "{% for v in vectors %}{% for i in v %}{% if i == 3 %}{% continue %}{% endif %}{{ i }}{% endfor %}{% endfor %}",
            "06147"
        ),
        (
            "{% for a in [1, true, 1.1, 'hello'] %}{{a}}{% endfor %}",
            "1true1.1hello"
        ),
        // https://github.com/Keats/tera/issues/301
        (
            "{% set start = 0 %}{% set end = start + 3 %}{% for i in range(start=start, end=end) %}{{ i }}{% endfor%}",
            "012"
        )
    ];

    for (input, expected) in inputs {
        println!("{:?} -> {:?}", input, expected);
        assert_eq!(render_template(input, &context).unwrap(), expected);
    }
}

#[test]
fn render_magic_variable_isnt_escaped() {
    let mut context = Context::new();
    context.insert("html", &"<html>");

    let result = render_template("{{ __tera_context }}", &context);

    assert_eq!(
        result.unwrap(),
        r#"{
  "html": "<html>"
}"#
        .to_owned()
    );
}

// https://github.com/Keats/tera/issues/185
#[test]
fn ok_many_variable_blocks() {
    let mut context = Context::new();
    context.insert("username", &"bob");

    let mut tpl = String::new();
    for _ in 0..200 {
        tpl.push_str("{{ username }}")
    }
    let mut expected = String::new();
    for _ in 0..200 {
        expected.push_str("bob")
    }
    assert_eq!(render_template(&tpl, &context).unwrap(), expected);
}

#[test]
fn can_set_variable_in_global_context_in_forloop() {
    let mut context = Context::new();
    context.insert("tags", &vec![1, 2, 3]);
    context.insert("default", &"default");

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
    context.insert("existing", "hello");

    let inputs = vec![
        (r#"{{ existing | default(value="hey") }}"#, "hello"),
        (r#"{{ val | default(value=1) }}"#, "1"),
        (r#"{{ val | default(value="hey") | capitalize }}"#, "Hey"),
        (r#"{{ obj.val | default(value="hey") | capitalize }}"#, "Hey"),
        (r#"{{ obj.val | default(value="hey") | capitalize }}"#, "Hey"),
        (r#"{{ not admin | default(value=false) }}"#, "true"),
        (r#"{{ not admin | default(value=true) }}"#, "false"),
    ];

    for (input, expected) in inputs {
        println!("{:?} -> {:?}", input, expected);
        assert_eq!(render_template(input, &context).unwrap(), expected);
    }
}

#[test]
fn filter_filter_works() {
    #[derive(Debug, Serialize)]
    struct Author {
        id: u8,
    };

    let mut context = Context::new();
    context.insert("authors", &vec![Author { id: 1 }, Author { id: 2 }, Author { id: 3 }]);

    let inputs =
        vec![(r#"{{ authors | filter(attribute="id", value=1) | first | get(key="id") }}"#, "1")];

    for (input, expected) in inputs {
        println!("{:?} -> {:?}", input, expected);
        assert_eq!(render_template(input, &context).unwrap(), expected);
    }
}

#[test]
fn can_do_string_concat() {
    let mut context = Context::new();
    context.insert("a_string", "hello");
    context.insert("another_string", "xXx");
    context.insert("an_int", &1);
    context.insert("a_float", &3.14);

    let inputs = vec![
        (r#"{{ "hello" ~ " world" }}"#, "hello world"),
        (r#"{{ "hello" ~ 1 }}"#, "hello1"),
        (r#"{{ "hello" ~ 3.14 }}"#, "hello3.14"),
        (r#"{{ 3.14 ~ "hello"}}"#, "3.14hello"),
        (r#"{{ "hello" ~ get_string() }}"#, "helloHello"),
        (r#"{{ get_string() ~ "hello" }}"#, "Hellohello"),
        (r#"{{ get_string() ~ 3.14 }}"#, "Hello3.14"),
        (r#"{{ a_string ~ " world" }}"#, "hello world"),
        (r#"{{ a_string ~ ' world ' ~ another_string }}"#, "hello world xXx"),
        (r#"{{ a_string ~ another_string }}"#, "helloxXx"),
        (r#"{{ a_string ~ an_int }}"#, "hello1"),
        (r#"{{ a_string ~ a_float }}"#, "hello3.14"),
    ];

    for (input, expected) in inputs {
        println!("{:?} -> {:?}", input, expected);
        assert_eq!(render_template(input, &context).unwrap(), expected);
    }
}

#[test]
fn can_fail_rendering_from_template() {
    let mut context = Context::new();
    context.insert("title", "hello");
    let res = render_template(
        r#"{{ throw(message="Error: " ~ title ~ " did not include a summary") }}"#,
        &context,
    );
    assert!(res.is_err());
    let err = res.unwrap_err();
    assert_eq!(err.iter().nth(1).unwrap().description(), "Error: hello did not include a summary");
}

#[test]
fn does_render_owned_for_loop_with_objects() {
    let mut context = Context::new();
    let data = json!([
            {"id": 1, "year": 2015},
            {"id": 2, "year": 2015},
            {"id": 3, "year": 2016},
            {"id": 4, "year": 2017},
            {"id": 5, "year": 2017},
            {"id": 6, "year": 2017},
            {"id": 7, "year": 2018},
            {"id": 8},
            {"id": 9, "year": null},
        ]);
    context.insert("something", &data);

    let tpl =
        r#"{% for year, things in something | group_by(attribute="year") %}{{year}},{% endfor %}"#;
    let expected = "2015,2016,2017,2018,";
    assert_eq!(render_template(tpl, &context).unwrap(), expected);
}

#[test]
fn render_magic_variable_gets_all_contexts() {
    let mut context = Context::new();
    context.insert("html", &"<html>");
    context.insert("num", &1);
    context.insert("i", &10);

    let result = render_template(
        "{% set some_val = 1 %}{% for i in range(start=0, end=1) %}{% set for_val = i %}{{ __tera_context }}{% endfor %}",
        &context
    );

    assert_eq!(
        result.unwrap(),
        r#"{
  "for_val": 0,
  "html": "<html>",
  "i": 0,
  "num": 1,
  "some_val": 1
}"#
        .to_owned()
    );
}

#[test]
fn render_magic_variable_macro_doesnt_leak() {
    let mut context = Context::new();
    context.insert("html", &"<html>");
    context.insert("num", &1);
    context.insert("i", &10);

    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("macros", "{% macro hello(arg=1) %}{{ __tera_context }}{% endmacro hello %}"),
        ("tpl", "{% import \"macros\" as macros %}{{macros::hello()}}"),
    ])
    .unwrap();
    let result = tera.render("tpl", &context);

    assert_eq!(
        result.unwrap(),
        r#"{
  "arg": 1
}"#
        .to_owned()
    );
}

// https://github.com/Keats/tera/issues/342
#[test]
fn redefining_loop_value_doesnt_break_loop() {
    let mut tera = Tera::default();
    tera.add_raw_template(
        "tpl",
        r#"
{%- set string = "abcdefghdijklm" | split(pat="d") -%}
{% for i in string -%}
    {%- set j = i ~ "lol" ~ " " -%}
    {{ j }}
{%- endfor -%}
        "#,
    )
    .unwrap();
    let context = Context::new();
    let result = tera.render("tpl", &context);

    assert_eq!(result.unwrap(), "abclol efghlol ijklmlol ");
}

#[test]
fn can_use_concat_to_push_to_array() {
    let mut tera = Tera::default();
    tera.add_raw_template(
        "tpl",
        r#"
{%- set ids = [] -%}
{% for i in range(end=5) -%}
{%- set_global ids = ids | concat(with=i) -%}
{%- endfor -%}
{{ids}}"#,
    )
    .unwrap();
    let context = Context::new();
    let result = tera.render("tpl", &context);

    assert_eq!(result.unwrap(), "[0, 1, 2, 3, 4]");
}
