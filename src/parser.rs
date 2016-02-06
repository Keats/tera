use lexer::{Lexer, TokenType, Token};
use nodes::{Node, SpecificNode};


#[derive(Debug)]
pub struct Parser {
    name: String,
    text: String,
    lexer: Lexer,
    root: Node,
    current_token: usize, // where we are in the parsing of the tokens
}

impl Parser {
    pub fn new(name: &str, text: &str) -> Parser {
        let mut lexer = Lexer::new(name, text);
        lexer.run();

        Parser {
            name: name.to_owned(),
            text: text.to_owned(),
            root: Node::new(0, SpecificNode::List(vec![])),
            lexer: lexer,
            current_token: 0
        }
    }

    pub fn parse(&mut self) {
        loop {
            let node = match self.parse_next() {
                Some(n) => n,
                None => break
            };

            self.root.push(node);
        }
    }

    fn peek(&self) -> Token {
        self.lexer.tokens.get(self.current_token).unwrap().clone()
    }

    fn peek_non_space(&mut self) -> Token {
        let mut token = self.next();
        loop {
            if token.kind != TokenType::Space {
                break;
            }
            token = self.next();
        }
        // Only rewind once (see once i have tests)
        self.current_token -= 1;

        token
    }

    fn next(&mut self) -> Token {
        let token = self.peek();
        self.current_token += 1;

        token
    }

    fn next_non_space(&mut self) -> Token {
        let mut token = self.next();
        loop {
            if token.kind != TokenType::Space {
                break;
            }
            token = self.next();
        }

        token
    }

    fn expect(&mut self, kind: TokenType) -> Token {
        let token = self.peek_non_space();
        if token.kind != kind {
            panic!("Unexpected token: {:?}", token);
        }

        self.next_non_space()
    }

    fn parse_next(&mut self) -> Option<Box<Node>> {
        loop {
            match self.peek().kind {
                TokenType::VariableStart => return self.parse_variable_block(),
                TokenType::TagStart => (),
                TokenType::Text => return self.parse_text(),
                _ => break
            };
        }

        None
    }

    fn parse_text(&mut self) -> Option<Box<Node>> {
        let token = self.next();
        Some(Box::new(Node::new(token.position, SpecificNode::Text(token.value))))
    }

    fn parse_variable_block(&mut self) -> Option<Box<Node>> {
        let token = self.expect(TokenType::VariableStart);
        let contained = self.parse_whole_expression(None, TokenType::VariableEnd);
        let node = Node::new(token.position, SpecificNode::VariableBlock(contained.unwrap()));
        self.expect(TokenType::VariableEnd);

        Some(Box::new(node))
    }

    // Parse a block/tag until we get to the terminator
    fn parse_whole_expression(&mut self, stack: Option<Node>, terminator: TokenType) -> Option<Box<Node>> {
        let token = self.peek_non_space();

        let mut node_stack = stack.unwrap_or(Node::new(token.position, SpecificNode::List(vec![])));
        let next = self.parse_single_expression(&node_stack, &terminator).unwrap();
        node_stack.push(next);

        loop {
            let token = self.peek_non_space();
            if token.kind == terminator {
                if node_stack.get_children().len() == 0 {
                    panic!("Unexpected terminator");
                }
                return node_stack.get_children().pop();
            }
        }

        None
    }

    fn parse_single_expression(&mut self, stack: &Node, terminator: &TokenType) -> Option<Box<Node>> {
        let token = self.peek_non_space();

        if token.kind == *terminator {
            panic!("Unexpected terminator");
        }

        match token.kind {
            TokenType::Identifier => return self.parse_identifier(),
            TokenType::Float | TokenType::Int | TokenType::Bool => return self.parse_literal(),
            TokenType::Add | TokenType::Substract => {
                panic!("wololo");
            }
            _ => panic!("unexpected")
        }

        None
    }

    fn parse_identifier(&mut self) -> Option<Box<Node>> {
        let ident = self.next_non_space();
        Some(Box::new(Node::new(ident.position, SpecificNode::Identifier(ident.value))))
    }

    fn parse_literal(&mut self) -> Option<Box<Node>> {
        let literal = self.next_non_space();

        match literal.kind {
            TokenType::Int => {
                let value = literal.value.parse::<i32>().unwrap();
                return Some(Box::new(Node::new(literal.position, SpecificNode::Int(value))));
            },
            TokenType::Float => {
                let value = literal.value.parse::<f32>().unwrap();
                return Some(Box::new(Node::new(literal.position, SpecificNode::Float(value))));
            },
            TokenType::Bool => {
                let value = if literal.value == "false" { false } else { true };
                return Some(Box::new(Node::new(literal.position, SpecificNode::Bool(value))));
            },
            _ => panic!("unexpected type when parsing literal")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Parser};
    use nodes::{Node, SpecificNode};

    fn compared_expected(expected: Vec<SpecificNode>, got: Vec<Box<Node>>) {
        if expected.len() != got.len() {
            assert!(false);
        }

        for (i, node) in got.iter().enumerate() {
            let expected_node = expected.get(i).unwrap().clone();
            assert_eq!(expected_node, node.specific);
        }
    }

    fn test_parser(input: &str, expected: Vec<SpecificNode>) {
        let mut parser = Parser::new("dummy", input);
        parser.parse();
        let children = parser.root.get_children();
        compared_expected(expected, children)
    }

    // #[test]
    // fn test_empty() {
    //     let mut parser = Parser::new("empty", "");
    //     parser.parse();
    //     assert_eq!(0, parser.root.get_children().len());
    // }

    // #[test]
    // fn test_plain_string() {
    //     test_parser(
    //         "Hello world",
    //         vec![SpecificNode::Text("Hello world".to_owned())]
    //     );
    // }

    #[test]
    fn test_variable_block_and_text() {
        test_parser(
            "{{ greeting }} 世界",
            vec![
                SpecificNode::VariableBlock(
                    Box::new(Node::new(3, SpecificNode::Identifier("greeting".to_owned())))
                ),
                SpecificNode::Text(" 世界".to_owned()),
            ]
        );
    }
}
