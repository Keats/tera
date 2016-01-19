use std::fmt;

use nodes::{Node, ListNode, TextNode};
use lexer::{Lexer, ItemType, Item};


pub struct Tree {
    name: String,
    root: ListNode,
    lexer: Lexer,
    peeks: Vec<Item>
}

impl fmt::Debug for Tree {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,
            "(name: {}, root: {:?}, lexer: {:?})",
            self.name, self.root, self.lexer.clone().collect::<Vec<Item>>()
        )
    }
}

impl Tree {
    pub fn new(name: &str, input: &str) -> Tree {
        let lexer = Lexer::new(input);

        Tree {
            name: name.to_owned(),
            lexer: lexer,
            root: ListNode::new(0),
            peeks: vec![]
        }
    }

    fn is_unexpected(&self, token: Item) -> ! {
        // TODO: get line number
        panic!("Unexpected item {:?}", token);
    }

    fn next_non_space(&mut self) -> Item {
        // We will have peeked the first one already
        let mut token = self.peeks[0].clone();
        self.peeks = vec![];
        loop {
            if token.kind != ItemType::Space {
                break;
            }
            token = self.lexer.next().unwrap();
        }
        token
    }

    // fn variable_block(&mut self) -> Box<TextNode> {

    // }

    fn text_or_action(&mut self) -> Box<TextNode> {
        let next = self.next_non_space();
        println!("Text or action: {:?}", next);
        match next.kind {
            ItemType::Text => Box::new(TextNode::new(next.position, next.value)),
            // ItemType::VariableStart => (),
            _ => self.is_unexpected(next)
        }
    }

    fn parse(&mut self) {
        loop {
            let next = match self.lexer.next() {
                Some(n) => n,
                None => break
            };
            if next.kind == ItemType::Eof {
                break
            }
            println!("Next: {:?}", next);
            self.peeks = vec![next];
            let n = self.text_or_action();
            self.root.append(n);
        }
    }
}

pub fn parse(name: &str, input: &str) -> Tree {
    let mut tree = Tree::new(name, input);
    tree.parse();

    tree
}

#[cfg(test)]
mod tests {
    use super::{parse};

    #[test]
    fn test_empty() {
        let tree = parse("test", "");
        assert_eq!(tree.root.nodes.len(), 0);
    }

    #[test]
    fn test_only_text() {
        let tree = parse("test", "Hello world");
        assert_eq!(tree.root.nodes.len(), 1);
        //assert_eq!(tree.root.nodes[0].as_ref().text, "Hello world");
    }

    #[test]
    fn test_only_variable() {
        let tree = parse("test", "{{ greeting }}");
        assert_eq!(tree.root.nodes.len(), 1);
    }
}
