use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};

use serde::Serialize;

use tera::{Context, Tera};

#[derive(Serialize)]
struct DataWrapper {
    i: usize,
    v: String,
}

impl DataWrapper {
    fn new(i: usize) -> DataWrapper {
        DataWrapper {
            i,
            v: "Meta
Before we get to the details, two important notes about the ownership system.
Rust has a focus on safety and speed. It accomplishes these goals through many ‘zero-cost abstractions’, which means that in Rust, abstractions cost as little as possible in order to make them work. The ownership system is a prime example of a zero cost abstraction. All of the analysis we’ll talk about in this guide is done at compile time. You do not pay any run-time cost for any of these features.
However, this system does have a certain cost: learning curve. Many new users to Rust experience something we like to call ‘fighting with the borrow checker’, where the Rust compiler refuses to compile a program that the author thinks is valid. This often happens because the programmer’s mental model of how ownership should work doesn’t match the actual rules that Rust implements. You probably will experience similar things at first. There is good news, however: more experienced Rust developers report that once they work with the rules of the ownership system for a period of time, they fight the borrow checker less and less.
With that in mind, let’s learn about borrowing.".into(),
        }
    }
}

#[derive(Serialize)]
struct BigObject {
    field_a: DataWrapper,
    field_b: DataWrapper,
    field_c: DataWrapper,
    field_d: DataWrapper,
    field_e: DataWrapper,
    field_f: DataWrapper,
}

impl BigObject {
    fn new(i: usize) -> BigObject {
        BigObject {
            field_a: DataWrapper::new(i),
            field_b: DataWrapper::new(i),
            field_c: DataWrapper::new(i),
            field_d: DataWrapper::new(i),
            field_e: DataWrapper::new(i),
            field_f: DataWrapper::new(i),
        }
    }
}

#[derive(Serialize)]
struct Team {
    name: String,
    score: u8,
}

static BIG_TABLE_TEMPLATE: &str = "
<table>
{% for row in table %}
<tr>{% for col in row %}<td>{{ col }}</td>{% endfor %}</tr>
{% endfor %}
</table>
";

static TEAMS_TEMPLATE: &str = r#"
<html>
  <head>
    <title>{{ year }}</title>
  </head>
  <body>
    <h1>CSL {{ year }}</h1>
    <ul>
    {% for team in teams %}
      <li class="{% if loop.index0 == 0 %}champion{% endif %}">
      <b>{{ team.name }}</b>: {{ team.score }}
      </li>
    {% endfor %}
    </ul>
  </body>
</html>
"#;

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("big-context", |b| {
        const NUM_OBJECTS: usize = 100;
        let mut objects = Vec::with_capacity(NUM_OBJECTS);
        for i in 0..NUM_OBJECTS {
            objects.push(BigObject::new(i));
        }

        let mut tera = Tera::default();
        tera.add_raw_templates(vec![(
            "big_loop.html",
            "
{%- for object in objects -%}
{{ object.field_a.i }}
{%- if object.field_a.i > 2 -%}
{%- break -%}
{%- endif -%}
{%- endfor -%}
",
        )])
        .unwrap();
        let mut context = Context::new();
        context.insert("objects", &objects);
        let rendering = tera.render("big_loop.html", &context).expect("Good render");
        assert_eq!(&rendering[..], "0123");
        b.iter(|| {
            // cloning as making the context is the bottleneck part
            tera.render("big_loop.html", &context.clone())
        });
    });

    c.bench_function("big-table", |b| {
        let length = 100;
        let mut table = Vec::with_capacity(length);
        for _ in 0..length {
            let mut inner = Vec::with_capacity(length);
            for i in 0..length {
                inner.push(i);
            }
            table.push(inner);
        }

        let mut tera = Tera::default();
        tera.add_raw_templates(vec![("big-table.html", BIG_TABLE_TEMPLATE)])
            .unwrap();
        let mut ctx = Context::new();
        ctx.insert("table", &table);

        b.iter(|| {
            let res = tera.render("big-table.html", &ctx);
            black_box(res).unwrap();
        })
    });

    c.bench_function("teams", |b| {
        let mut tera = Tera::default();
        tera.add_raw_templates(vec![("teams.html", TEAMS_TEMPLATE)])
            .unwrap();
        let mut ctx = Context::new();
        ctx.insert("year", &2015);
        ctx.insert(
            "teams",
            &vec![
                Team {
                    name: "Jiangsu".into(),
                    score: 43,
                },
                Team {
                    name: "Beijing".into(),
                    score: 27,
                },
                Team {
                    name: "Guangzhou".into(),
                    score: 22,
                },
                Team {
                    name: "Shandong".into(),
                    score: 12,
                },
            ],
        );

        b.iter(|| {
            let res = tera.render("teams.html", &ctx);
            black_box(res).unwrap();
        })
    });

    c.bench_function("realistic", |b| {
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

        b.iter(|| {
            let res = tera.render("page.html", &ctx);
            black_box(res).unwrap();
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
