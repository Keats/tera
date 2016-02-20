use lexer::{Lexer, TokenType, Token};
use nodes::{Node, SpecificNode};

// TODO: vec![block_type]

// Keeps track of which tag we are currently in
// Needed to parse inside if/for for example and keep track on when to stop
// those nodes
#[derive(Debug, Clone)]
enum InsideBlock {
    If,
    Elif,
    Else,
    For
}

#[derive(Debug)]
pub struct Parser {
    name: String,
    text: String,
    lexer: Lexer,
    pub root: Node,
    current_token: usize, // where we are in the parsing of the tokens

    // The ones below are needed for nested if/for blocks
    currently_in: Vec<InsideBlock>,
    tag_nodes: Vec<Node>
}

impl Parser {
    pub fn new(name: &str, text: &str) -> Parser {
        let mut lexer = Lexer::new(name, text);
        lexer.run();

        let mut parser = Parser {
            name: name.to_owned(),
            text: text.to_owned(),
            root: Node::new(0, SpecificNode::List(vec![])),
            lexer: lexer,
            current_token: 0,

            currently_in: vec![],
            tag_nodes: vec![]
        };
        parser.parse();

        parser
    }

    // Main loop of the parser, stops when there are no token left
    pub fn parse(&mut self) {
        while let Some(node) = self.parse_next() {
            self.root.push(node);
        }
    }

    // Look at the next token
    fn peek(&self) -> Token {
        self.lexer.tokens.get(self.current_token).unwrap().clone()
    }

    // Look at the next token that isn't space
    fn peek_non_space(&mut self) -> Token {
        let mut token = self.next_token();
        loop {
            if token.kind != TokenType::Space {
                break;
            }
            token = self.next_token();
        }
        self.current_token -= 1;

        token
    }

    // Get the next token
    fn next_token(&mut self) -> Token {
        let token = self.peek();
        self.current_token += 1;

        token
    }

    // Get the next token that isn't space
    fn next_non_space(&mut self) -> Token {
        let mut token = self.next_token();
        loop {
            if token.kind != TokenType::Space {
                break;
            }
            token = self.next_token();
        }

        token
    }

    // Used at a {% token to know the tag name
    fn peek_tag_name(&mut self) -> TokenType {
        let before_peeking = self.current_token;
        self.next_token();
        let tag_name = self.peek_non_space();
        self.current_token = before_peeking;

        tag_name.kind
    }

    fn rewind(&mut self) {
        self.current_token += 1;
    }

    // Panics if the expected token isn't found
    fn expect(&mut self, kind: TokenType) -> Token {
        let token = self.peek_non_space();
        if token.kind != kind {
            panic!("Unexpected token: {:?}, expected: {:?}", token, kind);
        }

        self.next_non_space()
    }

    // All the different "states" the parser can be in: either in a block, in text
    // or in a tag
    fn parse_next(&mut self) -> Option<Box<Node>> {
        loop {
            match self.peek().kind {
                TokenType::TagStart => {
                    let tag_name = self.peek_tag_name();
                    // Out of the match because I was getting a match have incompatible
                    // return types
                    if tag_name == TokenType::Endif && self.tag_nodes.len() == 1 {
                        println!("What's left? {:?}", self.currently_in);
                        println!("tag_nodes: {:?}", self.tag_nodes);
                        return self.parse_tag_block();
                    }
                    match tag_name {
                        TokenType::If | TokenType::Elif
                        | TokenType::Else | TokenType::Endif => self.parse_tag_block(),
                        _ => unreachable!()
                    };
                },
                TokenType::VariableStart => return self.parse_variable_block(),
                TokenType::Text => return self.parse_text(),
                TokenType::Eof => {
                    if self.tag_nodes.len() == 1 {
                        return Some(Box::new(self.tag_nodes.pop().unwrap()));
                    }
                    break;
                }
                _ => break
            };
        }

        None
    }

    // Parse some html text
    fn parse_text(&mut self) -> Option<Box<Node>> {
        let token = self.next_token();
        Some(Box::new(Node::new(token.position, SpecificNode::Text(token.value))))
    }

    // Parse the content of a {{ }} block
    fn parse_variable_block(&mut self) -> Option<Box<Node>> {
        let token = self.expect(TokenType::VariableStart);
        let contained = self.parse_whole_expression(None, TokenType::VariableEnd);
        let node = Node::new(token.position, SpecificNode::VariableBlock(contained.unwrap()));
        self.expect(TokenType::VariableEnd);

        Some(Box::new(node))
    }

