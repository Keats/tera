use lexer::{Lexer, TokenType, Token};
use nodes::*;


#[derive(Debug)]
pub struct Parser {
    name: String,
    text: String,
    lexer: Lexer,
    root: ListNode,
    current_token: usize, // where we are in the parsing of the tokens
}

impl Parser {
    pub fn new(name: &str, text: &str) -> Parser {
        let mut lexer = Lexer::new(name, text);
        lexer.run();

        Parser {
            name: name.to_owned(),
            text: text.to_owned(),
            root: ListNode::new(0),
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

            self.root.nodes.push(node);
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
                TokenType::VariableStart => (),
                TokenType::TagStart => (),
                TokenType::Text => return self.parse_text(),
                _ => break
            };
        }

        None
    }

    fn parse_text(&mut self) -> Option<Box<Node>> {
        let token = self.next();
        Some(Box::new(TextNode::new(token.position, token.value)))
    }

    fn parse_variable_block(&mut self) -> Option<Box<Node>> {
        let token = self.expect(TokenType::VariableStart);
        let contained = self.parse_whole_expression(None, TokenType::VariableEnd);
        let node = VariableBlockNode::new(token.position, contained.unwrap());
        self.expect(TokenType::VariableEnd);

        Some(Box::new(node))
    }

    fn parse_whole_expression(&mut self, stack: Option<ListNode>, terminator: TokenType) -> Option<Box<Node>> {
        let token = self.peek_non_space();
        let node_stack = stack.unwrap_or(ListNode::new(token.position));
        // TODO: finish
        None
    }

    fn parse_single_expression(&mut self, stack: Option<ListNode>, terminator: TokenType) -> Option<Box<Node>> {
        let token = self.peek_non_space();

        if token.kind == terminator {
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
        Some(Box::new(IdentifierNode::new(ident.position, ident.value)))
    }

    fn parse_literal(&mut self) -> Option<Box<Node>> {
        let literal = self.next_non_space();

        match literal.kind {
            TokenType::Int => {
                let value = literal.value.parse::<i32>().unwrap();
                return Some(Box::new(IntNode::new(literal.position, value)));
            },
            TokenType::Float => {
                let value = literal.value.parse::<f32>().unwrap();
                return Some(Box::new(FloatNode::new(literal.position, value)));
            },
            TokenType::Bool => {
                let value = if literal.value == "false" { false } else { true };
                return Some(Box::new(BoolNode::new(literal.position, value)));
            },
            _ => panic!("unexpected type when parsing literal")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Parser};
    use nodes::*;

    fn compare_expected_nodes(expected: Vec<NodeKind>, got: ListNode) {

    }

    // #[test]
    // fn test_empty() {
    //     let mut parser = Parser::new("empty", "");
    //     parser.parse();
    //     assert_eq!(0, parser.root.nodes.len());
    // }

    #[test]
    fn test_plain_string() {
        let mut parser = Parser::new("plain_string", "Hello world");
        parser.parse();
        assert_eq!(1, parser.root.nodes.len());
        let node = parser.root.nodes[0];
        assert_eq!(node.get_kind(), NodeKind::Text);
    }
}
