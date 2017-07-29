extern crate tera;

use std::path::Path;

use tera::Tera;


#[test]
fn test_can_load_templates() {
    let tera = Tera::new("tests/templates/**/*").unwrap();

    assert!(tera.get_template("basic.html").is_ok());
}

#[test]
fn test_can_load_template_file() {
    let mut tera = Tera::default();
    tera.add_template_file(Path::new("tests/templates/basic.html"), None).unwrap();
    assert!(tera.get_template("tests/templates/basic.html").is_ok());
    tera.add_template_file(Path::new("tests/templates/basic.html"), Some("basic.html")).unwrap();
    assert!(tera.get_template("basic.html").is_ok());
}

#[test]
fn test_can_load_template_files() {
    let mut tera = Tera::default();
    tera.add_template_files(vec![
        (Path::new("tests/templates/basic.html"), None),
        (Path::new("tests/templates/basic.html"), Some("basic.html")),
    ]).unwrap();
    assert!(tera.get_template("tests/templates/basic.html").is_ok());
    assert!(tera.get_template("basic.html").is_ok());
}


#[test]
fn test_nested_object() {
    use tera::Context;

    let parent = {
        let mut p = Context::new();
        p.add("parent", &None::<Context>);
        p.add("label", &"Parent");
        p
    };

    let child = {
        let mut c = Context::new();
        c.add("parent", &parent);
        c.add("label", &"Child");
        c
    };

    let context_failing = {
        let mut c = Context::new();
        c.add("objects", &vec![child.clone()]);
        c
    };

    let context_passing = {
        let mut c = Context::new();
        c.add("obj", &child);
        c
    };
    
    let mut tera = Tera::default();

    tera.add_template_files(vec![
        (Path::new("tests/templates/macros.html"), Some("macros.html")),
        (Path::new("tests/templates/use_nested_macro_failing.html"), Some("use_nested_macro_failing.html")),
        (Path::new("tests/templates/use_nested_macro_passing.html"), Some("use_nested_macro_passing.html")),
    ]).unwrap();

    let out = tera.render("use_nested_macro_passing.html", &context_passing);

    assert!(out.is_ok());

    let out = tera.render("use_nested_macro_failing.html", &context_failing);

    assert!(out.is_ok());



}