    // Parse the content of a {% %} block
    fn parse_tag_block(&mut self) -> Option<Box<Node>> {
        let token = self.expect(TokenType::TagStart);
        match self.peek_non_space().kind {
            TokenType::If | TokenType::Elif | TokenType::Else => self.parse_if_block(token.position),
            TokenType::Endif => {
                println!("Found endif!");
                self.expect(TokenType::Endif);
                self.expect(TokenType::TagEnd);
                // println!("Got 1 {:?}", self.tag_nodes);
                if self.tag_nodes.len() == 1 {
                    return self.tag_nodes.pop().map(|n| Box::new(n));
                } else {
                    let last = self.tag_nodes.pop().unwrap();
                    self.tag_nodes.last_mut().unwrap().push(Box::new(last));
                }
            },
            _ => unreachable!()
        };

        None
    }

    // Used by parse_if_block to parse the if/elif/else nodes
    fn parse_conditional_nodes(&mut self) -> Option<Box<Node>> {
        // consume the tag name
        match self.next_non_space().kind {
            TokenType::If => self.currently_in.push(InsideBlock::If),
            TokenType::Elif => self.currently_in.push(InsideBlock::Elif),
            _ => unreachable!()
        };
        let condition = self.parse_whole_expression(None, TokenType::TagEnd).unwrap();
        self.expect(TokenType::TagEnd);

        let body = self.parse_tag_body().unwrap();

        Some(Box::new(
            Node::new(
                condition.position,
                SpecificNode::Conditional {condition: condition, body: body}
            )
        ))
    }

    fn parse_if_block(&mut self, start_position: usize) {
        // {% if true %}parent{% if a %}nested{% endif b%}{% endif %}
        match self.peek_non_space().kind {
            TokenType::If => {
                self.tag_nodes.push(Node::new(
                    start_position,
                    SpecificNode::If {condition_nodes: vec![], else_node: None}
                ));
                let conditional = self.parse_conditional_nodes().unwrap();
                println!("Parsed conditional");
                println!("{:#?}", conditional);
                println!("-----");
                println!("Got a if, current tag_nodes: {:?}", self.tag_nodes);
                match self.tag_nodes.pop() {
                    Some(mut t) => {
                        t.push(conditional);
                        self.tag_nodes.push(t);
                    }
                    None => {
                        self.tag_nodes.push(Node::new(
                            start_position,
                            SpecificNode::If {condition_nodes: vec![conditional], else_node: None}
                        ));
                    }
                }
            }
            TokenType::Elif => {
                let mut if_node = self.tag_nodes.pop().unwrap();
                if_node.push(self.parse_conditional_nodes().unwrap());
                self.tag_nodes.push(if_node);
            },
            TokenType::Else => {
                self.expect(TokenType::Else);
                self.expect(TokenType::TagEnd);
                self.currently_in.push(InsideBlock::Else);
                // Replace the last one now that we have else
                let if_node = self.tag_nodes.pop().unwrap();
                let else_body = self.parse_tag_body();
                self.tag_nodes.push(Node::new(
                    if_node.position,
                    SpecificNode::If {
                        condition_nodes: if_node.get_children(),
                        else_node: else_body
                    }
                ));
            },
            _ => unreachable!()
        }
    }

    // Same as normal parsing except it can stop on elif/else/endif/endfor
    // Meeds to keep track of how many levels deep we are, for example
    // if we have a {% if x %}{% if y %}{% endif %}{% endif %}, parsing
    // should continue until the last endif and not stop at the first
    fn parse_tag_body(&mut self) -> Option<Box<Node>> {
        let mut body = Node::new(self.peek().position, SpecificNode::List(vec![]));

        loop {
            let node = match self.parse_next() {
                Some(n) => n,
                None => {
                    panic!("Unexpected EOF");
                }
            };

            body.push(node);
            let currently = self.currently_in.last().cloned().unwrap();

            // TODO: consume endif here
            match self.peek().kind {
                TokenType::TagStart => {
                    let tag_name = self.peek_tag_name();
                    match currently {
                        InsideBlock::If | InsideBlock::Elif => match tag_name {
                            TokenType::Elif | TokenType::Else => {
                                self.currently_in.pop();
                                return Some(Box::new(body));
                            },
                            TokenType::Endif => {
                                self.expect(TokenType::TagStart);
                                self.expect(TokenType::Endif);
                                self.expect(TokenType::TagEnd);
                                self.currently_in.pop();
                                return Some(Box::new(body))
                            }
                            TokenType::Endfor => panic!("Unexpected endfor"),
                            _ => ()
                        },
                        InsideBlock::Else => match tag_name {
                            TokenType::Endif => {
                                self.expect(TokenType::TagStart);
                                self.expect(TokenType::Endif);
                                self.expect(TokenType::TagEnd);
                                self.currently_in.pop();
                                return Some(Box::new(body))
                            },
                            TokenType::Endfor | TokenType::Elif | TokenType::Else  => panic!("Unexpected {}", tag_name),
                            _ => ()
                        },
                        InsideBlock::For => match tag_name {
                            TokenType::Endfor => {
                                self.currently_in.pop();
                                return Some(Box::new(body));
                            },
                            TokenType::If | TokenType::Elif | TokenType::Else | TokenType::Endif  => panic!("Unexpected {}", tag_name),
                            _ => ()
                        },
                    }
                },
                TokenType::Eof => {
                    self.currently_in.pop();
                    return Some(Box::new(body));
                },
                _ => ()
            }
        }
    }

