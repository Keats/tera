use context::Context;
use nodes::Node;
use parser::{Parser};
use render::{Renderer};
use error::{TemplateError};


// This is the parsed equivalent of a html template file
// also handles rendering a template
// It really ties the library together
#[derive(Debug)]
pub struct Template {
    pub name: String, // filename
    ast: Node // will always be a ListNode
}

impl Template {
    pub fn new(name: &str, input: &str) -> Result<Template, TemplateError> {
        let parser = try!(Parser::new(&name, input));

        Ok(Template {
            name: name.to_owned(),
            ast: parser.root
        })
    }

    pub fn render(&self, context: Context) -> Result<String, TemplateError> {
        let mut renderer = Renderer::new(self.ast.clone(), context);

        renderer.render()
    }
}
