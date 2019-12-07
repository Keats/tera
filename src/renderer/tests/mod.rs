use serde_derive::Serialize;

mod basic;
mod errors;
mod inheritance;
mod macros;
mod square_brackets;
mod whitespace;

#[allow(dead_code)]
#[derive(Debug, Serialize)]
pub struct NestedObject {
    pub label: String,
    pub parent: Option<Box<NestedObject>>,
    pub numbers: Vec<usize>,
}

#[derive(Debug, Serialize)]
pub struct Review {
    title: String,
    paragraphs: Vec<String>,
}

impl Review {
    #[allow(dead_code)]
    pub fn new() -> Review {
        Review {
            title: "My review".to_owned(),
            paragraphs: vec!["A".to_owned(), "B".to_owned(), "C".to_owned()],
        }
    }
}