    // Parse a block/tag until we get to the terminator
    // Also handles all the precedence
    fn parse_whole_expression(&mut self, stack: Option<Node>, terminator: TokenType) -> Option<Box<Node>> {
        let token = self.peek_non_space();

        let mut node_stack = stack.unwrap_or_else(||
            Node::new(token.position, SpecificNode::List(vec![]))
        );

        let next = self.parse_single_expression(&terminator).unwrap();
        node_stack.push(next);

        loop {
            let token = self.peek_non_space();
            if token.kind == terminator {
                if node_stack.is_empty() {
                    panic!("Unexpected terminator");
                }
                return Some(node_stack.pop());
            }

            // TODO: this whole thing can probably be refactored and simplified
            match token.kind {
                TokenType::Add | TokenType::Substract => {
                    // consume it
                    self.next_non_space();
                    if node_stack.is_empty() {
                        continue;
                    }

                    let rhs = self.parse_whole_expression(Some(node_stack.clone()), terminator.clone()).unwrap();

                    // Now for + - we need to know if the next token has a higher
                    // precedence (ie * or /)
                    let next_token = self.peek_non_space();
                    if next_token.precedence() > token.precedence() {
                        node_stack.push(rhs);
                        return self.parse_whole_expression(Some(node_stack.clone()), terminator.clone());
                    } else {
                        // Or the next thing has lower precedence and we just
                        // add the node to the stack
                        let lhs = node_stack.pop();
                        let node = Node::new(
                            lhs.position,
                            SpecificNode::Math{lhs: lhs, rhs: rhs, operator: token.kind}
                        );
                        node_stack.push(Box::new(node));
                    }
                },
                TokenType::Divide | TokenType::Multiply => {
                    // consume the operator
                    self.next_non_space();
                    if node_stack.is_empty() {
                        panic!("Unexpected division or multiplication"); // TODO details
                    }

                    // * and / have the highest precedence so no need to check
                    // the following operators precedences
                    let rhs = self.parse_single_expression(&terminator).unwrap();
                    let lhs = node_stack.pop();
                    let node = Node::new(
                        lhs.position,
                        SpecificNode::Math{lhs: lhs, rhs: rhs, operator: token.kind}
                    );
                    node_stack.push(Box::new(node));
                },
                TokenType::Equal | TokenType::NotEqual | TokenType::GreaterOrEqual
                | TokenType::Greater | TokenType::Lower | TokenType::LowerOrEqual => {
                    // consume the operator
                    self.next_non_space();
                    // Those have the highest precedence in term of logic
                    // (higher than && and ||)
                    if node_stack.is_empty() {
                        panic!("Unexpected logic token"); // TODO details
                    }

                    let rhs = self.parse_single_expression(&terminator).unwrap();
                    let next_token = self.peek_non_space();

                    if next_token.precedence() > token.precedence() {
                        node_stack.push(rhs);
                        return self.parse_whole_expression(Some(node_stack.clone()), terminator.clone());
                    } else {
                        let lhs = node_stack.pop();
                        let node = Node::new(
                            lhs.position,
                            SpecificNode::Logic{lhs: lhs, rhs: rhs, operator: token.kind}
                        );
                        node_stack.push(Box::new(node));
                    }
                },
                TokenType::And | TokenType::Or => {
                    // consume the operator
                    self.next_non_space();
                    if node_stack.is_empty() {
                        panic!("Unexpected logic token"); // TODO details
                    }
                    let lhs = node_stack.pop();
                    let rhs = self.parse_whole_expression(Some(node_stack.clone()), terminator.clone()).unwrap();
                    let node = Node::new(
                        lhs.position,
                        SpecificNode::Logic{lhs: lhs, rhs: rhs, operator: token.kind}
                    );
                    node_stack.push(Box::new(node));

                },
                _ => unreachable!()
            }
        }
    }

