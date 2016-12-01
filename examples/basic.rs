
extern crate tera;
#[macro_use] extern crate lazy_static;

use tera::{Tera, Context};


lazy_static! {
    static ref TEMPLATES: Tera = {
        let mut tera = Tera::new("templates/**/*");
        tera.autoescape_on(vec!["html", ".sql"]);
        tera
    };
}

fn main() {
    let mut context = Context::new();
    context.add("username", &"Bob");
    context.add("bio", &"<script>alert('pwnd');</script>");

    println!("{:?}", TEMPLATES.render("profile.html", context));
}
