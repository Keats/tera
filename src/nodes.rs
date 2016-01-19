use std::fmt;


#[derive(Copy, Clone, PartialEq, Debug)]
pub enum NodeKind {
    List, // Contains other nodes
    Text, // html
    VariableBlock, // A variable block
    Identifier,
    Int,
    Float,
    Bool
}

pub trait Node: ToString {
    fn get_kind(self) -> NodeKind;
    fn get_position(self) -> usize;
}


macro_rules! impl_node {
    ($n: ty) => {
        impl Node for $n {
            fn get_kind(self) -> NodeKind {
                self.kind
            }

            fn get_position(self) -> usize {
                self.position
            }
        }
    }
}


pub struct ListNode {
    kind: NodeKind,
    position: usize,
    pub nodes: Vec<Box<Node>>
}
impl_node!(ListNode);

impl fmt::Debug for ListNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "<{:?} ({})>", self.kind, self.position)
    }
}

impl ToString for ListNode {
    fn to_string(&self) -> String {
        // TODO: print nodes to_string
        // format!("<{:?}>", self.kind)
        let mut stringified = String::new();
        for node in &self.nodes {
            stringified.push_str(&node.to_string());
            stringified.push_str("\n");
        }

        stringified
    }
}

impl ListNode {
    pub fn new(position: usize) -> ListNode {
        ListNode {
            kind: NodeKind::List,
            position: position,
            nodes: vec![]
        }
    }

    pub fn append(&mut self, node: Box<Node>) {
        self.nodes.push(node);
    }
}

#[derive(Debug)]
pub struct TextNode {
    kind: NodeKind,
    position: usize,
    text: String
}
impl_node!(TextNode);

impl TextNode {
    pub fn new(position: usize, text: String) -> TextNode {
        TextNode {
            kind: NodeKind::Text,
            position: position,
            text: text
        }
    }
}

impl ToString for TextNode {
    fn to_string(&self) -> String {
        format!("<{:?}> {:?}", self.kind, self.text)
    }
}


pub struct VariableBlockNode {
    kind: NodeKind,
    position: usize,
    node: Box<Node>,
}
impl_node!(VariableBlockNode);

impl VariableBlockNode {
    pub fn new(position: usize, node: Box<Node>) -> VariableBlockNode {
        VariableBlockNode {
            kind: NodeKind::VariableBlock,
            position: position,
            node: node
        }
    }
}

impl ToString for VariableBlockNode {
    fn to_string(&self) -> String {
        format!("{{ {:?} }}", self.node.to_string())
    }
}

#[derive(Debug)]
pub struct IdentifierNode {
    kind: NodeKind,
    position: usize,
    name: String,
}
impl_node!(IdentifierNode);

impl IdentifierNode {
    pub fn new(position: usize, name: String) -> IdentifierNode {
        IdentifierNode {
            kind: NodeKind::Identifier,
            position: position,
            name: name
        }
    }
}

impl ToString for IdentifierNode {
    fn to_string(&self) -> String {
        self.name.clone()
    }
}

#[derive(Debug)]
pub struct IntNode {
    kind: NodeKind,
    position: usize,
    value: i32,
}
impl_node!(IntNode);
impl IntNode {
    pub fn new(position: usize, value: i32) -> IntNode {
        IntNode {
            kind: NodeKind::Int,
            position: position,
            value: value
        }
    }
}
impl ToString for IntNode {
    fn to_string(&self) -> String {
        format!("{}", self.value)
    }
}

#[derive(Debug)]
pub struct FloatNode {
    kind: NodeKind,
    position: usize,
    value: f32,
}
impl_node!(FloatNode);
impl FloatNode {
    pub fn new(position: usize, value: f32) -> FloatNode {
        FloatNode {
            kind: NodeKind::Float,
            position: position,
            value: value
        }
    }
}
impl ToString for FloatNode {
    fn to_string(&self) -> String {
        format!("{}", self.value)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct BoolNode {
    kind: NodeKind,
    position: usize,
    value: bool,
}
impl_node!(BoolNode);
impl BoolNode {
    pub fn new(position: usize, value: bool) -> BoolNode {
        BoolNode {
            kind: NodeKind::Bool,
            position: position,
            value: value
        }
    }
}
impl ToString for BoolNode {
    fn to_string(&self) -> String {
        format!("{}", self.value)
    }
}
