use std::collections::HashMap;

use context::Context;
use nodes::Node;
use parser::Parser;
use render::Renderer;
use errors::{TeraResult, template_not_found};


// This is the parsed equivalent of a html template file
// also handles rendering a template
// It really ties the library together
#[derive(Debug, Clone)]
pub struct Template {
    pub name: String, // filename
    pub ast: Node, // will always be a ListNode
    pub blocks: HashMap<String, Node>,
    parent: Option<String>
}

impl Template {
    pub fn new(name: &str, input: &str) -> Template {
        let parser = Parser::new(&name, input);

        Template {
            name: name.to_owned(),
            ast: parser.root,
            blocks: parser.blocks,
            parent: parser.extends
        }
    }

    pub fn render(&self, context: Context, templates: HashMap<String, Template>) -> TeraResult<String> {
        let parent = match self.parent {
            Some(ref n) => match templates.get(n) {
                Some(p) => Some(p),
                None => { return Err(template_not_found(n)); }
            },
            None => None
        };

        // TODO: return a TemplateResult if there is a TemplateNotFound
        let mut renderer = Renderer::new(self, parent, context);

        renderer.render()
    }
}
