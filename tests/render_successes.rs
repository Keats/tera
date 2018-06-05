extern crate tera;
#[macro_use]
extern crate serde_derive;

use tera::{Context, Result, Tera};

mod common;
use common::{Product, Review};

#[derive(Debug, Serialize)]
pub struct Point {
    pub x: i32,
    pub y: i32
}

fn render_tpl(template: &str) -> Result<String> {
    let mut context = Context::new();
    context.add("product", &Product::new());
    context.add("username", &"bob");
    context.add("friend_reviewed", &true);
    context.add("number_reviews", &2);
    context.add("show_more", &true);
    context.add("reviews", &vec![Review::new(), Review::new()]);
    context.add("points", &vec![
        Point { x: 1, y: 0},
        Point { x: 0, y: 1},
        Point { x: -1, y: 0},
        Point { x: 0, y: -1},
        Point { x: 1, y: 0},        
    ]);

    Tera::one_off(template, &context, true)
}

#[test]
fn test_json_pointer() {
    assert_eq!(render_tpl("{{ product.name }}").unwrap(), "Moto G")
}

#[test]
fn test_simple_for_loop() {
    assert_eq!(
        dark_matter(
            &render_tpl(
                "
{% for review in reviews %}
{{ review.title }}:
{% endfor %}
    ",
            ).unwrap()
        ),
        "Myreview:Myreview:"
    );
}

#[test]
fn test_simple_nested_for_loop() {
    let result = render_tpl(
        "
{% for r1 in reviews %}
    {% for r2 in reviews %}
{{ r2.title }}:{{ r1.title }}
    {% endfor %}
{% endfor %}
    ",
    ).unwrap();

    assert_eq!(
        dark_matter(&result),
        "Myreview:MyreviewMyreview:MyreviewMyreview:MyreviewMyreview:Myreview"
    );
}

#[test]
fn test_simple_key_val_for_loop() {
    assert_eq!(
        dark_matter(
            &render_tpl(
                "
{% for key, val in product %}
{{ key }} -> {{ val }}:
{% endfor %}
    ",
            ).unwrap()
        ),
        "manufacturer->Motorala:name->MotoG:price->100:summary->Aphone:"
    );
}

#[test]
fn test_simple_assignment() {
    let result = render_tpl("
{% set all_reviews = reviews %}
{% for a_review in all_reviews %}
{{ a_review.title }}:
{% endfor %}
    ").unwrap();

    assert_eq!(dark_matter(&result), "Myreview:Myreview:");
}

#[test]
fn test_nested_for_var_access() {
    let result = render_tpl(
        "
{%- for p in points -%}
{%- set x = p -%}
    {%- for p in points -%}
    {%- set x = p %}
    inner ({{ p.x }}, {{ p.y }})
    {%- endfor %}
outer ({{ p.x }}, {{ p.y }})
{%- endfor -%}
    ",
    ).unwrap();

    assert_eq!(&result, "
    inner (1, 0)
    inner (0, 1)
    inner (-1, 0)
    inner (0, -1)
    inner (1, 0)
outer (1, 0)
    inner (1, 0)
    inner (0, 1)
    inner (-1, 0)
    inner (0, -1)
    inner (1, 0)
outer (0, 1)
    inner (1, 0)
    inner (0, 1)
    inner (-1, 0)
    inner (0, -1)
    inner (1, 0)
outer (-1, 0)
    inner (1, 0)
    inner (0, 1)
    inner (-1, 0)
    inner (0, -1)
    inner (1, 0)
outer (0, -1)
    inner (1, 0)
    inner (0, 1)
    inner (-1, 0)
    inner (0, -1)
    inner (1, 0)
outer (1, 0)");
}

#[inline]
fn dark_matter(s: &str) -> String {
    s.replace("\n", "").replace(" ", "")
}
