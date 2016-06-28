/// Tests Tera with a variety of real templates
extern crate tera;
extern crate glob;

use std::collections::HashMap;
use std::io::prelude::*;
use std::fs::File;

use tera::{Tera, Context};
use glob::glob;

mod common;
use common::{Product, Review};


// Almost a copy paste of the Tera constructor
fn read_all_expected(dir: &str) -> HashMap<String, String> {
    let mut expected = HashMap::new();

    for entry in glob(dir).unwrap().filter_map(|e| e.ok()) {
        let path = entry.as_path();
        // We only care about actual files
        if path.is_file() {
            // We clean the filename by removing the dir given
            // to Tera so users don't have to prefix everytime
            let parent_dir = dir.split_at(dir.find("*").unwrap()).0;
            let filepath = path.to_string_lossy().replace(parent_dir, "");
            // we know the file exists so unwrap all the things
            let mut f = File::open(path).unwrap();
            let mut input = String::new();
            f.read_to_string(&mut input).unwrap();
            expected.insert(filepath, input);
        }
    }

    expected
}

fn assert_template_eq(tera: &Tera, tpl_name: &str, expected: String) {
    let mut context = Context::new();
    context.add("product", &Product::new());
    context.add("username", &"bob");
    context.add("friend_reviewed", &true);
    context.add("number_reviews", &2);
    context.add("show_more", &true);
    context.add("reviews", &vec![Review::new(), Review::new()]);
    let empty: Vec<Review> = Vec::new();
    context.add("empty", &empty);

    let rendered = tera.render(tpl_name, context).unwrap();
    if rendered != expected {
        println!("Template {:?} was rendered incorrectly", tpl_name);
        println!("Got: \n {:#?}", rendered);
        println!("Expected: \n {:#?}", expected);
        // Uncomment below to save ouput to html file since
        // we don't ignore whitespace right now it's a bit tricky to get
        // the exact \n and spacing
        let mut file = File::create("out.html").unwrap();
        file.write_all(rendered.as_bytes()).unwrap();
        assert!(false);
    }
}

#[test]
fn test_valid_templates() {
    let tera = Tera::new("tests/templates/**/*");
    let expected = read_all_expected("tests/expected/**/*");

    for tpl in vec![
        "basic.html",
        "comment.html",
        "comment_alignment.html",
        "variables.html",
        "conditions.html",
        "loops.html",
        "empty_loop.html",
        "basic_inheritance.html",
    ] {
        assert_template_eq(
            &tera,
            tpl,
            expected.get(tpl).unwrap().clone(),
        );
    }
}

