
#[macro_use] extern crate tera;
#[macro_use] extern crate lazy_static;

use tera::{Tera, Context};


lazy_static! {
    pub static ref TEMPLATES: Tera = {
        let mut tera = compile_templates!("examples/templates/**/*");
        tera.autoescape_on(vec!["html", ".sql"]);
        tera
    };
}

fn main() {
    let mut context = Context::new();
    context.add("username", &"Bob");
    context.add("numbers", &vec![1,2,3]);
    context.add("bio", &"<script>alert('pwnd');</script>");

    match TEMPLATES.render("users/profile.html", context) {
        Ok(s) => println!("{:?}", s),
        Err(e) => {
            println!("Error: {}", e);
            for e in e.iter().skip(1) {
                println!("Reason: {}", e);
            }
        }
    };
}
