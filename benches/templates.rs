#![feature(test)]
extern crate tera;
extern crate test;
#[macro_use]
extern crate serde_derive;

// Benches from https://github.com/djc/template-benchmarks-rs

use tera::{Context, Tera};

#[bench]
pub fn big_table(b: &mut test::Bencher) {
    // 100 instead of 50 in the original benchmark to make the time bigger
    let size = 100;
    let mut table = Vec::with_capacity(size);
    for _ in 0..size {
        let mut inner = Vec::with_capacity(size);
        for i in 0..size {
            inner.push(i);
        }
        table.push(inner);
    }

    let mut tera = Tera::default();
    tera.add_raw_templates(vec![("big-table.html", BIG_TABLE_TEMPLATE)]).unwrap();
    let mut ctx = Context::new();
    ctx.insert("table", &table);

    b.iter(|| tera.render("big-table.html", &ctx));
}

static BIG_TABLE_TEMPLATE: &str = "<table>
{% for row in table %}
<tr>{% for col in row %}<td>{{ col }}</td>{% endfor %}</tr>
{% endfor %}
</table>";

#[derive(Serialize)]
struct Team {
    name: String,
    score: u8,
}

#[bench]
pub fn teams(b: &mut test::Bencher) {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![("teams.html", TEAMS_TEMPLATE)]).unwrap();
    let mut ctx = Context::new();
    ctx.insert("year", &2015);
    ctx.insert(
        "teams",
        &vec![
            Team { name: "Jiangsu".into(), score: 43 },
            Team { name: "Beijing".into(), score: 27 },
            Team { name: "Guangzhou".into(), score: 22 },
            Team { name: "Shandong".into(), score: 12 },
        ],
    );

    b.iter(|| tera.render("teams.html", &ctx));
}

static TEAMS_TEMPLATE: &str = "<html>
  <head>
    <title>{{ year }}</title>
  </head>
  <body>
    <h1>CSL {{ year }}</h1>
    <ul>
    {% for team in teams %}
      <li class=\"{% if loop.first %}champion{% endif %}\">
      <b>{{team.name}}</b>: {{team.score}}
      </li>
    {% endfor %}
    </ul>
  </body>
</html>";