    // Parses the next non-space token as a simple expression
    // Used when parsing inside a block/tag and we want to get the next value
    fn parse_single_expression(&mut self, terminator: &TokenType) -> Option<Box<Node>> {
        let token = self.peek_non_space();

        if token.kind == *terminator {
            panic!("Unexpected terminator");
        }

        match token.kind {
            TokenType::Identifier => return self.parse_identifier(),
            TokenType::Float | TokenType::Int | TokenType::Bool => return self.parse_literal(),
            _ => panic!("unexpected token type: {:?}", token.kind)
        }

        None
    }

    // Parse an identifier (variable name or keyword)
    fn parse_identifier(&mut self) -> Option<Box<Node>> {
        let ident = self.next_non_space();
        Some(Box::new(Node::new(ident.position, SpecificNode::Identifier(ident.value))))
    }

    // Parse a bool/int/float
    fn parse_literal(&mut self) -> Option<Box<Node>> {
        let literal = self.next_non_space();

        match literal.kind {
            TokenType::Int => {
                let value = literal.value.parse::<i32>().unwrap();
                Some(Box::new(Node::new(literal.position, SpecificNode::Int(value))))
            },
            TokenType::Float => {
                let value = literal.value.parse::<f32>().unwrap();
                Some(Box::new(Node::new(literal.position, SpecificNode::Float(value))))
            },
            TokenType::Bool => {
                let value = if literal.value == "false" { false } else { true };
                Some(Box::new(Node::new(literal.position, SpecificNode::Bool(value))))
            },
            _ => unreachable!()
        }
    }
}


#[cfg(test)]
mod tests {
    use super::{Parser};
    use lexer::TokenType;
    use nodes::{Node, SpecificNode};

    fn compared_expected(expected: Vec<SpecificNode>, got: Vec<Box<Node>>) {
        if expected.len() != got.len() {
            println!("Got: {:#?}", got);
            assert!(false);
        }

        for (i, node) in got.iter().enumerate() {
            let expected_node = expected.get(i).unwrap().clone();
            if expected_node != node.specific {
                println!("Expected: {:#?}", expected_node);
                println!("Got: {:#?}", node.specific);
            }
            assert_eq!(expected_node, node.specific);
        }
    }

    fn test_parser(input: &str, expected: Vec<SpecificNode>) {
        let parser = Parser::new("dummy", input);
        let children = parser.root.get_children();
        compared_expected(expected, children)
    }

    #[test]
    fn test_empty() {
        let parser = Parser::new("empty", "");
        assert_eq!(0, parser.root.len());
    }

    #[test]
    fn test_plain_string() {
        test_parser(
            "Hello world",
            vec![SpecificNode::Text("Hello world".to_owned())]
        );
    }

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

    #[test]
    fn test_basic_math() {
        test_parser(
            "{{1+3.41}}{{1-42}}{{1*42}}{{1/42}}{{test+1}}",
            vec![
                SpecificNode::VariableBlock(
                    Box::new(Node::new(2, SpecificNode::Math {
                        lhs: Box::new(Node::new(2, SpecificNode::Int(1))),
                        rhs: Box::new(Node::new(4, SpecificNode::Float(3.41))),
                        operator: TokenType::Add
                    }))
                ),
                SpecificNode::VariableBlock(
                    Box::new(Node::new(12, SpecificNode::Math {
                        lhs: Box::new(Node::new(12, SpecificNode::Int(1))),
                        rhs: Box::new(Node::new(14, SpecificNode::Int(42))),
                        operator: TokenType::Substract
                    }))
                ),
                SpecificNode::VariableBlock(
                    Box::new(Node::new(20, SpecificNode::Math {
                        lhs: Box::new(Node::new(20, SpecificNode::Int(1))),
                        rhs: Box::new(Node::new(22, SpecificNode::Int(42))),
                        operator: TokenType::Multiply
                    }))
                ),
                SpecificNode::VariableBlock(
                    Box::new(Node::new(28, SpecificNode::Math {
                        lhs: Box::new(Node::new(28, SpecificNode::Int(1))),
                        rhs: Box::new(Node::new(30, SpecificNode::Int(42))),
                        operator: TokenType::Divide
                    }))
                ),
                SpecificNode::VariableBlock(
                    Box::new(Node::new(36, SpecificNode::Math {
                        lhs: Box::new(Node::new(36, SpecificNode::Identifier("test".to_owned()))),
                        rhs: Box::new(Node::new(41, SpecificNode::Int(1))),
                        operator: TokenType::Add
                    }))
                ),
            ]
        );
    }

