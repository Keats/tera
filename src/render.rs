use serde::ser::Serialize;

use nodes::{Node};
use nodes::SpecificNode::*;
use context::Context;
use parser::Parser;


#[derive(Debug)]
struct Renderer {
    output: String,
    context: Context,
    parser: Parser,
}

impl Renderer {
    pub fn new(parser: Parser, context: Context) -> Renderer {
        Renderer {
            output: String::new(),
            parser: parser,
            context: context
        }
    }

    fn eval_math(&self, node: &Node) -> f64 {
        match node.specific {
            Identifier(ref s) => panic!("TODO"),
            Int(ref s) => *s as f64,
            Float(ref s) => *s as f64,
            Math { ref lhs, ref rhs, ref operator } => {
                let l = self.eval_math(lhs);
                let r = self.eval_math(rhs);
                match operator as &str {
                    "*" => l * r,
                    "/" => l / r,
                    "+" => l + r,
                    "-" => l - r,
                    _ => panic!("unexpected operator: {}", operator)
                }
            }
            _ => panic!("Unexpected node")
        }
    }

    // eval all the values in a  {{ }} block
    fn render_variable_block(&mut self, node: Node) {
        match node.specific {
            Identifier(ref s) => panic!("TODO"),
            Math { .. } => {
                let result = self.eval_math(&node);
                self.output.push_str(&result.to_string());
            }
            _ => panic!("Unexpected node in variable block: {}", node)
        }
    }

    pub fn render(&mut self) {
        for node in self.parser.root.get_children() {
            match node.specific {
                Text(ref s) => self.output.push_str(s),
                VariableBlock(s) => self.render_variable_block(*s),
                _ => panic!("woo")
            }
        }
    }
}

pub fn render_from_string<T: Serialize>(template: &str, data: &T) -> String {
    let context = Context::new(data);
    let parser = Parser::new("string", template);
    let mut renderer = Renderer::new(parser, context);
    renderer.render();

    renderer.output
}


#[cfg(test)]
mod tests {
    use super::{render_from_string};

    #[test]
    fn test_render_simple_string() {
        let result = render_from_string("<h1>Hello world</h1>", &"");
        assert_eq!(result, "<h1>Hello world</h1>".to_owned());
    }

    #[test]
    fn test_render_with_math() {
        let result = render_from_string("This is {{ 2000 + 16 }}.", &"");
        assert_eq!(result, "This is 2016.".to_owned());
    }
}
