extern crate rio_templates;

use rio_templates::{Context, Engine};
use std::alloc::System;

#[global_allocator]
static GLOBAL: System = System;

static BIG_TABLE_TEMPLATE: &str = "<table>
{% for row in table %}
<tr>{% for col in row %}<td>{{ col }}</td>{% endfor %}</tr>
{% endfor %}
</table>";

fn main() {
    let size = 100;

    let mut table = Vec::with_capacity(size);
    for _ in 0..size {
        let mut inner = Vec::with_capacity(size);
        for i in 0..size {
            inner.push(i);
        }
        table.push(inner);
    }

    let mut engine = Engine::default();
    engine
        .add_raw_templates(vec![("big-table.html", BIG_TABLE_TEMPLATE)])
        .unwrap();
    let mut ctx = Context::new();
    ctx.insert("table", &table);

    let _ = engine.render("big-table.html", &ctx).unwrap();
    println!("Done!");
}
