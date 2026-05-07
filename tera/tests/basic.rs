use std::collections::HashMap;

use serde::Serialize;

use tera::{Context, Filter, Function, Kwargs, State, Tera};

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
        &Test {
            a: "hi".into(),
            b: "i_am_actually_b".into(),
            c: vec!["fred".into()],
        },
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

// https://github.com/Keats/tera/issues/754
#[test]
fn can_get_value_if_key_contains_period() {
    let mut context = Context::new();
    context.insert("name", "Mt. Robson Provincial Park");
    let mut map = HashMap::new();
    map.insert(
        "Mt. Robson Provincial Park".to_string(),
        "hello".to_string(),
    );
    context.insert("tag_info", &map);

    let res = Tera::one_off(r#"{{ tag_info[name] }}"#, &context, true);
    assert!(res.is_ok());
    let res = res.unwrap();
    assert_eq!(res, "hello");
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

#[test]
fn rendering_include_ok() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("world", r#"{% set a = "world" %}[Include => (a={{a}}, name={{ name }}, b={{ b }})]"#),
        ("hello", "<h1>Hello {% set b = 1 %} {% include \"world\" %} {% if a %}shouldfail{% endif %}name={{name}}</h1>"),
    ]).unwrap();
    let mut context = Context::new();
    context.insert("name", &"Bob");
    let out = tera.render("hello", &context).unwrap();
    assert_eq!(
        out,
        "<h1>Hello  [Include => (a=world, name=Bob, b=1)] name=Bob</h1>"
    );
}

#[test]
fn rendering_realistic_benchmark() {
    let items = vec!["Hello world"; 20];
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        (
            "index.html",
            std::fs::read_to_string("benches/realistic/index.html").unwrap(),
        ),
        (
            "components.html",
            std::fs::read_to_string("benches/realistic/components.html").unwrap(),
        ),
        (
            "page.html",
            std::fs::read_to_string("benches/realistic/page.html").unwrap(),
        ),
    ])
    .unwrap();
    let mut ctx = Context::new();
    ctx.insert("base_url", &"https://tera.netlify.app/");
    ctx.insert("description", &"Some description");
    ctx.insert("content", &"<a>Some HTML</a>");
    ctx.insert("title", &"Tera");
    ctx.insert("items", &items);
    ctx.insert("show_ad", &true);
    let out = tera.render("page.html", &ctx).unwrap();
    insta::assert_snapshot!(out);
}

#[test]
fn can_render_to_write() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![("hello", "Hello {{name}}")])
        .unwrap();
    let mut ctx = Context::new();
    ctx.insert("name", &"Bob");
    let mut out = Vec::new();
    tera.render_to("hello", &ctx, &mut out).unwrap();
    assert_eq!(String::from_utf8(out).unwrap(), "Hello Bob")
}

#[test]
fn render_str_can_use_locally_defined_component() {
    let tera = Tera::default();
    let src = r#"{% component greet(name="world") %}Hello {{name}}!{% endcomponent greet %}{{<greet name="Bob"/>}}"#;
    let out = tera.render_str(src, &Context::new(), false).unwrap();
    assert_eq!(out, "Hello Bob!");
}

struct BoldFilter;

impl Filter<&str, String> for BoldFilter {
    fn call(&self, value: &str, _kwargs: Kwargs, _state: &State) -> String {
        format!("<b>{value}</b>")
    }

    fn is_safe(&self) -> bool {
        true
    }
}

struct SafeHtmlFunction;

impl Function<String> for SafeHtmlFunction {
    fn call(&self, _kwargs: Kwargs, _state: &State) -> String {
        "<em>hello</em>".to_string()
    }

    fn is_safe(&self) -> bool {
        true
    }
}

#[test]
fn filter_is_safe_skips_autoescape() {
    let mut tera = Tera::default();
    tera.autoescape_on(vec![".html"]);
    tera.register_filter("bold", BoldFilter);
    tera.add_raw_template("tpl.html", "{{ name | bold }}")
        .unwrap();

    let mut ctx = Context::new();
    ctx.insert("name", "world");
    let out = tera.render("tpl.html", &ctx).unwrap();
    assert_eq!(out, "<b>world</b>");
}

#[test]
fn function_is_safe_skips_autoescape() {
    let mut tera = Tera::default();
    tera.autoescape_on(vec![".html"]);
    tera.register_function("safe_html", SafeHtmlFunction);
    tera.add_raw_template("tpl.html", "{{ safe_html() }}")
        .unwrap();

    let out = tera.render("tpl.html", &Context::new()).unwrap();
    assert_eq!(out, "<em>hello</em>");
}

#[test]
fn filter_not_safe_gets_autoescaped() {
    let mut tera = Tera::default();
    tera.autoescape_on(vec![".html"]);
    tera.register_filter("wrap_b", |val: &str, _: Kwargs, _: &State| {
        format!("<b>{val}</b>")
    });
    tera.add_raw_template("tpl.html", "{{ name | wrap_b }}")
        .unwrap();

    let mut ctx = Context::new();
    ctx.insert("name", "world");
    let out = tera.render("tpl.html", &ctx).unwrap();
    assert!(out.contains("&lt;b&gt;"));
}
