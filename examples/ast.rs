
#[macro_use] extern crate tera;
#[macro_use] extern crate lazy_static;
extern crate serde_json;

use tera::{Tera, ast};



lazy_static! {
    pub static ref TERA: Tera = compile_templates!("examples/templates/**/*");
}

// Very basic fn to find identifier names
fn find_identifiers(node: ast::Node, names: &mut Vec<String>) {
    match node {
        ast::Node::Block {ref body, ..} => {
          for n in body.get_children() {
              find_identifiers(n, names);
          }
        },
        ast::Node::VariableBlock(n) => match *n {
            ast::Node::Identifier {ref name, ..} => names.push(name.clone()),
            _ => ()
        },
        _ => ()
    }
}

fn main() {
    let mut var_names = vec![];

    for node in TERA.get_template("users/profile.html").unwrap().ast.get_children() {
        find_identifiers(node, &mut var_names);
    }

    println!("Variables used: {:?}", var_names);
}