    #[test]
    fn test_math_precedence_simple() {
        test_parser(
            "{{ 1 / 2 + 1 }}",
            vec![
                SpecificNode::VariableBlock(
                    Box::new(Node::new(3, SpecificNode::Math {
                        lhs: Box::new(Node::new(3, SpecificNode::Math {
                            lhs: Box::new(Node::new(3, SpecificNode::Int(1))),
                            rhs: Box::new(Node::new(7, SpecificNode::Int(2))),
                            operator: TokenType::Divide
                        })),
                        rhs: Box::new(Node::new(11, SpecificNode::Int(1))),
                        operator: TokenType::Add
                    }))
                ),
            ]
        );
    }

    #[test]
    fn test_math_precedence_complex() {
        test_parser(
            "{{ 1 / 2 + 3 * 2 + 42 }}",
            vec![
                SpecificNode::VariableBlock(
                    Box::new(Node::new(3, SpecificNode::Math {
                        lhs: Box::new(Node::new(3, SpecificNode::Math {
                            lhs: Box::new(Node::new(3, SpecificNode::Int(1))),
                            rhs: Box::new(Node::new(7, SpecificNode::Int(2))),
                            operator: TokenType::Divide
                        })),
                        rhs: Box::new(Node::new(11, SpecificNode::Math {
                            lhs: Box::new(Node::new(11, SpecificNode::Math {
                                lhs: Box::new(Node::new(11, SpecificNode::Int(3))),
                                rhs: Box::new(Node::new(15, SpecificNode::Int(2))),
                                operator: TokenType::Multiply
                            })),
                            rhs: Box::new(Node::new(19, SpecificNode::Int(42))),
                            operator: TokenType::Add
                        })),
                        operator: TokenType::Add
                    }))
                ),
            ]
        );
    }

    #[test]
    fn test_basic_logic() {
        test_parser(
            "{{1==1}}{{1>1}}{{1<1}}{{1>=1}}{{1<=1}}{{1&&1}}{{1||1}}",
            vec![
                SpecificNode::VariableBlock(
                    Box::new(Node::new(2, SpecificNode::Logic {
                        lhs: Box::new(Node::new(2, SpecificNode::Int(1))),
                        rhs: Box::new(Node::new(5, SpecificNode::Int(1))),
                        operator: TokenType::Equal
                    }))
                ),
                SpecificNode::VariableBlock(
                    Box::new(Node::new(10, SpecificNode::Logic {
                        lhs: Box::new(Node::new(10, SpecificNode::Int(1))),
                        rhs: Box::new(Node::new(12, SpecificNode::Int(1))),
                        operator: TokenType::Greater
                    }))
                ),
                SpecificNode::VariableBlock(
                    Box::new(Node::new(17, SpecificNode::Logic {
                        lhs: Box::new(Node::new(17, SpecificNode::Int(1))),
                        rhs: Box::new(Node::new(19, SpecificNode::Int(1))),
                        operator: TokenType::Lower
                    }))
                ),
                SpecificNode::VariableBlock(
                    Box::new(Node::new(24, SpecificNode::Logic {
                        lhs: Box::new(Node::new(24, SpecificNode::Int(1))),
                        rhs: Box::new(Node::new(27, SpecificNode::Int(1))),
                        operator: TokenType::GreaterOrEqual
                    }))
                ),
                SpecificNode::VariableBlock(
                    Box::new(Node::new(32, SpecificNode::Logic {
                        lhs: Box::new(Node::new(32, SpecificNode::Int(1))),
                        rhs: Box::new(Node::new(35, SpecificNode::Int(1))),
                        operator: TokenType::LowerOrEqual
                    }))
                ),
                SpecificNode::VariableBlock(
                    Box::new(Node::new(40, SpecificNode::Logic {
                        lhs: Box::new(Node::new(40, SpecificNode::Int(1))),
                        rhs: Box::new(Node::new(43, SpecificNode::Int(1))),
                        operator: TokenType::And
                    }))
                ),
                SpecificNode::VariableBlock(
                    Box::new(Node::new(48, SpecificNode::Logic {
                        lhs: Box::new(Node::new(48, SpecificNode::Int(1))),
                        rhs: Box::new(Node::new(51, SpecificNode::Int(1))),
                        operator: TokenType::Or
                    }))
                ),
            ]
        );
    }

