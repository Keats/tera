#![feature(custom_derive, plugin)]
#![plugin(serde_macros)]

/// Tests Tera with a variety of real templates
extern crate serde;
extern crate serde_json;
extern crate tera;
extern crate walkdir;

use std::io::prelude::*;
use std::fs::File;
use std::collections::BTreeMap;

use tera::{Tera, Template, Context};
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
            expected.insert(filepath, input);
        }
    }

    expected
}


#[derive(Debug, Serialize)]
struct Product {
    name: String,
    manufacturer: String,
    price: i32,
    summary: String
}
impl Product {
    pub fn new() -> Product {
        Product {
            name: "Moto G".to_owned(),
            manufacturer: "Motorala".to_owned(),
            summary: "A phone".to_owned(),
            price: 100
        }
    }
}

#[derive(Debug, Serialize)]
struct Review {
    title: String,
    paragraphs: Vec<String>
}
impl Review {
    pub fn new() -> Review {
        Review {
            title: "My review".to_owned(),
            paragraphs: vec![
                "A".to_owned(), "B".to_owned(), "C".to_owned()
            ]
        }
    }
}

fn assert_template_eq(template: &Template, expected: String) {
    let mut context = Context::new();
    context.add("product", &Product::new());
    context.add("username", &"bob");
    context.add("friend_reviewed", &true);
    context.add("number_reviews", &2);
    context.add("show_more", &true);
    context.add("reviews", &vec![Review::new(), Review::new()]);

    let rendered = template.render(context).unwrap();
    if rendered != expected {
        println!("Template {:?} was rendered incorrectly", template.name);
        println!("Got: \n {:#?}", rendered);
        println!("Expected: \n {:#?}", expected);
        // Uncomment below to save ouput to html file since
        // we don't ignore whitespace right now it's a bit tricky to get
        // the exact \n and spacing
        // let mut file = File::create("out.html").unwrap();
        // file.write_all(rendered.as_bytes()).unwrap();
        assert!(false);
    }
}

#[test]
fn test_templates() {
    let tera = Tera::new("./tests/templates/");
    let expected = read_all_expected("./tests/expected/");

    for tpl in vec![
        "basic.html", "variables.html", "conditions.html", "loops.html"
    ] {
        assert_template_eq(
            tera.get_template(tpl).unwrap(),
            expected.get(tpl).unwrap().clone()
        );
    }
}
