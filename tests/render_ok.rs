//#[macro_use]
//extern crate serde_derive;
//extern crate tera;
//extern crate glob;
//
//
//use std::io::prelude::*;
//use std::fs::File;
//use std::collections::BTreeMap;
//
//use tera::{Tera, Result, Context, GlobalFn, Value, to_value, from_value};
//
//mod common;
//use common::{Product, Review, NestedObject, read_file};
//
//
//fn make_url_for(urls: BTreeMap<String, String>) -> GlobalFn {
//    Box::new(move |args| -> Result<Value> {
//        match args.get("name") {
//            Some(val) => match from_value::<String>(val.clone()) {
//                Ok(v) =>  Ok(to_value(urls.get(&v).unwrap()).unwrap()),
//                Err(_) => Err("oops".into()),
//            },
//            None => Err("oops".into()),
//        }
//    })
//}
//
//fn assert_template_ok(path: &str, others: Vec<&str>) {
//    let mut tera = Tera::default();
//    tera.autoescape_on(vec!["html"]);
//    let mut urls = BTreeMap::new();
//    urls.insert("home".to_string(), "vincent.is".to_string());
//    tera.register_global_function("url_for", make_url_for(urls));
//
//    for p in others {
//        let base = p.to_string();
//        let split = base.split('/').collect::<Vec<&str>>();
//        let name = split.last().unwrap();
//        tera.add_raw_template(name, &read_file(&base)).unwrap();
//    }
//    tera.add_raw_template("tpl.html", &read_file(path)).unwrap();
//    let expected = read_file(&path.replace("templates", "expected"));
//
//    let mut context = Context::new();
//    context.add("product", &Product::new());
//    context.add("username", &"bob");
//    context.add("friend_reviewed", &true);
//    context.add("number_reviews", &2);
//    context.add("show_more", &true);
//    context.add("reviews", &vec![Review::new(), Review::new()]);
//    let mut map = BTreeMap::new();
//    map.insert("bob", "comment 1");
//    map.insert("jane", "comment 2");
//    context.add("comments", &map);
//    context.add("a_tuple", &(1, 2, 3));
//    context.add("an_array_of_tuple", &vec![(1, 2, 3), (1, 2, 3)]);
//    let empty: Vec<Review> = Vec::new();
//    context.add("empty", &empty);
//
//    let rendered = tera.render("tpl.html", &context).unwrap();
//    // replace to make tests pass in windows
//    if rendered.replace("\r\n", "\n") != expected.replace("\r\n", "\n") {
//        println!("Template {:?} was rendered incorrectly", path);
//        println!("Got: \n {:#?}", rendered);
//        println!("Expected: \n {:#?}", expected);
//        let mut file = File::create("out.html").unwrap();
//        file.write_all(rendered.as_bytes()).unwrap();
//        assert!(false);
//    }
//}






//
//// https://github.com/Keats/tera/issues/202
//#[test]
//fn test_recursive_macro_with_loops() {
//    let parent = NestedObject { label: "Parent".to_string(), parent: None, numbers: vec![1,2,3]};
//    let child = NestedObject { label: "Child".to_string(), parent: Some(Box::new(parent)), numbers: vec![1,2,3] };
//    let mut context = Context::new();
//    context.add("objects", &vec![child]);
//
//    let mut tera = Tera::default();
//
//    tera.add_template_files(vec![
//        ("tests/templates/macros.html", Some("macros.html")),
//        ("tests/templates/recursive_macro_in_forloop.html", Some("tpl")),
//    ]).unwrap();
//    let expected = read_file("tests/expected/recursive_macro_in_forloop.html");
//    let rendered = tera.render("tpl", &context).unwrap();
//
//    if rendered != expected {
//        println!("Template recursive_macro_in_forloop was rendered incorrectly");
//        println!("Got: \n {:#?}", rendered);
//        println!("Expected: \n {:#?}", expected);
//        let mut file = File::create("out.html").unwrap();
//        file.write_all(rendered.as_bytes()).unwrap();
//        assert!(false);
//    }
//}
