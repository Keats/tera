extern crate tera;

use tera::Tera;

#[test]
fn test_can_load_templates() {
    let tera = Tera::new("tests/templates/**/*").unwrap();

    assert!(tera.get_template("basic.html").is_ok());
}
