
#[macro_use] extern crate tera;
#[macro_use] extern crate lazy_static;
extern crate serde_json;

use tera::{Tera, ast};



lazy_static! {
    pub static ref TERA: Tera = compile_templates!("examples/templates/**/*");
}


fn main() {
    for node in TERA.get_template("users/profile.html").unwrap().ast.get_children() {
        match node {
            ast::Node::Extends(ref name) => println!("Extending {}", name),
            _ => println!("Another node")
        }
    }
}
