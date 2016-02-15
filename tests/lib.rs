/// Tests Tera with a variety of real templates
extern crate tera;
extern crate walkdir;

use std::io::prelude::*;
use std::fs::File;
use std::collections::BTreeMap;

use tera::{Tera, Template};
use walkdir::WalkDir;



// Almost a copy paste of the Tera constructor
fn read_all_expected(dir: &str) -> BTreeMap<String, String> {
    let mut expected = BTreeMap::new();

    for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        // We only care about actual files
        if path.is_file() {
            // We clean the filename by removing the dir given
            // to Tera so users don't have to prefix everytime
            let filepath = path.to_string_lossy().replace(dir, "");
            // we know the file exists so unwrap all the things
            let mut f = File::open(path).unwrap();
            let mut input = String::new();
            f.read_to_string(&mut input).unwrap();
            expected.insert(filepath.to_owned(), input);
        }
    }

    expected
}

fn assert_template_eq(template: &Template, expected: String) {
    let rendered = template.render(&"");
    if rendered != expected {
        println!("Template {:?} was rendered incorrectly", template.name);
        println!("Got {:#?}", rendered);
        println!("Expected {:#?}", expected);
        assert!(false);
    }
}

#[test]
fn test_templates() {
    let tera = Tera::new("./tests/templates/");
    let expected = read_all_expected("./tests/expected/");

    assert_template_eq(
        tera.get_template("basic.html").unwrap(),
        expected.get("basic.html").unwrap().clone()
    );
}
