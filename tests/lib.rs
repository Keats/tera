/// Tests Tera with a variety of real templates
extern crate tera;

// use std::io::prelude::*;
// use std::fs::File;

use tera::Tera;

// fn read_expected(filename: &str) -> String {
//     let mut f = File::open("foo.txt").unwrap();
//     let mut s = String::new();
//     f.read_to_string(&mut s).unwrap();

//     s
// }


#[test]
fn test_templates() {
    let tera = Tera::new("./tests/templates/");
}
