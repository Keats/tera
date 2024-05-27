use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use lazy_static::lazy_static;
use serde_derive::Serialize;
use serde_json::{json, Value};

use crate::builtins::functions::Function;
use crate::context::Context;
use crate::errors::Result;
use crate::tera::Tera;

use super::Review;

fn render_template(content: &str, context: &Context) -> Result<String> {
    let mut tera = Tera::default();
    tera.add_raw_template("hello.html", content).unwrap();
    tera.register_function("get_number", |_: &HashMap<String, Value>| Ok(Value::Number(10.into())));
    tera.register_function("get_true", |_: &HashMap<String, Value>| Ok(Value::Bool(true)));
    tera.register_function("get_string", |_: &HashMap<String, Value>| {
        Ok(Value::String("Hello".to_string()))
    });

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
        ("{{ 3.18 }}", "3.18"),
        ("{{ \"hey\" }}", "hey"),
        (r#"{{ "{{ hey }}" }}"#, "{{ hey }}"),
        ("{{ true }}", "true"),
        ("{{ false }}", "false"),
        ("{{ false and true or true }}", "true"),
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
        ("{{ 1 / 0 }}", "NaN"),
        ("{{ true and 10 }}", "true"),
        ("{{ true and not 10 }}", "false"),
        ("{{ not true }}", "false"),
        ("{{ [1, 2, 3] }}", "[1, 2, 3]"),
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
    context.insert("with_newline", &"Animal Alphabets\nB is for Bee-Eater");

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
        ("{{ malicious | upper }}", "&lt;HTML&gt;"),
        ("{{ malicious | upper | safe }}", "<HTML>"),
        ("{{ malicious | safe | upper }}", "&lt;HTML&gt;"),
        ("{{ review | length }}", "2"),
        ("{{ review.paragraphs.1 }}", "B"),
        ("{{ numbers }}", "[1, 2, 3]"),
        ("{{ numbers.0 }}", "1"),
        ("{{ tuple_list.1.1 }}", "2"),
        ("{{ name and true }}", "true"),
        ("{{ name | length }}", "4"),
        ("{{ name is defined }}", "true"),
        ("{{ not name is defined }}", "false"),
        ("{{ name is not defined }}", "false"),
        ("{{ not name is not defined }}", "true"),
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
        ("{{ 4 + 40 / (2 + 8) / 4 }}", "5"),
        ("{{ ( ( 2 ) + ( 2 ) ) }}", "4"),
        ("{{ ( ( 4 / 1 ) + ( 2 / 1 ) ) }}", "6"),
        ("{{ ( ( 4 + 2 ) / ( 2 + 1 ) ) }}", "2"),
        // https://github.com/Keats/tera/issues/435
        (
            "{{ with_newline | replace(from='\n', to='<br>') | safe }}",
            "Animal Alphabets<br>B is for Bee-Eater",
        ),
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
    let mut hashmap = HashMap::new();
    hashmap.insert("a", 1);
    hashmap.insert("b", 10);
    hashmap.insert("john", 100);
    context.insert("object", &hashmap);
    context.insert("urls", &vec!["https://test"]);

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
        ("{{ name == 'john' }}", "true"),
        ("{{ name != 'john' }}", "false"),
        ("{{ name == 'john' | capitalize }}", "false"),
        ("{{ name != 'john' | capitalize }}", "true"),
        ("{{ 1 in numbers }}", "true"),
        ("{{ 1 not in numbers }}", "false"),
        ("{{ 40 not in numbers }}", "true"),
        ("{{ 'e' in 'hello' }}", "true"),
        ("{{ 'e' not in 'hello' }}", "false"),
        ("{{ 'x' not in 'hello' }}", "true"),
        ("{{ name in 'hello john' }}", "true"),
        ("{{ name not in 'hello john' }}", "false"),
        ("{{ name not in 'hello' }}", "true"),
        ("{{ name in ['bob', 2, 'john'] }}", "true"),
        ("{{ a in ['bob', 2, 'john'] }}", "true"),
        ("{{ \"https://test\" in [\"https://test\"] }}", "true"),
        ("{{ \"https://test\" in urls }}", "true"),
        ("{{ 'n' in name }}", "true"),
        ("{{ '<' in malicious }}", "true"),
        ("{{ 'a' in object }}", "true"),
        ("{{ name in object }}", "true"),
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
fn escaping_happens_at_the_end() {
    let inputs = vec![
        #[cfg(feature = "builtins")]
        ("{{ url | urlencode | safe }}", "https%3A//www.example.org/apples-%26-oranges/"),
        ("{{ '<html>' }}", "&lt;html&gt;"),
        ("{{ '<html>' | safe }}", "<html>"),
        ("{{ 'hello' | safe | replace(from='h', to='&') }}", "&amp;ello"),
        ("{{ 'hello' | replace(from='h', to='&') | safe }}", "&ello"),
    ];

    for (input, expected) in inputs {
        let mut context = Context::new();
        context.insert("url", "https://www.example.org/apples-&-oranges/");
        assert_eq!(render_template(input, &context).unwrap(), expected);
    }
}

#[test]
fn filter_args_are_not_escaped() {
    let mut context = Context::new();
    context.insert("my_var", &"hey");
    context.insert("to", &"&");
    let input = r#"{{ my_var | replace(from="h", to=to) }}"#;

    assert_eq!(render_template(input, &context).unwrap(), "&amp;ey");
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
fn render_include_array_tag() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("world", "world"),
        ("hello", "<h1>Hello {% include [\"custom/world\", \"world\"] %}</h1>"),
    ])
    .unwrap();
    let result = tera.render("hello", &Context::new()).unwrap();
    assert_eq!(result, "<h1>Hello world</h1>".to_owned());

    tera.add_raw_template("custom/world", "custom world").unwrap();
    let result = tera.render("hello", &Context::new()).unwrap();
    assert_eq!(result, "<h1>Hello custom world</h1>".to_owned());
}

