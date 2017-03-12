extern crate tera;

use tera::Tera;

#[test]
fn test_full_reload_with_glob() {
    let mut tera = Tera::new("tests/templates/**/*").unwrap();
    tera.full_reload().unwrap();

    assert!(tera.get_template("basic.html").is_ok());
}

#[test]
fn test_full_reload_with_glob_after_extending() {
    let mut tera = Tera::new("tests/templates/**/*").unwrap();
    let mut framework_tera = Tera::default();
    framework_tera.add_raw_templates(vec![
        ("one", "FRAMEWORK"),
        ("four", "Framework X"),
    ]).unwrap();
    tera.extend(&framework_tera).unwrap();
    tera.full_reload().unwrap();

    assert!(tera.get_template("basic.html").is_ok());
    assert!(tera.get_template("one").is_ok());
}
