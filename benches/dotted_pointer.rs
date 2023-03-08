#![feature(test)]
extern crate tera;
extern crate test;

use tera::Value;

fn deep_object() -> Value {
    let data = r#"{
                    "foo": {
                        "bar": {
                            "goo": {
                                "moo": {
                                    "cows": [
                                        {
                                            "name": "betsy",
                                            "age" : 2,
                                            "temperament": "calm"
                                        },
                                        {
                                            "name": "elsie",
                                            "age": 3,
                                            "temperament": "calm"
                                        },
                                        {
                                            "name": "veal",
                                            "age": 1,
                                            "temperament": "ornery"
                                        }
                                    ]
                                }
                            }
                        },
                        "http://example.com/": {
                            "goo": {
                                "moo": {
                                    "cows": [
                                        {
                                            "name": "betsy",
                                            "age" : 2,
                                            "temperament": "calm"
                                        },
                                        {
                                            "name": "elsie",
                                            "age": 3,
                                            "temperament": "calm"
                                        },
                                        {
                                            "name": "veal",
                                            "age": 1,
                                            "temperament": "ornery"
                                        }
                                    ]
                                }
                            }
                        }
                    }
                  }"#;

    serde_json::from_str(data).unwrap()
}

#[bench]
fn bench_get_dotted_pointer(b: &mut test::Bencher) {
    let value = deep_object();
    b.iter(|| tera::dotted_pointer(&value, "foo.bar.goo"))
}

#[bench]
fn bench_get_dotted_pointer_with_map(b: &mut test::Bencher) {
    let value = deep_object();
    b.iter(|| tera::dotted_pointer(&value, "foo[\"http://example.com/\"].goo"))
}
