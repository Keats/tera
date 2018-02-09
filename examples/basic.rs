#[macro_use]
extern crate lazy_static;
extern crate serde_json;
#[macro_use]
extern crate tera;

use std::collections::HashMap;

use tera::{Context, Result, Tera};
use serde_json::value::{to_value, Value};

lazy_static! {
    pub static ref TEMPLATES: Tera = {
        let mut tera = compile_templates!("examples/templates/**/*");
        tera.autoescape_on(vec!["html", ".sql"]);
        tera.register_filter("do_nothing", do_nothing_filter);
        tera
    };
}

pub fn do_nothing_filter(value: Value, _: HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("do_nothing_filter", "value", String, value);
    Ok(to_value(&s).unwrap())
}

fn main() {
    let mut context = Context::new();
    context.add("username", &"Bob");
    context.add("numbers", &vec![1, 2, 3]);
    context.add("show_all", &false);
    context.add("bio", &"<script>alert('pwnd');</script>");

    // A one off template
    Tera::one_off("hello", &Context::new(), true).unwrap();

    match TEMPLATES.render("users/profile.html", &context) {
        Ok(s) => println!("{:?}", s),
        Err(e) => {
            println!("Error: {}", e);
            for e in e.iter().skip(1) {
                println!("Reason: {}", e);
            }
        }
    };
}
