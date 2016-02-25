use serde_json::value::{from_value};

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

    // TODO: clean up this, too ugly right now for the == and != nodes
    fn eval_condition(&self, node: &Node) -> bool {
        match node.specific {
            // Simple truthiness check
            Identifier(ref n) => {
                // TODO: no unwrap here
                let value = self.context.get(n).unwrap();
                value.is_truthy()
            },
            Logic { ref lhs, ref rhs, ref operator } => {
                match *operator {
                    TokenType::Or => {
                        return self.eval_condition(lhs) || self.eval_condition(rhs);
                    },
                    TokenType::And => {
                        return self.eval_condition(lhs) && self.eval_condition(rhs);
                    },
                    TokenType::GreaterOrEqual | TokenType::Greater
                    | TokenType::LowerOrEqual | TokenType::Lower => {
                        let l = self.eval_math(lhs);
                        let r = self.eval_math(rhs);
                        let result = match *operator {
                            TokenType::GreaterOrEqual => l >= r,
                            TokenType::Greater => l > r,
                            TokenType::LowerOrEqual => l <= r,
                            TokenType::Lower => l < r,
                            _ => unreachable!()
                        };
                        return result;
                    },
                    // This is quite different from the other operators
                    // TODO: clean this up, this is ugly
                    TokenType::Equal | TokenType::NotEqual => {
                        match lhs.specific {
                            Logic { .. } => {
                                // let l = self.eval_condition(lhs);
                                // TODO: rhs MUST be bool like
                                panic!("Unimplemented");
                            },
                            Identifier(ref n) => {
                                let l = self.context.get(n).unwrap();
                                // who knows what rhs is
                                // Here goes a whole new level of ugliness
                                match rhs.specific {
                                    Identifier(ref i) => {
                                        let r = self.context.get(i).unwrap();
                                        let result = match *operator {
                                            TokenType::Equal => l == r,
                                            TokenType::NotEqual => l != r,
                                            _ => unreachable!()
                                        };
                                        return result;
                                    },
                                    Int(r) => {
                                        // TODO: error handling
                                        let l2: i32 = from_value(l.clone()).unwrap();
                                        let result = match *operator {
                                            TokenType::Equal => l2 == r,
                                            TokenType::NotEqual => l2 != r,
                                            _ => unreachable!()
                                        };
                                        return result;
                                    },
                                    Float(r) => {
                                        let l2: f32 = from_value(l.clone()).unwrap();
                                        let result = match *operator {
                                            TokenType::Equal => l2 == r,
                                            TokenType::NotEqual => l2 != r,
                                            _ => unreachable!()
                                        };
                                        return result;
                                    },
                                    _ => unreachable!()
                                }
                            },
                            Int(n) => {
                                // rhs MUST be a number
                                let l = n as f32; // TODO: that's going to cause issues
                                let r = self.eval_math(rhs);
                                let result = match *operator {
                                    TokenType::Equal => l == r,
                                    TokenType::NotEqual => l != r,
                                    _ => unreachable!()
                                };
                                return result;
                            },
                            Float(l) => {
                                // rhs MUST be a number
                                let r = self.eval_math(rhs);
                                let result = match *operator {
                                    TokenType::Equal => l == r,
                                    TokenType::NotEqual => l != r,
                                    _ => unreachable!()
                                };
                                return result;
                            },
                            Math { .. } => {
                                // rhs MUST be a number
                                let l = self.eval_math(lhs);
                                let r = self.eval_math(rhs);
                                let result = match *operator {
                                    TokenType::Equal => l == r,
                                    TokenType::NotEqual => l != r,
                                    _ => unreachable!()
                                };
                                return result;
                            },
                            _ => unreachable!()
                        }
                    },
                    _ => unreachable!()
                }
                false
            },
            _ => panic!("Got in eval_condition {:?}", node)
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
        let mut skip_else = false;
        for node in condition_nodes {
            match node.specific {
                Conditional {ref condition, ref body } => {
                    if self.eval_condition(condition) {
                        skip_else = true;
                        self.render_node(*body.clone());
                    }
                },
                _ => unreachable!()
            }
        }

        if skip_else {
            return;
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
    use serde_json::value::{to_value};

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
    fn test_render_if_or_conditions() {
        let mut d = BTreeMap::new();
        d.insert("is_adult".to_owned(), to_value(&false));
        d.insert("age".to_owned(), to_value(&18));

        let result = Template::new("", "{% if is_adult || age + 1 > 18 %}Adult{% endif %}").render(&d);
        assert_eq!(result, "Adult".to_owned());
    }

    #[test]
    fn test_render_if_and_conditions_with_equality() {
        let mut d = BTreeMap::new();
        d.insert("is_adult".to_owned(), to_value(&true));
        d.insert("age".to_owned(), to_value(&18));

        let result = Template::new("", "{% if is_adult && age == 18 %}Adult{% endif %}").render(&d);
        assert_eq!(result, "Adult".to_owned());
    }
}
