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


#[should_panic]
#[test]
fn test_can_only_parse_templates() {
    let mut tera = Tera::parse("examples/templates/**/*").unwrap();
    for tpl in tera.templates.values_mut() {
        tpl.name = format!("a-theme/templates/{}", tpl.name);
        if let Some(ref parent) = tpl.parent.clone() {
            tpl.parent = Some(format!("a-theme/templates/{}", parent));
        }
    }
    // Will panic here as we changed the parent and it won't be able
    // to build the inheritance chain in this case
    tera.build_inheritance_chains().unwrap();
}
