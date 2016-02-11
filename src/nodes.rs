use std::fmt;


#[derive(PartialEq, Debug, Clone)]
pub enum SpecificNode {
    List(Vec<Box<Node>>),
    Text(String),
    VariableBlock(Box<Node>),
    Identifier(String),
    Int(i32),
    Float(f32),
    Bool(bool),
    Addition {lhs: Box<Node>, rhs: Box<Node>},
    Substraction {lhs: Box<Node>, rhs: Box<Node>},
    Multiplication {lhs: Box<Node>, rhs: Box<Node>},
    Division {lhs: Box<Node>, rhs: Box<Node>}
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

    // Only used by SpecificNode::List
    pub fn push(&mut self, specific: Box<Node>) {
        match self.specific {
            SpecificNode::List(ref mut l) => l.push(specific),
            _ => panic!("tried to push on a non list node")
        }
    }

    // Only used by SpecificNode::List
    pub fn pop(&mut self) -> Box<Node> {
        match self.specific {
            SpecificNode::List(ref mut l) => l.pop().unwrap(),
            _ => panic!("tried to pop on a non list node")
        }
    }

    // Only used by SpecificNode::List
    pub fn get_children(&mut self) -> Vec<Box<Node>> {
        match self.specific {
            SpecificNode::List(ref l) => l.clone(),
            _ => panic!("tried to get_children on a non list node")
        }
    }

    // Only used by SpecificNode::List
    pub fn len(&mut self) -> usize {
        match self.specific {
            SpecificNode::List(ref l) => l.len(),
            _ => panic!("tried to len() on a non list node")
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
            SpecificNode::Text(ref s) => write!(f, "<Text> {}", s),
            SpecificNode::VariableBlock(ref s) => write!(f, "{{ {} }}", s),
            SpecificNode::Identifier(ref s) => write!(f, "<{}>", s),
            SpecificNode::Int(ref s) => write!(f, "<{}>", s),
            SpecificNode::Float(ref s) => write!(f, "<{}>", s),
            SpecificNode::Bool(ref s) => write!(f, "<{}>", s),
            SpecificNode::Addition { ref lhs, ref rhs } => write!(f, "<{} + {}>", lhs, rhs),
            SpecificNode::Substraction { ref lhs, ref rhs } => write!(f, "<{} - {}>", lhs, rhs),
            SpecificNode::Multiplication { ref lhs, ref rhs } => write!(f, "<{} * {}>", lhs, rhs),
            SpecificNode::Division { ref lhs, ref rhs } => write!(f, "<{} / {}>", lhs, rhs),
        }
    }
}