#[test]
fn render_include_tag_missing() {
    let mut tera = Tera::default();
    tera.add_raw_template("hello", "<h1>Hello {% include \"world\" %}</h1>").unwrap();
    let result = tera.render("hello", &Context::new());
    assert!(result.is_err());

    let mut tera = Tera::default();
    tera.add_raw_template("hello", "<h1>Hello {% include \"world\" ignore missing %}</h1>")
        .unwrap();
    let result = tera.render("hello", &Context::new()).unwrap();
    assert_eq!(result, "<h1>Hello </h1>".to_owned());
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
        ("{% filter safe %}{% filter upper %}<Hello>{% endfilter %}{% endfilter%}", "<HELLO>")
    ];

    let context = Context::new();
    for (input, expected) in inputs {
        println!("{:?} -> {:?}", input, expected);
        assert_eq!(render_template(input, &context).unwrap(), expected);
    }
}

#[test]
fn render_tests() {
    let mut context = Context::new();
    context.insert("is_true", &true);
    context.insert("is_false", &false);
    context.insert("age", &18);
    context.insert("name", &"john");
    let mut map = HashMap::new();
    map.insert(0, 1);
    context.insert("map", &map);
    context.insert("numbers", &vec![1, 2, 3]);
    context.insert::<Option<usize>, _>("maybe", &None);

    let inputs = vec![
        ("{% if is_true is defined %}Admin{% endif %}", "Admin"),
        ("{% if hello is undefined %}Admin{% endif %}", "Admin"),
        ("{% if name is string %}Admin{% endif %}", "Admin"),
        ("{% if age is number %}Admin{% endif %}", "Admin"),
        ("{% if age is even %}Admin{% endif %}", "Admin"),
        ("{% if age is odd %}Admin{%else%}even{% endif %}", "even"),
        ("{% if age is divisibleby(2) %}Admin{% endif %}", "Admin"),
        ("{% if numbers is iterable %}Admin{% endif %}", "Admin"),
        ("{% if map is iterable %}Admin{% endif %}", "Admin"),
        ("{% if map is object %}Admin{% endif %}", "Admin"),
        ("{% if name is starting_with('j') %}Admin{% endif %}", "Admin"),
        ("{% if name is ending_with('n') %}Admin{% endif %}", "Admin"),
        ("{% if numbers is containing(2) %}Admin{% endif %}", "Admin"),
        ("{% if name is matching('^j.*') %}Admin{% endif %}", "Admin"),
        ("{% if maybe is defined %}Admin{% endif %}", "Admin"),
    ];

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
    context.insert("name", &"john");
    context.insert("empty_string", &"");
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
        // testing string conditions
        ("{% if 'true' %}a{% endif %}", "a"),
        ("{% if name %}a{% endif %}", "a"),
        ("{% if '' %}a{% endif %}", ""),
        ("{% if empty_string %}a{% endif %}", ""),
        ("{% if '' ~ name %}a{% endif %}", "a"),
        ("{% if '' ~ empty_string %}a{% endif %}", ""),
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
        // with in operator
        (
            "{% if 1 in numbers %}Admin{% elif 100 in numbers %}User{% else %}Hmm{% endif %}",
            "Admin",
        ),
        ("{% if 100 in numbers %}Admin{% elif 1 in numbers %}User{% else %}Hmm{% endif %}", "User"),
        ("{% if 'n' in name %}Admin{% else %}Hmm{% endif %}", "Admin"),
        // function in if
        ("{% if get_true() %}Truth{% endif %}", "Truth"),
        // Parentheses around logic expressions
        ("{% if age >= 18 and name == 'john' %}Truth{% endif %}", "Truth"),
        ("{% if (age >= 18) and (name == 'john') %}Truth{% endif %}", "Truth"),
        ("{% if (age >= 18) or (name == 'john') %}Truth{% endif %}", "Truth"),
        ("{% if (age < 18) or (name == 'john') %}Truth{% endif %}", "Truth"),
        ("{% if (age >= 18) or (name != 'john') %}Truth{% endif %}", "Truth"),
        ("{% if (age < 18) and (name != 'john') %}Truth{% endif %}", ""),
        ("{% if (age >= 18) and (name != 'john') %}Truth{% endif %}", ""),
        ("{% if (age >= 18 and name == 'john') %}Truth{% endif %}", "Truth"),
        ("{% if (age < 18 and name == 'john') %}Truth{% endif %}", ""),
        ("{% if (age >= 18 and name != 'john') %}Truth{% endif %}", ""),
        ("{% if age >= 18 or name == 'john' and is_false %}Truth{% endif %}", "Truth"),
        ("{% if (age >= 18 or name == 'john') and is_false %}Truth{% endif %}", ""),
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
        ),
        // https://github.com/Keats/tera/issues/395
        (
            "{% for a in [] %}{{a}}{% else %}hello{% endfor %}",
            "hello"
        ),
        (
            "{% for a in undefined_variable | default(value=[]) %}{{a}}{% else %}hello{% endfor %}",
            "hello"
        ),
        (
            "{% for a in [] %}{{a}}{% else %}{% if 1 == 2 %}A{% else %}B{% endif %}{% endfor %}",
            "B"
        ),
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
    let i: Option<usize> = None;
    context.insert("existing", "hello");
    context.insert("null", &i);

    let inputs = vec![
        (r#"{{ existing | default(value="hey") }}"#, "hello"),
        (r#"{{ val | default(value=1) }}"#, "1"),
        (r#"{{ val | default(value="hey") | capitalize }}"#, "Hey"),
        (r#"{{ obj.val | default(value="hey") | capitalize }}"#, "Hey"),
        (r#"{{ obj.val | default(value="hey") | capitalize }}"#, "Hey"),
        (r#"{{ not admin | default(value=false) }}"#, "true"),
        (r#"{{ not admin | default(value=true) }}"#, "false"),
        (r#"{{ null | default(value=true) }}"#, "true"),
        (r#"{{ null | default(value="hey") | capitalize }}"#, "Hey"),
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
    }

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
fn filter_on_array_literal_works() {
    let mut context = Context::new();
    let i: Option<usize> = None;
    context.insert("existing", "hello");
    context.insert("null", &i);

    let inputs = vec![
        (r#"{{ [1, 2, 3] | length }}"#, "3"),
        (r#"{% set a = [1, 2, 3] | length %}{{ a }}"#, "3"),
        (r#"{% for a in [1, 2, 3] | slice(start=1) %}{{ a }}{% endfor %}"#, "23"),
    ];

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
    context.insert("a_float", &3.18);

    let inputs = vec![
        (r#"{{ "hello" ~ " world" }}"#, "hello world"),
        (r#"{{ "hello" ~ 1 }}"#, "hello1"),
        (r#"{{ "hello" ~ 3.18 }}"#, "hello3.18"),
        (r#"{{ 3.18 ~ "hello"}}"#, "3.18hello"),
        (r#"{{ "hello" ~ get_string() }}"#, "helloHello"),
        (r#"{{ get_string() ~ "hello" }}"#, "Hellohello"),
        (r#"{{ get_string() ~ 3.18 }}"#, "Hello3.18"),
        (r#"{{ a_string ~ " world" }}"#, "hello world"),
        (r#"{{ a_string ~ ' world ' ~ another_string }}"#, "hello world xXx"),
        (r#"{{ a_string ~ another_string }}"#, "helloxXx"),
        (r#"{{ a_string ~ an_int }}"#, "hello1"),
        (r#"{{ a_string ~ a_float }}"#, "hello3.18"),
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

    let err = res.expect_err("This should always fail to render");
    let source = err.source().expect("Must have a source");
    assert_eq!(source.to_string(), "Function call 'throw' failed");

    let source = source.source().expect("Should have a nested error");
    assert_eq!(source.to_string(), "Error: hello did not include a summary");
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
fn does_render_owned_for_loop_with_objects_string_keys() {
    let mut context = Context::new();
    let data = json!([
        {"id": 1, "group": "a"},
        {"id": 2, "group": "b"},
        {"id": 3, "group": "c"},
        {"id": 4, "group": "a"},
        {"id": 5, "group": "b"},
        {"id": 6, "group": "c"},
        {"id": 7, "group": "a"},
        {"id": 8},
        {"id": 9, "year": null},
    ]);
    context.insert("something", &data);

    let tpl = r#"{% for group, things in something | group_by(attribute="group") %}{{group}},{% endfor %}"#;
    let expected = "a,b,c,";
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

struct Next(AtomicUsize);

impl Function for Next {
    fn call(&self, _args: &HashMap<String, Value>) -> Result<Value> {
        Ok(Value::Number(self.0.fetch_add(1, Ordering::Relaxed).into()))
    }
}

#[derive(Clone)]
struct SharedNext(Arc<Next>);

impl Function for SharedNext {
    fn call(&self, args: &HashMap<String, Value>) -> Result<Value> {
        self.0.call(args)
    }
}

lazy_static! {
    static ref NEXT_GLOBAL: SharedNext = SharedNext(Arc::new(Next(AtomicUsize::new(1))));
}

#[test]
fn stateful_global_fn() {
    fn make_tera() -> Tera {
        let mut tera = Tera::default();
        tera.add_raw_template(
            "fn.html",
            "<h1>{{ get_next() }}, {{ get_next_shared() }}, {{ get_next() }}...</h1>",
        )
        .unwrap();

        tera.register_function("get_next", Next(AtomicUsize::new(1)));
        tera.register_function("get_next_shared", NEXT_GLOBAL.clone());
        tera
    }

    assert_eq!(
        make_tera().render("fn.html", &Context::new()).unwrap(),
        "<h1>1, 1, 2...</h1>".to_owned()
    );
    assert_eq!(
        make_tera().render("fn.html", &Context::new()).unwrap(),
        "<h1>1, 2, 2...</h1>".to_owned()
    );
}

// https://github.com/Keats/tera/issues/373
#[test]
fn split_on_context_value() {
    let mut tera = Tera::default();
    tera.add_raw_template("split.html", r#"{{ body | split(pat="\n") }}"#).unwrap();
    let mut context = Context::new();
    context.insert("body", "multi\nple\nlines");
    let res = tera.render("split.html", &context);
    assert_eq!(res.unwrap(), "[multi, ple, lines]");
}

// https://github.com/Keats/tera/issues/422
#[test]
fn default_filter_works_in_condition() {
    let mut tera = Tera::default();
    tera.add_raw_template("test.html", r#"{% if frobnicate|default(value=True) %}here{% endif %}"#)
        .unwrap();
    let res = tera.render("test.html", &Context::new());
    assert_eq!(res.unwrap(), "here");
}

#[test]
fn safe_filter_works() {
    struct Safe;
    impl crate::Filter for Safe {
        fn filter(&self, value: &Value, _args: &HashMap<String, Value>) -> Result<Value> {
            Ok(Value::String(format!("<div>{}</div>", value.as_str().unwrap())))
        }

        fn is_safe(&self) -> bool {
            true
        }
    }

    let mut tera = Tera::default();
    tera.register_filter("safe_filter", Safe);
    tera.add_raw_template("test.html", r#"{{ "Hello" | safe_filter }}"#).unwrap();

    let res = tera.render("test.html", &Context::new());
    assert_eq!(res.unwrap(), "<div>Hello</div>");
}

#[test]
fn safe_function_works() {
    struct Safe;
    impl crate::Function for Safe {
        fn call(&self, _args: &HashMap<String, Value>) -> Result<Value> {
            Ok(Value::String("<div>Hello</div>".to_owned()))
        }

        fn is_safe(&self) -> bool {
            true
        }
    }

    let mut tera = Tera::default();
    tera.register_function("safe_function", Safe);
    tera.add_raw_template("test.html", "{{ safe_function() }}").unwrap();

    let res = tera.render("test.html", &Context::new());
    assert_eq!(res.unwrap(), "<div>Hello</div>");
}
