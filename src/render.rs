use lexer::TokenType;
use nodes::Node;
use nodes::SpecificNode::*;
use context::{Context, JsonRender, JsonNumber, JsonTruthy};


#[derive(Debug)]
pub struct Renderer {
    output: String,
    context: Context,
    ast: Node,
}

impl Renderer {
    pub fn new(ast: Node, context: Context) -> Renderer {
        Renderer {
            output: String::new(),
            ast: ast,
            context: context
        }
    }

    fn eval_math(&self, node: &Node) -> f32 {
        match node.specific {
            Identifier(ref s) => {
                // TODO: no unwrap here
                let value = self.context.get(s).unwrap();
                value.to_number().unwrap()
            },
            Int(s) => s as f32,
            Float(s) => s,
            Math { ref lhs, ref rhs, ref operator } => {
                let l = self.eval_math(lhs);
                let r = self.eval_math(rhs);
                let mut result = match *operator {
                    TokenType::Multiply => l * r,
                    TokenType::Divide => l / r,
                    TokenType::Add => l + r,
                    TokenType::Substract => l - r,
                    _ => panic!("unexpected operator: {:?}", operator)
                };
                // TODO: fix properly
                // TODO: add tests for float maths arithmetics
                if result.fract() < 0.01 {
                    result = result.round();
                }
                result
            }
            _ => unreachable!()
        }
    }

    fn eval_condition(&self, node: &Node) -> bool {
        match node.specific {
            Identifier(ref n) => {
                // TODO: no unwrap here
                let value = self.context.get(n).unwrap();
                value.is_truthy()
            },
            _ => panic!("Got {:?}", node)
        }
    }

    // eval all the values in a  {{ }} block
    fn render_variable_block(&mut self, node: Node) {
        match node.specific {
            Identifier(ref s) => {
                // TODO: no unwrap here
                let value = self.context.get(s).unwrap();
                self.output.push_str(&value.render());
            },
            Math { .. } => {
                let result = self.eval_math(&node);
                self.output.push_str(&result.to_string());
            }
            _ => unreachable!()
        }
    }

    // evaluates conditions and render bodies accordingly
    fn render_if(&mut self, condition_nodes: Vec<Box<Node>>, else_node: Option<Box<Node>>) {
        for node in condition_nodes {
            match node.specific {
                Conditional {ref condition, ref body } => {
                    if self.eval_condition(condition) {
                        self.render_node(*body.clone());
                    }
                },
                _ => unreachable!()
            }
        }

        if let Some(e) = else_node {
            self.render_node(*e)
        };
    }

    pub fn render_node(&mut self, node: Node) {
        match node.specific {
            Text(ref s) => self.output.push_str(s),
            VariableBlock(s) => self.render_variable_block(*s),
            If {ref condition_nodes, ref else_node} => {
                self.render_if(condition_nodes.clone(), else_node.clone());
            },
            List(body) => {
                for n in body {
                    self.render_node(*n);
                }
            },
            _ => panic!("woo {:?}", node)
        }
    }

    pub fn render(&mut self) -> String {
        for node in self.ast.get_children() {
            self.render_node(*node);
        }

        self.output.clone()
    }
}

#[cfg(test)]
mod tests {
    use template::Template;
    use std::collections::BTreeMap;

    #[test]
    fn test_render_simple_string() {
        let result = Template::new("", "<h1>Hello world</h1>").render(&"");
        assert_eq!(result, "<h1>Hello world</h1>".to_owned());
    }

    #[test]
    fn test_render_math() {
        let result = Template::new("", "This is {{ 2000 + 16 }}.").render(&"");
        assert_eq!(result, "This is 2016.".to_owned());
    }

    #[test]
    fn test_render_basic_variable() {
        let mut d = BTreeMap::new();
        d.insert("name".to_owned(), "Vincent");

        let result = Template::new("", "My name is {{ name }}.").render(&d);
        assert_eq!(result, "My name is Vincent.".to_owned());
    }

    #[test]
    fn test_render_math_with_variable() {
        let mut d = BTreeMap::new();
        d.insert("vat_rate".to_owned(), 0.20);

        let result = Template::new("", "Vat: £{{ 100 * vat_rate }}.").render(&d);
        assert_eq!(result, "Vat: £20.".to_owned());
    }

    #[test]
    fn test_render_if_simple() {
        let mut d = BTreeMap::new();
        d.insert("is_admin".to_owned(), true);

        let result = Template::new("", "{% if is_admin %}Admin{% endif %}").render(&d);
        assert_eq!(result, "Admin".to_owned());
    }

    #[test]
    fn test_render_if_complex_conditions() {
        let mut d = BTreeMap::new();
        d.insert("is_admin".to_owned(), true);

        let result = Template::new("", "{% if is_admin %}Admin{% endif %}").render(&d);
        assert_eq!(result, "Admin".to_owned());
    }
}
