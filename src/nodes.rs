use std::fmt;


#[derive(Debug)]
pub enum SpecificNode {
    List(Vec<Box<Node>>),
    Text(String),
    VariableBlock(Box<Node>),
    Identifier(String),
    Int(i32),
    Float(f32),
    Bool(bool),
}

#[derive(Debug)]
pub struct Node {
    position: usize,
    specific: SpecificNode,
}

impl Node {
    pub fn new(position: usize, specific: SpecificNode) -> Node {
        Node {
            position: position,
            specific: specific
        }
    }

    // Only used by SpecificNode::List
    pub fn push(&mut self, specific: Box<Node>) {
        match self.specific {
            SpecificNode::List(ref mut l) => l.push(specific),
            _ => panic!("tried to push on a non list node")
        }
    }
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.specific {
            SpecificNode::List(ref l) => {
                let mut stringified = String::new();
                for n in l {
                    stringified.push_str(&format!("{}", n));
                    stringified.push_str("\n");
                }
                write!(f, "{}", stringified)
            },
            SpecificNode::Text(ref s) => write!(f, "<Text> {}", s),
            SpecificNode::VariableBlock(ref s) => write!(f, "{{ {} }}", s),
            SpecificNode::Identifier(ref s) => write!(f, "<{}>", s),
            SpecificNode::Int(ref s) => write!(f, "<{}>", s),
            SpecificNode::Float(ref s) => write!(f, "<{}>", s),
            SpecificNode::Bool(ref s) => write!(f, "<{}>", s)
        }
    }
}
