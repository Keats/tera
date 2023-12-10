#[macro_use]
extern crate rio_templates;
#[macro_use]
extern crate lazy_static;
extern crate serde_json;

use std::collections::HashMap;

use serde_json::value::{to_value, Value};
use std::error::Error;
use rio_templates::{Context, Result, Engine};

lazy_static! {
    pub static ref TEMPLATES: Engine = {
        let mut engine = match Engine::new("examples/basic/templates/**/*") {
            Ok(t) => t,
            Err(e) => {
                println!("Parsing error(s): {}", e);
                ::std::process::exit(1);
            }
        };
        engine.autoescape_on(vec!["html", ".sql"]);
        engine.register_filter("do_nothing", do_nothing_filter);
        engine
    };
}

pub fn do_nothing_filter(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("do_nothing_filter", "value", String, value);
    Ok(to_value(s).unwrap())
}

fn main() {
    let mut context = Context::new();
    context.insert("username", &"Bob");
    context.insert("numbers", &vec![1, 2, 3]);
    context.insert("show_all", &false);
    context.insert("bio", &"<script>alert('pwnd');</script>");

    // A one off template
    Engine::one_off("hello", &Context::new(), true).unwrap();

    match TEMPLATES.render("users/profile.html", &context) {
        Ok(s) => println!("{:?}", s),
        Err(e) => {
            println!("Error: {}", e);
            let mut cause = e.source();
            while let Some(e) = cause {
                println!("Reason: {}", e);
                cause = e.source();
            }
        }
    };
}
