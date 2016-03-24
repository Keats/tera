use std::fmt;

use lexer::TokenType;

// All the different types of node we can have in the ast
// We only have one math node instead of one for each operation
// to simplify the pattern matching
#[derive(PartialEq, Debug, Clone)]
pub enum SpecificNode {
    List(Vec<Box<Node>>),
    Text(String),
    VariableBlock(Box<Node>),
    Identifier(String),
    Int(i32),
    Float(f32),
    Bool(bool),
    Math {lhs: Box<Node>, rhs: Box<Node>, operator: TokenType},
    Logic {lhs: Box<Node>, rhs: Box<Node>, operator: TokenType},
    If {condition_nodes: Vec<Box<Node>>, else_node: Option<Box<Node>>},
    // represents a if/elif block and its body (body is a List)
    Conditional {condition: Box<Node>, body: Box<Node>},
    For {local: Box<Node>, array: Box<Node>, body: Box<Node>},
    Block {name: String, body: Box<Node>},
    Extends(String)
}

#[derive(PartialEq, Debug, Clone)]
pub struct Node {
    pub position: usize,
    pub specific: SpecificNode,
}

impl Node {
    pub fn new(position: usize, specific: SpecificNode) -> Node {
        Node {
            position: position,
            specific: specific
        }
    }

    pub fn push(&mut self, specific: Box<Node>) {
        match self.specific {
            SpecificNode::List(ref mut l) => l.push(specific),
            SpecificNode::If {ref mut condition_nodes, ..} => condition_nodes.push(specific),
            SpecificNode::Conditional {ref mut body, ..} | SpecificNode::For {ref mut body, ..}
            | SpecificNode::Block {ref mut body, ..} => {
                body.push(specific)
            },
            _ => panic!("tried to push on a non list node")
        }
    }

    // Only used by If
    pub fn push_to_else(&mut self, node: Box<Node>) {
        match self.specific {
            SpecificNode::If {ref mut else_node, ..} => {
                if let Some(e) = else_node.as_mut() { e.push(node); }
            },
            _ => panic!("tried to push_to_else on a non-if node")
        }
    }

    // Only used by SpecificNode::List
    pub fn pop(&mut self) -> Box<Node> {
        match self.specific {
            SpecificNode::List(ref mut l) => l.pop().unwrap(),
            _ => panic!("tried to pop on a non list node")
        }
    }

    // Only used by SpecificNode::List and SpecificNode::If
    pub fn get_children(&self) -> Vec<Box<Node>> {
        match self.specific {
            SpecificNode::List(ref l) => l.clone(),
            SpecificNode::If {ref condition_nodes, ..} => condition_nodes.clone(),
            _ => panic!("tried to get_children on a non-list/if node")
        }
    }

    // Only used by SpecificNode::List
    pub fn len(&self) -> usize {
        match self.specific {
            SpecificNode::List(ref l) => l.len(),
            _ => panic!("tried to len() on a non list node")
        }
    }

    // Only used by SpecificNode::List
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    // Only used by SpecificNode::If
    pub fn append_to_last_conditional(&mut self, node: Box<Node>) {
        match self.specific {
            SpecificNode::If {ref mut condition_nodes, ..} => {
                condition_nodes.last_mut().unwrap().push(node);
            },
            _ => panic!("tried to append_to_last_conditional on a non-if node")
        }
    }
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.specific {
            SpecificNode::List(ref l) => {
                let mut stringified = String::new();
                for n in l {
                    stringified.push_str(&format!("{}\n", n));
                }
                write!(f, "{}", stringified)
            },
            SpecificNode::Text(ref s) => write!(f, "{}", s),
            SpecificNode::VariableBlock(ref s) => write!(f, "{{ {} }}", s),
            SpecificNode::Identifier(ref s) => write!(f, "[{}]", s),
            SpecificNode::Int(ref s) => write!(f, "<{}>", s),
            SpecificNode::Float(ref s) => write!(f, "<{}>", s),
            SpecificNode::Bool(ref s) => write!(f, "<{}>", s),
            SpecificNode::Math { ref lhs, ref rhs , ref operator} => {
                write!(f, "<{} {} {}>", lhs, operator, rhs)
            },
            SpecificNode::Logic { ref lhs, ref rhs , ref operator} => {
                write!(f, "<{} {} {}>", lhs, operator, rhs)
            },
            SpecificNode::If { ref condition_nodes, ref else_node } => {
                let mut stringified = String::new();
                for (i, n) in condition_nodes.iter().enumerate() {
                    if i == 0 {
                        stringified.push_str(&format!("if: {}\n", n));
                    } else {
                        stringified.push_str(&format!("elif: {}\n", n));
                    }
                }
                if else_node.is_some() {
                    stringified.push_str(&format!("else: {}\n", else_node.clone().unwrap()));
                }
                write!(f, "{}", stringified)
            },
            SpecificNode::Conditional {ref condition, ref body } => {
                write!(f, "{} ? => {}", condition, body)
            },
            SpecificNode::For {ref local, ref array, ref body } => {
                write!(f, "for {} in {} ? => {}", local, array, body)
            },
            SpecificNode::Extends(ref s) => write!(f, "extends {}", s),
            SpecificNode::Block { ref name, ref body } => {
                write!(f, "block {} => {}", name, body)
            }
        }
    }
}