    #[test]
    fn test_logic_precedence_complex() {
        test_parser(
            "{{1 > 2 || 3 == 4 && admin}}",
            vec![
                SpecificNode::VariableBlock(
                    Box::new(Node::new(2, SpecificNode::Logic {
                        lhs: Box::new(Node::new(2, SpecificNode::Logic {
                            lhs: Box::new(Node::new(2, SpecificNode::Int(1))),
                            rhs: Box::new(Node::new(6, SpecificNode::Int(2))),
                            operator: TokenType::Greater
                        })),
                        rhs: Box::new(Node::new(11, SpecificNode::Logic {
                            lhs: Box::new(Node::new(11, SpecificNode::Logic {
                                lhs: Box::new(Node::new(11, SpecificNode::Int(3))),
                                rhs: Box::new(Node::new(16, SpecificNode::Int(4))),
                                operator: TokenType::Equal
                            })),
                            rhs: Box::new(Node::new(21, SpecificNode::Identifier("admin".to_owned()))),
                            operator: TokenType::And
                        })),
                        operator: TokenType::Or
                    }))
                ),
            ]
        )
    }

    #[test]
    fn test_if() {
        test_parser(
            "{% if true %}Hey{% elif a %}Hey{% elif b%}Hey{% else %}Hey{% endif %}",
            vec![
                SpecificNode::If {
                    condition_nodes: vec![
                        Box::new(Node::new(6, SpecificNode::Conditional {
                            condition: Box::new(Node::new(6, SpecificNode::Bool(true))),
                            body: Box::new(Node::new(13, SpecificNode::List(vec![
                                Box::new(Node::new(13, SpecificNode::Text("Hey".to_owned()))),
                            ])))
                        })),
                        Box::new(Node::new(24, SpecificNode::Conditional {
                            condition: Box::new(Node::new(24, SpecificNode::Identifier("a".to_owned()))),
                            body: Box::new(Node::new(28, SpecificNode::List(vec![
                                Box::new(Node::new(28, SpecificNode::Text("Hey".to_owned()))),
                            ])))
                        })),
                        Box::new(Node::new(39, SpecificNode::Conditional {
                            condition: Box::new(Node::new(39, SpecificNode::Identifier("b".to_owned()))),
                            body: Box::new(Node::new(42, SpecificNode::List(vec![
                                Box::new(Node::new(42, SpecificNode::Text("Hey".to_owned()))),
                            ])))
                        })),
                    ],
                    else_node: Some(Box::new(Node::new(55, SpecificNode::List(vec![
                        Box::new(Node::new(55, SpecificNode::Text("Hey".to_owned()))),
                    ]))))
                },
            ]
        );
    }


    #[test]
    fn test_nested_if() {
        test_parser(
            "{% if true %}parent{% if a %}nested{% endif %}{% endif %} hey",
            vec![
                SpecificNode::If {
                    condition_nodes: vec![
                        Box::new(Node::new(6, SpecificNode::Conditional {
                            condition: Box::new(Node::new(6, SpecificNode::Bool(true))),
                            body: Box::new(Node::new(13, SpecificNode::List(vec![
                                Box::new(Node::new(13, SpecificNode::Text("Hey".to_owned()))),
                            ])))
                        })),
                        Box::new(Node::new(24, SpecificNode::Conditional {
                            condition: Box::new(Node::new(24, SpecificNode::Identifier("a".to_owned()))),
                            body: Box::new(Node::new(28, SpecificNode::List(vec![
                                Box::new(Node::new(28, SpecificNode::Text("Hey".to_owned()))),
                            ])))
                        })),
                        Box::new(Node::new(39, SpecificNode::Conditional {
                            condition: Box::new(Node::new(39, SpecificNode::Identifier("b".to_owned()))),
                            body: Box::new(Node::new(42, SpecificNode::List(vec![
                                Box::new(Node::new(42, SpecificNode::Text("Hey".to_owned()))),
                            ])))
                        })),
                    ],
                    else_node: None
                },
            ]
        );
    }
}
