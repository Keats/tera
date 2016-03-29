#![feature(custom_derive, plugin)]
#![plugin(serde_macros)]

/// Tests Tera with a variety of real templates
extern crate serde;
extern crate serde_json;
extern crate tera;
extern crate glob;

use std::collections::HashMap;
use std::io::prelude::*;
use std::fs::File;

use tera::{Tera, Template, Context};
use glob::glob;



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

fn assert_template_eq(template: &Template, expected: String, all_templates: HashMap<String, Template>) {
    let mut context = Context::new();
    context.add("product", &Product::new());
    context.add("username", &"bob");
    context.add("friend_reviewed", &true);
    context.add("number_reviews", &2);
    context.add("show_more", &true);
    context.add("reviews", &vec![Review::new(), Review::new()]);

    let rendered = template.render(context, all_templates);
    if rendered != expected {
        println!("Template {:?} was rendered incorrectly", template.name);
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
        "basic.html", "variables.html", "conditions.html", "loops.html",
        "basic_inheritance.html"
    ] {
        assert_template_eq(
            tera.get_template(tpl).unwrap(),
            expected.get(tpl).unwrap().clone(),
            tera.templates.clone()
        );
    }
}


// Loads a file and parse it
fn assert_fail_parsing(filename: &str, path: &str) {
    let mut f = File::open(path).unwrap();
    let mut input = String::new();
    f.read_to_string(&mut input).unwrap();
    // should panic
    Template::new(filename, &input);
}

#[should_panic(expected = "Block `hello` is duplicated in template `duplicate`")]
#[test]
fn test_error_parser_duplicate_block() {
    assert_fail_parsing("duplicate", "tests/failures/duplicate_block.html");
}

#[should_panic(expected = "Found endblock `goodbye` while we were hoping for `hello` at line 3 of template `wrong_endblock`")]
#[test]
fn test_error_parser_wrong_endblock() {
    assert_fail_parsing("wrong_endblock", "tests/failures/wrong_endblock.html");
}

#[should_panic(expected = "Missing endblock name at line 3 of template `missing_name`. It should be `hello`.")]
#[test]
fn test_error_parser_missing_endblock_name() {
    assert_fail_parsing("missing_name", "tests/failures/missing_endblock_name.html");
}

#[should_panic(expected = "{% extends %} tag need to be the first thing in a template. It is not the case in `extends`")]
#[test]
fn test_error_parser_extends_not_at_beginning() {
    assert_fail_parsing("extends", "tests/failures/invalid_extends.html");
}

#[should_panic(expected = "Found a elif in a Else block at line 3 of template `elif`, which is impossible.")]
#[test]
fn test_error_parser_invalid_elif() {
    assert_fail_parsing("elif", "tests/failures/invalid_elif.html");
}

#[should_panic(expected = "Found a else in a Else block at line 3 of template `else`, which is impossible.")]
#[test]
fn test_error_parser_invalid_else() {
    assert_fail_parsing("else", "tests/failures/invalid_else.html");
}

#[should_panic(expected = "Error: Found EOF while lexing spaces at line 1 of template unterminated")]
#[test]
fn test_error_parser_unterminated_variable_tag() {
    assert_fail_parsing("unterminated", "tests/failures/unterminated.html");
}
