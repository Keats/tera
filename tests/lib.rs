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
