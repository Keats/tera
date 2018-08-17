extern crate tera;
extern crate serde_json;

use serde_json::{Value, to_value};

#[macro_use]
extern crate serde_derive;
use tera::{Context, Tera};

#[derive(Serialize)]
struct ManyFields {
    a: String,
    b: String,
    c: String,
    d: Vec<BigObject>,
    e: Vec<String>,
}

impl ManyFields {
    fn new() -> ManyFields {
        let mut d = Vec::new();
        for i in 0..500 {
            d.push(BigObject::new(i));
        }
        let mut e = Vec::new();
        for i in 0..100 {
            e.push(format!("This is String({})", i));
        }

        ManyFields {
            a: "A".into(),
            b: "B".into(),
            c: "C".into(),
            d,
            e,
        }
    }
}

#[derive(Serialize)]
struct DataWrapper {
    i: usize,
    v: String,
}

impl DataWrapper {
    fn new(i: usize) -> DataWrapper {
        DataWrapper { i, v: "Meta
Before we get to the details, two important notes about the ownership system.

Rust has a focus on safety and speed. It accomplishes these goals through many ‘zero-cost abstractions’, which means that in Rust, abstractions cost as little as possible in order to make them work. The ownership system is a prime example of a zero cost abstraction. All of the analysis we’ll talk about in this guide is done at compile time. You do not pay any run-time cost for any of these features.

However, this system does have a certain cost: learning curve. Many new users to Rust experience something we like to call ‘fighting with the borrow checker’, where the Rust compiler refuses to compile a program that the author thinks is valid. This often happens because the programmer’s mental model of how ownership should work doesn’t match the actual rules that Rust implements. You probably will experience similar things at first. There is good news, however: more experienced Rust developers report that once they work with the rules of the ownership system for a period of time, they fight the borrow checker less and less.

With that in mind, let’s learn about borrowing.".into() }
    }
}

#[derive(Serialize)]
struct BigObject {
    field_a: DataWrapper,
    field_b: DataWrapper,
    field_c: DataWrapper,
    field_d: DataWrapper,
    field_e: DataWrapper,
    field_f: DataWrapper,
}

impl BigObject {
    fn new(i: usize) -> BigObject {
        BigObject {
            field_a: DataWrapper::new(i),
            field_b: DataWrapper::new(i),
            field_c: DataWrapper::new(i),
            field_d: DataWrapper::new(i),
            field_e: DataWrapper::new(i),
            field_f: DataWrapper::new(i),
        }
    }
}


fn main() {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![(
        "no_loop.html",
        "
{{ many_fields.a }}
{{ many_fields.b }}
{{ many_fields.c }}
",
    )]).unwrap();
    let mut context = Context::new();
    context.add("many_fields", &ManyFields::new());

    let context = to_value(context).unwrap();
    let mut results = Vec::with_capacity(1000);
    for _i in 0..10 {
        results.push(tera.render("no_loop.html", &context).expect("Good render"));
    }
    //assert_eq!(&rendering[..], "\nA\nB\nC\n");

    println!("Done with {}", results.len());
}
