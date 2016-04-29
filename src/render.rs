use std::f32::EPSILON;
use serde_json::value::{Value as Json, from_value, to_value};

use context::{Context, JsonRender, JsonNumber, JsonTruthy};
use lexer::TokenType;
use nodes::Node;
use nodes::SpecificNode::*;
use template::Template;
use errors::{TeraResult, field_not_found, not_a_number, not_an_array};


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

    pub fn get(&self) -> Option<&Json> {
        self.values.get(self.current)
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
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
    fn lookup_variable(&self, key: &str) -> TeraResult<Json> {
        // Look in the plain context if we aren't in a for loop
        if self.for_loops.is_empty() {
            return self.context.lookup(key).cloned().ok_or_else(|| field_not_found(key));
        }

        for for_loop in self.for_loops.iter().rev() {
            if key.starts_with(&for_loop.variable_name) {
                let value = match for_loop.get() {
                    Some(f) => f,
                    None => { return Ok(to_value(&"")); }
                };

                // might be a struct or some nested structure
                if key.contains('.') {
                    let new_key = key.split_terminator('.').skip(1).collect::<Vec<&str>>().join(".");
                    return value.lookup(&new_key).cloned().ok_or_else(|| field_not_found(key));
                } else {
                    return Ok(value.clone());
                }
            } else {
                match key {
                    "loop.index" => { return Ok(to_value(&(for_loop.current + 1))); },
                    "loop.index0" => { return Ok(to_value(&for_loop.current)); },
                    "loop.first" => { return Ok(to_value(&(for_loop.current == 0))); },
                    "loop.last" => { return Ok(to_value(&(for_loop.current == for_loop.len() - 1))); },
                    _ => ()
                };
            }
        }

        // dummy statement to satisfy the compiler
        // TODO: make it so that's not needed
        self.context.lookup(key).cloned().ok_or_else(|| field_not_found(key))
    }

    fn eval_math(&self, node: &Node) -> TeraResult<f32> {
        match node.specific {
            Identifier(ref s) => {
                let value = try!(self.lookup_variable(s));
                match value.to_number() {
                    Ok(v) =>  Ok(v),
                    Err(_) => Err(not_a_number(s))
                }
            },
            Int(s) => Ok(s as f32),
            Float(s) => Ok(s),
            Math { ref lhs, ref rhs, ref operator } => {
                let l = try!(self.eval_math(lhs));
                let r = try!(self.eval_math(rhs));
                let mut result = match *operator {
                    TokenType::Multiply => l * r,
                    TokenType::Divide => l / r,
                    TokenType::Add => l + r,
                    TokenType::Substract => l - r,
                    _ => unreachable!()
                };
                // TODO: fix properly
                // TODO: add tests for float maths arithmetics
                if result.fract() < 0.01 {
                    result = result.round();
                }
                Ok(result)
            }
            _ => unreachable!()
        }
    }

    // TODO: clean up this, too ugly right now for the == and != nodes
    fn eval_condition(&self, node: &Node) -> TeraResult<bool> {
        match node.specific {
            // Simple truthiness check
            Identifier(ref n) => {
                let value = try!(self.lookup_variable(n));
                Ok(value.is_truthy())
            },
            Logic { ref lhs, ref rhs, ref operator } => {
                match *operator {
                    TokenType::Or => {
                        let result = try!(self.eval_condition(lhs)) || try!(self.eval_condition(rhs));
                        return Ok(result);
                    },
                    TokenType::And => {
                        let result = try!(self.eval_condition(lhs)) && try!(self.eval_condition(rhs));
                        return Ok(result);
                    },
                    TokenType::GreaterOrEqual | TokenType::Greater
                    | TokenType::LowerOrEqual | TokenType::Lower => {
                        let l = try!(self.eval_math(lhs));
                        let r = try!(self.eval_math(rhs));
                        let result = match *operator {
                            TokenType::GreaterOrEqual => l >= r,
                            TokenType::Greater => l > r,
                            TokenType::LowerOrEqual => l <= r,
                            TokenType::Lower => l < r,
                            _ => unreachable!()
                        };
                        return Ok(result);
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
                                let l = try!(self.lookup_variable(n));
                                // who knows what rhs is
                                // Here goes a whole new level of ugliness
                                match rhs.specific {
                                    Identifier(ref i) => {
                                        let r = try!(self.lookup_variable(i));
                                        let result = match *operator {
                                            TokenType::Equal => l == r,
                                            TokenType::NotEqual => l != r,
                                            _ => unreachable!()
                                        };
                                        return Ok(result);
                                    },
                                    Int(r) => {
                                        let l2: i32 = match from_value(l.clone()) {
                                            Ok(k) => k,
                                            Err(_) => { return Err(not_a_number(n)); }
                                        };
                                        let result = match *operator {
                                            TokenType::Equal => l2 == r,
                                            TokenType::NotEqual => l2 != r,
                                            _ => unreachable!()
                                        };
                                        return Ok(result);
                                    },
                                    Float(r) => {
                                        let l2: f32 = match from_value(l.clone()) {
                                            Ok(k) => k,
                                            Err(_) => { return Err(not_a_number(n)); }
                                        };
                                        let result = match *operator {
                                            TokenType::Equal => (l2 - r).abs() < EPSILON,
                                            TokenType::NotEqual => (l2 - r).abs() > EPSILON,
                                            _ => unreachable!()
                                        };
                                        return Ok(result);
                                    },
                                    _ => unreachable!()
                                }
                            },
                            Int(n) => {
                                // rhs MUST be a number
                                let l = n as f32; // TODO: that's going to cause issues
                                let r = try!(self.eval_math(rhs));
                                let result = match *operator {
                                    TokenType::Equal => (l - r).abs() < EPSILON,
                                    TokenType::NotEqual => (l - r).abs() > EPSILON,
                                    _ => unreachable!()
                                };
                                return Ok(result);
                            },
                            Float(l) => {
                                // rhs MUST be a number
                                let r = try!(self.eval_math(rhs));
                                let result = match *operator {
                                    TokenType::Equal => (l - r).abs() < EPSILON,
                                    TokenType::NotEqual => (l - r).abs() > EPSILON,
                                    _ => unreachable!()
                                };
                                return Ok(result);
                            },
                            Math { .. } => {
                                // rhs MUST be a number
                                let l = try!(self.eval_math(lhs));
                                let r = try!(self.eval_math(rhs));
                                let result = match *operator {
                                    TokenType::Equal => (l - r).abs() < EPSILON,
                                    TokenType::NotEqual => (l - r).abs() > EPSILON,
                                    _ => unreachable!()
                                };
                                return Ok(result);
                            },
                            _ => unreachable!()
                        }
                    },
                    _ => unreachable!()
                }
                Ok(false)
            },
            _ => unreachable!()
        }
    }

    // eval all the values in a  {{ }} block
    fn render_variable_block(&mut self, node: Node) -> TeraResult<String>  {
        match node.specific {
            Identifier(ref s) => {
                let value = try!(self.lookup_variable(s));
                Ok(value.render())
            },
            Math { .. } => {
                let result = try!(self.eval_math(&node));
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
                    if try!(self.eval_condition(condition)) {
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

        let list = try!(self.lookup_variable(&array_name));

        if !list.is_array() {
            return Err(not_an_array(&array_name));
        }

        // Safe unwrap
        let deserialized = list.as_array().unwrap();
        let length = deserialized.len();
        self.for_loops.push(ForLoop::new(local_name, deserialized.clone()));
        let mut i = 0;
        let mut output = String::new();
        loop {
            output.push_str(&&try!(self.render_node(*body.clone())));
            // Safe unwrap
            self.for_loops.last_mut().unwrap().increment();
            if length == 0 || i == length - 1 {
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
            // unwrap is safe here as we checked the template exists beforehand
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
    fn test_render_comment() {
        let result = Template::new("", "<h1>Hello {# comment #} world</h1>").render(Context::new(), HashMap::new());
        assert_eq!(result.unwrap(), "<h1>Hello  world</h1>".to_owned());
    }

    #[test]
    fn test_render_nested_comment() {
        let result = Template::new("", "<h1>Hello {# comment {# nested #} world</h1>").render(Context::new(), HashMap::new());
        assert_eq!(result.unwrap(), "<h1>Hello  world</h1>".to_owned());
    }

    #[test]
    fn test_ignore_variable_in_comment() {
        let mut context = Context::new();
        context.add("name", &"Vincent");

        let result = Template::new("", "My name {# was {{ name }} #} is No One.").render(context, HashMap::new());
        assert_eq!(result.unwrap(), "My name  is No One.".to_owned());
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
