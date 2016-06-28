extern crate serde;
extern crate serde_json;

#[derive(Debug)]
pub struct Product {
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
// Impl Serialize by hand so tests pass on stable and beta
impl serde::Serialize for Product {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: serde::Serializer
    {
        serializer.serialize_struct("Product", ProductMapVisitor {
            value: self,
            state: 0,
        })
    }
}

struct ProductMapVisitor<'a> {
    value: &'a Product,
    state: u8,
}

impl<'a> serde::ser::MapVisitor for ProductMapVisitor<'a> {
    fn visit<S>(&mut self, serializer: &mut S) -> Result<Option<()>, S::Error>
        where S: serde::Serializer
    {
        match self.state {
            0 => {
                self.state += 1;
                Ok(Some(try!(serializer.serialize_struct_elt("name", &self.value.name))))
            },
            1 => {
                self.state += 1;
                Ok(Some(try!(serializer.serialize_struct_elt("manufacturer", &self.value.manufacturer))))
            },
            2 => {
                self.state += 1;
                Ok(Some(try!(serializer.serialize_struct_elt("price", &self.value.price))))
            },
            3 => {
                self.state += 1;
                Ok(Some(try!(serializer.serialize_struct_elt("summary", &self.value.summary))))
            },
            _ => {
                Ok(None)
            }
        }
    }
}

#[derive(Debug)]
pub struct Review {
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
// Impl Serialize by hand so tests pass on stable and beta
impl serde::Serialize for Review {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: serde::Serializer
    {
        serializer.serialize_struct("Review", ReviewMapVisitor {
            value: self,
            state: 0,
        })
    }
}

struct ReviewMapVisitor<'a> {
    value: &'a Review,
    state: u8,
}

impl<'a> serde::ser::MapVisitor for ReviewMapVisitor<'a> {
    fn visit<S>(&mut self, serializer: &mut S) -> Result<Option<()>, S::Error>
        where S: serde::Serializer
    {
        match self.state {
            0 => {
                self.state += 1;
                Ok(Some(try!(serializer.serialize_struct_elt("title", &self.value.title))))
            },
            1 => {
                self.state += 1;
                Ok(Some(try!(serializer.serialize_struct_elt("paragraphs", &self.value.paragraphs))))
            },
            _ => {
                Ok(None)
            }
        }
    }
}
