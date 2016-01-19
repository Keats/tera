use std::fmt;

#[derive(Clone, PartialEq, Debug)]
enum NodeKind {
    List, // Contains other nodes
    Text, // html
    Variable, // one of the variables to replace
    Int,
    Float
}

pub trait Node {
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
        write!(f, "(kind: {:?}, position: {})", self.kind, self.position)
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

    pub fn get_nodes(self) -> Vec<Box<Node>> {
        self.nodes
        // let cloned = vec![];
        // for node in &self.nodes {
        //     cloned.push(node.clone());
        // }
        // cloned
    }
}

#[derive(Clone, Debug)]
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
