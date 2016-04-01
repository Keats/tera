use std::f32::EPSILON;
use serde_json::value::{Value as Json, from_value, to_value};

use context::{Context, JsonRender, JsonNumber, JsonTruthy};
use lexer::TokenType;
use nodes::Node;
use nodes::SpecificNode::*;
use template::Template;
use errors::TeraResult;


// we need to have some data in the renderer for when we are in a ForLoop
// For example, accessing the local variable would fail when
// looking it up in the context
#[derive(Debug)]
struct ForLoop {
    variable_name: String,
    current: usize,
    values: Vec<Json>
}
impl ForLoop {
    pub fn new(local: String, values: Vec<Json>) -> ForLoop {
        ForLoop {
            variable_name: local,
            current: 0,
            values: values
        }
    }

    pub fn increment(&mut self) {
        self.current += 1;
    }

    pub fn get(&self) -> &Json {
        self.values.get(self.current).unwrap()
    }

    pub fn len(&self) -> usize {
        return self.values.len();
    }
}

#[derive(Debug)]
pub struct Renderer<'a> {
    context: Json,
    current: &'a Template,
    parent: Option<&'a Template>,
    for_loops: Vec<ForLoop>
}

impl<'a> Renderer<'a> {
    pub fn new(current: &'a Template, parent: Option<&'a Template>, context: Context) -> Renderer<'a> {
        Renderer {
            current: current,
            parent: parent,
            context: context.as_json(),
            for_loops: vec![],
        }
    }

    // Lookup a variable name from the context and takes into
    // account for loops variables
    fn lookup_variable(&self, key: &str) -> Json {
        if self.for_loops.is_empty() {
            // TODO: no unwrap here
            return self.context.lookup(key).cloned().unwrap();
        }

        for for_loop in self.for_loops.iter().rev() {
            if key.starts_with(&for_loop.variable_name) {
                // TODO: no unwrap
                let value = for_loop.get();
                // might be a struct or some nested structure
                if key.contains('.') {
                    let new_key = key.split_terminator('.').skip(1).collect::<Vec<&str>>().join(".");
                    return value.lookup(&new_key).cloned().unwrap();
                } else {
                    return value.clone();
                }
            } else {
                match key {
                    "loop.index" => { return to_value(&(for_loop.current + 1)); },
                    "loop.index0" => { return to_value(&for_loop.current); },
                    "loop.first" => { return to_value(&(for_loop.current == 0)); },
                    "loop.last" => { return to_value(&(for_loop.current == for_loop.len() - 1)); },
                    _ => ()
                };
            }
        }

        // TODO: no unwrap here
        self.context.lookup(key).cloned().unwrap()
    }

    fn eval_math(&self, node: &Node) -> f32 {
        match node.specific {
            Identifier(ref s) => {
                // TODO: no unwrap here
                let value = self.context.lookup(s).unwrap();
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
                let value = self.context.lookup(n).unwrap();
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
                                let l = self.context.lookup(n).unwrap();
                                // who knows what rhs is
                                // Here goes a whole new level of ugliness
                                match rhs.specific {
                                    Identifier(ref i) => {
                                        let r = self.context.lookup(i).unwrap();
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
                                            TokenType::Equal => (l2 - r).abs() < EPSILON,
                                            TokenType::NotEqual => (l2 - r).abs() > EPSILON,
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
                                    TokenType::Equal => (l - r).abs() < EPSILON,
                                    TokenType::NotEqual => (l - r).abs() > EPSILON,
                                    _ => unreachable!()
                                };
                                return result;
                            },
                            Float(l) => {
                                // rhs MUST be a number
                                let r = self.eval_math(rhs);
                                let result = match *operator {
                                    TokenType::Equal => (l - r).abs() < EPSILON,
                                    TokenType::NotEqual => (l - r).abs() > EPSILON,
                                    _ => unreachable!()
                                };
                                return result;
                            },
                            Math { .. } => {
                                // rhs MUST be a number
                                let l = self.eval_math(lhs);
                                let r = self.eval_math(rhs);
                                let result = match *operator {
                                    TokenType::Equal => (l - r).abs() < EPSILON,
                                    TokenType::NotEqual => (l - r).abs() > EPSILON,
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
    fn render_variable_block(&mut self, node: Node) -> TeraResult<String>  {
        match node.specific {
            Identifier(ref s) => {
                // TODO: return error if value not found
                let value = self.lookup_variable(s);
                Ok(value.render())
            },
            Math { .. } => {
                let result = self.eval_math(&node);
                Ok(result.to_string())
            }
            _ => unreachable!()
        }
    }

    // evaluates conditions and render bodies accordingly
    fn render_if(&mut self, condition_nodes: Vec<Box<Node>>, else_node: Option<Box<Node>>) -> TeraResult<String> {
        let mut skip_else = false;
        let mut output = String::new();
        for node in condition_nodes {
            match node.specific {
                Conditional {ref condition, ref body } => {
                    if self.eval_condition(condition) {
                        skip_else = true;
                        output.push_str(&&try!(self.render_node(*body.clone())));
                    }
                },
                _ => unreachable!()
            }
        }

        if skip_else {
            return Ok(output);
        }

        if let Some(e) = else_node {
            output.push_str(&&try!(self.render_node(*e)));
        };

        Ok(output)
    }

    fn render_for(&mut self, local: Node, array: Node, body: Box<Node>) -> TeraResult<String> {
        let local_name = match local.specific {
            Identifier(s) => s,
            _ => unreachable!()
        };
        let array_name = match array.specific {
            Identifier(s) => s,
            _ => unreachable!()
        };

        // TODO: error if variable not found
        let list = self.lookup_variable(&array_name);

        if !list.is_array() {
            panic!("{:?} is not an array! can't iterate on it", list);
        }
        let deserialized = list.as_array().unwrap();
        let length = deserialized.len();
        self.for_loops.push(ForLoop::new(local_name, deserialized.clone()));
        let mut i = 0;
        let mut output = String::new();
        loop {
            output.push_str(&&try!(self.render_node(*body.clone())));
            self.for_loops.last_mut().unwrap().increment();
            if i == length - 1 {
                break;
            }
            i += 1;
        }
        // Trim right at the end of the loop.
        // Can't be done in the parser as it would remove all newlines between
        // loops
        output = output.trim_right().to_owned();

        Ok(output)
    }

    pub fn render_node(&mut self, node: Node) -> TeraResult<String> {
        match node.specific {
            Text(s) => Ok(s),
            VariableBlock(s) => self.render_variable_block(*s),
            If {ref condition_nodes, ref else_node} => {
                self.render_if(condition_nodes.clone(), else_node.clone())
            },
            List(body) => {
                let mut output = String::new();
                for n in body {
                    output.push_str(&&try!(self.render_node(*n)));
                }
                Ok(output)
            },
            For {local, array, body} => {
                self.render_for(*local, *array, body)
            },
            Block {ref name, ref body} => {
                match self.current.blocks.get(name) {
                    Some(b) => {
                        match b.specific {
                            Block {ref body, ..} => {
                                return self.render_node(*body.clone());
                            },
                            _ => unreachable!()
                        }
                    },
                    None => {
                        return self.render_node(*body.clone());
                    }
                };
            },
            _ => unreachable!()
        }
    }

    pub fn render(&mut self) -> TeraResult<String> {
        let children = if self.parent.is_none() {
            self.current.ast.get_children()
        } else {
            // Return err here instead of unwrap
            self.parent.unwrap().ast.get_children()
        };

        let mut output = String::new();
        for node in children {
            // TODO: not entirely sure why i need to && instead of &
            output.push_str(&&try!(self.render_node(*node)));
        }

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use template::Template;
    use context::Context;

    #[test]
    fn test_render_simple_string() {
        let result = Template::new("", "<h1>Hello world</h1>").render(Context::new(), HashMap::new());
        assert_eq!(result.unwrap(), "<h1>Hello world</h1>".to_owned());
    }

    #[test]
    fn test_render_math() {
        let result = Template::new("", "This is {{ 2000 + 16 }}.").render(Context::new(), HashMap::new());
        assert_eq!(result.unwrap(), "This is 2016.".to_owned());
    }

    #[test]
    fn test_render_basic_variable() {
        let mut context = Context::new();
        context.add("name", &"Vincent");

        let result = Template::new("", "My name is {{ name }}.").render(context, HashMap::new());
        assert_eq!(result.unwrap(), "My name is Vincent.".to_owned());
    }

    #[test]
    fn test_render_math_with_variable() {
        let mut context = Context::new();
        context.add("vat_rate", &0.20);

        let result = Template::new("", "Vat: £{{ 100 * vat_rate }}.").render(context, HashMap::new());
        assert_eq!(result.unwrap(), "Vat: £20.".to_owned());
    }

    #[test]
    fn test_render_if_simple() {
        let mut context = Context::new();
        context.add("is_admin", &true);

        let result = Template::new("", "{% if is_admin %}Admin{% endif %}").render(context, HashMap::new());
        assert_eq!(result.unwrap(), "Admin".to_owned());
    }

    #[test]
    fn test_render_if_or_conditions() {
        let mut context = Context::new();
        context.add("is_adult", &false);
        context.add("age", &18);

        let result = Template::new(
            "",
            "{% if is_adult || age + 1 > 18 %}Adult{% endif %}"
        ).render(context, HashMap::new());
        assert_eq!(result.unwrap(), "Adult".to_owned());
    }

    #[test]
    fn test_render_if_and_conditions_with_equality() {
        let mut context = Context::new();
        context.add("is_adult", &true);
        context.add("age", &18);

        let result = Template::new(
            "", "{% if is_adult && age == 18 %}Adult{% endif %}"
        ).render(context, HashMap::new());
        assert_eq!(result.unwrap(), "Adult".to_owned());
    }

    #[test]
    fn test_render_basic_for() {
        let mut context = Context::new();
        context.add("data", &vec![1,2,3]);

        let result = Template::new(
            "", "{% for i in data %}{{i}}{% endfor %}"
        ).render(context, HashMap::new());
        assert_eq!(result.unwrap(), "123".to_owned());
    }

    #[test]
    fn test_render_loop_variables() {
        let mut context = Context::new();
        context.add("data", &vec![1,2,3]);

        let result = Template::new(
            "",
            "{% for i in data %}{{loop.index}}{{loop.index0}}{{loop.first}}{{loop.last}}{% endfor %}"
        ).render(context, HashMap::new());

        assert_eq!(result.unwrap(), "10truefalse21falsefalse32falsetrue".to_owned());
    }
}
