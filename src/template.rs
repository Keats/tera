
use nodes::Node;
use parser::Parser;

// This is the parsed equivalent of a html template file
#[derive(Debug)]
pub struct Template {
    name: String, // filename
    ast: Node // will always be a ListNode
}

impl Template {
    pub fn new(name: &str, input: &str) -> Template {
        let parser = Parser::new(&name, input);

        Template {
            name: name.to_owned(),
            ast: parser.root
        }
    }
}
