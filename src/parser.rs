use std::collections::HashMap;

use serde_json::value::{Value as Json, to_value};

use lexer::{Lexer, TokenType, Token};
use nodes::{Node, SpecificNode};

// Keeps track of which tag we are currently in
// Needed to parse inside if/for for example and keep track on when to stop
// those nodes
#[derive(Debug, Clone, PartialEq)]
enum InsideBlock {
    If,
    Elif,
    Else,
    For,
    Block
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
    tag_nodes: Vec<Node>, // if/for nodes
    pub blocks: HashMap<String, Node>,
    pub extends: Option<String>
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
            tag_nodes: vec![],
            blocks: HashMap::new(),
            extends: None
        };
        parser.parse();

        parser
    }

    // Main loop of the parser, stops when we are at EOF or
    pub fn parse(&mut self) {
        loop {
            match self.peek().kind {
                TokenType::TagStart => self.parse_tag_block(),
                TokenType::VariableStart => self.parse_variable_block(),
                TokenType::Text => self.parse_text(),
                TokenType::Eof => break,
                _ => unreachable!()
            };
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

    // Find out what's the token after the next non space one?
    // Very similar to peek_tag_name, TODO: see if we can replace it
    fn peek_two_non_space(&mut self) -> Token {
        let before_peeking = self.current_token;
        self.next_non_space();
        let token_after = self.peek_non_space();
        self.current_token = before_peeking;

        token_after
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

    // Panics if the expected token isn't found
    fn expect(&mut self, kind: TokenType) -> Token {
        let token = self.peek_non_space();
        if token.kind != kind {
            panic!("Unexpected token: {:?}, expected: {:?} at \
                line {} of template {}", token, kind, token.line, self.name);
        }

        self.next_non_space()
    }

    // Panics and use a TokenType::Error as input for the error message
    fn throw_lexer_error(&self, token: &Token) -> ! {
        panic!("Error: {} at line {} of template {}", token.value, token.line, self.name);
    }

    fn add_node(&mut self, node: Node) {
        if self.tag_nodes.is_empty() {
            // Blocks are aside from the AST if we are in a child template
            match node.specific {
                SpecificNode::Block {ref name, ..} => {
                    if self.blocks.contains_key(name) {
                        panic!("Block `{}` is duplicated in template `{}`", name, self.name)
                    }
                    self.blocks.insert(name.to_owned(), node.clone());
                    // Blocks are rendered if there is no extend
                    if self.extends.is_none() {
                        self.root.push(Box::new(node.clone()));
                    }
                },
                _ => { self.root.push(Box::new(node)); }
            }
            return;
        }

        let currently_in = self.currently_in.last().cloned().unwrap();
        match currently_in {
            InsideBlock::If | InsideBlock::Elif => {
                self.tag_nodes.last_mut().unwrap().append_to_last_conditional(Box::new(node));
            },
            InsideBlock::Else => {
                self.tag_nodes.last_mut().unwrap().push_to_else(Box::new(node));
            },
            InsideBlock::For | InsideBlock::Block => {
                self.tag_nodes.last_mut().unwrap().push(Box::new(node));
            }
        };
    }

    // Parse some html text
    fn parse_text(&mut self) {
        let token = self.next_token();
        // Need to check if first in if/for to left trim
        let mut string = match self.tag_nodes.last() {
            Some(n) => {
                // The value in unwrap_or is just a dummy, different from Block
                let currently_in = self.currently_in.last().cloned().unwrap_or(InsideBlock::For);
                if n.is_empty() && currently_in != InsideBlock::Block {
                    token.value.trim_left().to_owned()
                } else {
                    token.value
                }
            },
            None => token.value
        };
        // And to right trim if next token is {%
        if self.peek_non_space().kind == TokenType::TagStart {
            match self.peek_tag_name() {
                TokenType::Endif | TokenType::Elif | TokenType::Else => {
                    string = string.trim_right().to_owned();
                }
                _ => ()
            }
        }
        self.add_node(Node::new(token.position, SpecificNode::Text(string)));
    }

    // Parse the content of a {{ }} block
    fn parse_variable_block(&mut self)  {
        let token = self.expect(TokenType::VariableStart);
        let contained = self.parse_whole_expression(None, TokenType::VariableEnd);
        let node = Node::new(token.position, SpecificNode::VariableBlock(contained));
        self.expect(TokenType::VariableEnd);

        self.add_node(node);
    }

    // Parse the content of a {% %} block
    fn parse_tag_block(&mut self) {
        let token = self.expect(TokenType::TagStart);

        match self.peek_non_space().kind {
            TokenType::If => self.parse_if(token.position),
            TokenType::Elif => self.parse_elif(),
            TokenType::Else => self.parse_else(),
            TokenType::For => self.parse_for(token.position),
            TokenType::Block => self.parse_block(token.position),
            TokenType::Extends => self.parse_extends(token.position),
            TokenType::Endif | TokenType::Endfor | TokenType::Endblock => {
                match self.peek_non_space().kind {
                    TokenType::Endif => { self.expect(TokenType::Endif); },
                    TokenType::Endfor => { self.expect(TokenType::Endfor); },
                    TokenType::Endblock => {
                        self.expect(TokenType::Endblock);
                        let next_node = self.peek_non_space();
                        match self.tag_nodes.last_mut().unwrap().specific {
                            SpecificNode::Block {ref name, ..} => {
                                if next_node.kind == TokenType::TagEnd {
                                    panic!("Missing endblock name at line {} of \
                                        template `{}`. It should be `{}`.",
                                        next_node.line, self.name, name);
                                }

                                let end_name = next_node.value;
                                if end_name != name.clone() {
                                    panic!("Found endblock `{}` while we were \
                                        hoping for `{}` at line {} of template `{}`",
                                        end_name, name, next_node.line, self.name);
                                }
                            },
                            _ => unreachable!()
                        }

                        self.next_non_space();
                    },
                    _ => unreachable!()
                };
                self.expect(TokenType::TagEnd);
                let tag = self.tag_nodes.pop().unwrap();
                self.currently_in.pop();
                self.add_node(tag);
            },
            _ => unreachable!()
        };
    }

    fn parse_extends(&mut self, start_position: usize) {
        if start_position > 0 {
            panic!("{{% extends %}} tag need to be the first thing \
                in a template. It is not the case in `{}`", self.name);
        }
        self.expect(TokenType::Extends);
        let name = self.next_non_space();
        self.expect(TokenType::TagEnd);
        self.extends = Some(name.value);
    }

    // Parse a block tag (inheritance one)
    fn parse_block(&mut self, start_position: usize) {
        self.currently_in.push(InsideBlock::Block);
        self.expect(TokenType::Block);
        let name = self.next_non_space();
        self.expect(TokenType::TagEnd);
        let body = Node::new(self.peek().position, SpecificNode::List(vec![]));

        let block_node = Node::new(
            start_position,
            SpecificNode::Block {name: name.value, body: Box::new(body)}
        );
        self.tag_nodes.push(block_node);
    }

    // Parse if/elif condition and setups the body
    fn parse_conditional_node(&mut self) -> Node {
        // consume the tag name
        match self.next_non_space().kind {
            TokenType::If => self.currently_in.push(InsideBlock::If),
            TokenType::Elif => self.currently_in.push(InsideBlock::Elif),
            _ => unreachable!()
        };
        let condition = self.parse_whole_expression(None, TokenType::TagEnd);
        self.expect(TokenType::TagEnd);
        let body = Node::new(self.peek().position, SpecificNode::List(vec![]));

        Node::new(
            condition.position,
            SpecificNode::Conditional {condition: condition, body: Box::new(body)}
        )
    }

    fn parse_for(&mut self, start_position: usize) {
        self.currently_in.push(InsideBlock::For);
        self.expect(TokenType::For);
        let local = self.parse_single_expression(&TokenType::TagEnd);
        self.expect(TokenType::In);
        let array = self.parse_single_expression(&TokenType::TagEnd);
        self.expect(TokenType::TagEnd);

        let body = Node::new(self.peek().position, SpecificNode::List(vec![]));
        let for_node = Node::new(
            start_position,
            SpecificNode::For {local: local, array: array, body: Box::new(body)}
        );
        self.tag_nodes.push(for_node);
    }

    fn parse_if(&mut self, start_position: usize) {
        let mut if_node = Node::new(
            start_position,
            SpecificNode::If {condition_nodes: vec![], else_node: None}
        );
        let node = self.parse_conditional_node();
        if_node.push(Box::new(node));

        self.tag_nodes.push(if_node);
    }

    fn parse_elif(&mut self) {
        let currently_in = self.currently_in.last().cloned().unwrap();
        match currently_in {
            InsideBlock::If | InsideBlock::Elif => self.currently_in.pop(),
            InsideBlock::Else | InsideBlock::For | InsideBlock::Block => {
                panic!("Found a elif in a {:?} block at line {} of template `{}`, \
                 which is impossible.", currently_in, self.peek_non_space().line, self.name);
            }
        };
        let node = Box::new(self.parse_conditional_node());
        self.tag_nodes.last_mut().unwrap().push(node);
    }

    fn parse_else(&mut self) {
        let currently_in = self.currently_in.last().cloned().unwrap();
        match currently_in {
            InsideBlock::If | InsideBlock::Elif => self.currently_in.pop(),
            InsideBlock::Else | InsideBlock::For | InsideBlock::Block => {
                panic!("Found a else in a {:?} block at line {} of template `{}`, \
                 which is impossible.", currently_in, self.peek_non_space().line, self.name);
            }
        };
        self.expect(TokenType::Else);
        self.expect(TokenType::TagEnd);
        self.currently_in.push(InsideBlock::Else);

        let if_node = self.tag_nodes.pop().unwrap();
        let list = Node::new(self.peek().position, SpecificNode::List(vec![]));
        self.tag_nodes.push(Node::new(
            if_node.position,
            SpecificNode::If {
                condition_nodes: if_node.get_children(),
                else_node: Some(Box::new(list))
            }
        ));
    }

    // Parse a block/tag until we get to the terminator
    // Also handles all the precedence
    // AKA magic and dragons
    fn parse_whole_expression(&mut self, stack: Option<Node>, terminator: TokenType) -> Box<Node> {
        let token = self.peek_non_space();

        let mut node_stack = stack.unwrap_or_else(||
            Node::new(token.position, SpecificNode::List(vec![]))
        );

        let next = self.parse_single_expression(&terminator);
        node_stack.push(next);

        loop {
            let token = self.peek_non_space();
            if token.kind == terminator {
                if node_stack.is_empty() {
                    panic!("Unexpected terminator {} at line {} in template {}",
                        token.value, token.line, self.name);
                }
                return node_stack.pop();
            }

            // TODO: this whole thing can probably be refactored and simplified
            match token.kind {
                TokenType::Error => self.throw_lexer_error(&token),
                TokenType::Add | TokenType::Substract => {
                    // consume it
                    self.next_non_space();
                    if node_stack.is_empty() {
                        continue;
                    }

                    let rhs = self.parse_whole_expression(Some(node_stack.clone()), terminator);

                    // Now for + - we need to know if the next token has a higher
                    // precedence (ie * or /)
                    let next_token = self.peek_non_space();
                    if next_token.precedence() > token.precedence() {
                        node_stack.push(rhs);
                        return self.parse_whole_expression(Some(node_stack.clone()), terminator);
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
                    // * and / have the highest precedence so no need to check
                    // the following operators precedences
                    let rhs = self.parse_single_expression(&terminator);
                    let lhs = node_stack.pop();
                    let node = Node::new(
                        lhs.position,
                        SpecificNode::Math{lhs: lhs, rhs: rhs, operator: token.kind}
                    );
                    node_stack.push(Box::new(node));
                },
                TokenType::Equal | TokenType::NotEqual | TokenType::GreaterOrEqual
                | TokenType::Greater | TokenType::Lower | TokenType::LowerOrEqual => {
                    // Interrupt arithmetic when we meet one of those
                    if node_stack.len() > 1 {
                        return node_stack.pop();
                    }

                    // consume the operator
                    self.next_non_space();
                    // Those have the highest precedence in term of logic
                    // (higher than && and ||)
                    let rhs = self.parse_single_expression(&terminator);
                    let next_token = self.peek_non_space();

                    if next_token.precedence() > token.precedence() {
                        node_stack.push(rhs);
                        return self.parse_whole_expression(Some(node_stack.clone()), terminator);
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
                    let lhs = node_stack.pop();
                    let rhs = self.parse_whole_expression(Some(node_stack.clone()), terminator);
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
    fn parse_single_expression(&mut self, terminator: &TokenType) -> Box<Node> {
        let token = self.peek_non_space();

        if token.kind == *terminator {
            panic!("Terminator `{}` is too early at line {} in template {}",
                token.value, token.line, self.name);
        }

        match token.kind {
            TokenType::Error => self.throw_lexer_error(&token),
            TokenType::Identifier => self.parse_identifier(),
            TokenType::Float | TokenType::Int | TokenType::Bool => self.parse_literal(),
            TokenType::Function => self.parse_function(),
            _ => unreachable!()
        }
    }

    fn parse_function(&mut self) -> Box<Node> {
        let function = self.next_non_space();
        self.expect(TokenType::LeftParenthesis);
        let mut args = Vec::new();
        let mut kwargs: HashMap<String, Json> = HashMap::new();
        let mut kwargs_var = HashMap::new();

        loop {
            if self.peek_non_space().kind == TokenType::RightParenthesis {
                break;
            }
            println!("{:?}", self.peek_non_space());

            match self.peek_non_space().kind {
                TokenType::Float | TokenType::Int | TokenType::Bool => {
                    args.push(self.parse_literal());
                },
                TokenType::Identifier => {
                    // if the token after is Assign then it's a kwargs, otherwise just arg
                    match self.peek_two_non_space().kind {
                        TokenType::Comma | TokenType::RightParenthesis => {
                            args.push(self.parse_identifier());
                        },
                        TokenType::Assign => {
                            // kwargs!
                            let kwarg = self.next_non_space();
                            if kwargs.contains_key(&kwarg.value) || kwargs_var.contains_key(&kwarg.value) {
                                return self.throw_lexer_error(&kwarg);
                            }

                            self.expect(TokenType::Assign);
                            match self.peek_non_space().kind {
                                TokenType::Float | TokenType::Int | TokenType::Bool => {
                                    let literal = self.parse_literal();
                                    kwargs.insert(kwarg.value, literal.get_value());
                                },
                                TokenType::String => {
                                    kwargs.insert(kwarg.value, to_value(&self.next_non_space().value));
                                },
                                TokenType::Identifier => {
                                    let ident = self.next_non_space();
                                    kwargs_var.insert(kwarg.value, ident.value);
                                },
                                _ => unreachable!()
                            };
                        },
                        _ => unreachable!()
                    };
                },
                TokenType::Comma => { self.next_non_space(); },
                _ => unreachable!()
            };
        }

        self.expect(TokenType::RightParenthesis);

        Box::new(Node::new(function.position, SpecificNode::Function {
            name: function.value,
            args: args,
            kwargs: kwargs,
            kwargs_var: kwargs_var,
        }))
    }

    // Parse an identifier (variable name or keyword)
    fn parse_identifier(&mut self) -> Box<Node> {
        let ident = self.next_non_space();
        Box::new(Node::new(ident.position, SpecificNode::Identifier(ident.value)))
    }

    // Parse a bool/int/float
    fn parse_literal(&mut self) -> Box<Node> {
        let literal = self.next_non_space();

        match literal.kind {
            TokenType::Int => {
                let value = literal.value.parse::<i32>().unwrap();
                Box::new(Node::new(literal.position, SpecificNode::Int(value)))
            },
            TokenType::Float => {
                let value = literal.value.parse::<f32>().unwrap();
                Box::new(Node::new(literal.position, SpecificNode::Float(value)))
            },
            TokenType::Bool => {
                let value = literal.value == "true";
                Box::new(Node::new(literal.position, SpecificNode::Bool(value)))
            },
            _ => unreachable!()
        }
    }
}


#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serde_json::value::{to_value};

    use super::{Parser};
    use lexer::TokenType;
    use nodes::{Node, SpecificNode};

    fn compare_expected(expected: Vec<SpecificNode>, got: Vec<Box<Node>>) {
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
        compare_expected(expected, children)
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
            "{% if true %}1{% elif a %}2{% elif b %}3{% else %}4{% endif %}",
            vec![
                SpecificNode::If {
                    condition_nodes: vec![
                        Box::new(Node::new(6, SpecificNode::Conditional {
                            condition: Box::new(Node::new(6, SpecificNode::Bool(true))),
                            body: Box::new(Node::new(13, SpecificNode::List(vec![
                                Box::new(Node::new(13, SpecificNode::Text("1".to_owned()))),
                            ])))
                        })),
                        Box::new(Node::new(22, SpecificNode::Conditional {
                            condition: Box::new(Node::new(22, SpecificNode::Identifier("a".to_owned()))),
                            body: Box::new(Node::new(26, SpecificNode::List(vec![
                                Box::new(Node::new(26, SpecificNode::Text("2".to_owned()))),
                            ])))
                        })),
                        Box::new(Node::new(35, SpecificNode::Conditional {
                            condition: Box::new(Node::new(35, SpecificNode::Identifier("b".to_owned()))),
                            body: Box::new(Node::new(39, SpecificNode::List(vec![
                                Box::new(Node::new(39, SpecificNode::Text("3".to_owned()))),
                            ])))
                        })),
                    ],
                    else_node: Some(Box::new(Node::new(50, SpecificNode::List(vec![
                        Box::new(Node::new(50, SpecificNode::Text("4".to_owned()))),
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
                                Box::new(Node::new(13, SpecificNode::Text("parent".to_owned()))),
                                Box::new(Node::new(19, SpecificNode::If {
                                    condition_nodes: vec![
                                        Box::new(Node::new(25, SpecificNode::Conditional {
                                            condition: Box::new(Node::new(25, SpecificNode::Identifier("a".to_owned()))),
                                            body: Box::new(Node::new(29, SpecificNode::List(vec![
                                                Box::new(Node::new(29, SpecificNode::Text("nested".to_owned()))),
                                            ])))
                                        }))
                                    ],
                                    else_node: None,
                                })),
                            ])))
                        })),
                    ],
                    else_node: None
                },
                SpecificNode::Text(" hey".to_owned())
            ]
        );
    }

    #[test]
    fn test_for() {
        test_parser(
            "{% for x in items %}{% if x.show %}{{x}}{% endif %}{% endfor %}",
            vec![
                SpecificNode::For {
                    local: Box::new(Node::new(7, SpecificNode::Identifier("x".to_owned()))),
                    array: Box::new(Node::new(12, SpecificNode::Identifier("items".to_owned()))),
                    body: Box::new(Node::new(20, SpecificNode::List(vec![
                        Box::new(Node::new(20, SpecificNode::If {
                            condition_nodes: vec![
                                Box::new(Node::new(26, SpecificNode::Conditional {
                                    condition: Box::new(Node::new(26, SpecificNode::Identifier("x.show".to_owned()))),
                                    body: Box::new(Node::new(35, SpecificNode::List(vec![
                                        Box::new(Node::new(35, SpecificNode::VariableBlock(
                                            Box::new(Node::new(37, SpecificNode::Identifier("x".to_owned())))
                                        ))),
                                    ])))
                                }))
                            ],
                            else_node: None,
                        })),
                    ]))),
                }
            ]
        );
    }

    #[test]
    fn test_if_math_condition() {
        test_parser(
            "{% if age + 1 > 18 %}Adult{% endif %}",
            vec![
                SpecificNode::If {
                    condition_nodes: vec![
                        Box::new(Node::new(6, SpecificNode::Conditional {
                            condition: Box::new(Node::new(6, SpecificNode::Logic {
                                lhs: Box::new(Node::new(6, SpecificNode::Math {
                                    lhs: Box::new(Node::new(6, SpecificNode::Identifier("age".to_owned()))),
                                    rhs: Box::new(Node::new(12, SpecificNode::Int(1))),
                                    operator: TokenType::Add
                                })),
                                rhs: Box::new(Node::new(16, SpecificNode::Int(18))),
                                operator: TokenType::Greater
                            })),
                            body: Box::new(Node::new(21, SpecificNode::List(vec![
                                Box::new(Node::new(21, SpecificNode::Text("Adult".to_owned()))),
                            ])))
                        })),
                    ],
                    else_node: None
                },
            ]
        );
    }

    #[test]
    fn test_block() {
        test_parser(
            "{% block hello %}Hello{% endblock hello %}",
            vec![
                SpecificNode::Block {
                    name: "hello".to_owned(),
                    body: Box::new(Node::new(17, SpecificNode::List(vec![
                        Box::new(Node::new(17, SpecificNode::Text("Hello".to_owned()))),
                    ])))
                },
            ]
        );
    }

    #[test]
    fn test_extends() {
        let parser = Parser::new("dummy", "{% extends \"main.html\" %}");
        assert_eq!(parser.extends, Some("main.html".to_owned()));
    }

    #[test]
    fn test_function() {
        let mut kwargs = HashMap::new();
        kwargs.insert("format".to_owned(), to_value("YYYY-MM-DD"));

        let mut kwargs_var = HashMap::new();
        kwargs_var.insert("extra".to_owned(), "extra".to_owned());

        test_parser(
            "{{ format_date(birthday, format=\"YYYY-MM-DD\", extra=extra) }}",
            vec![
                SpecificNode::VariableBlock(
                    Box::new(Node::new(3, SpecificNode::Function {
                        name: "format_date".to_owned(),
                        args: vec![
                            Box::new(Node::new(15, SpecificNode::Identifier("birthday".to_owned())))
                        ],
                        kwargs: kwargs,
                        kwargs_var: kwargs_var
                    }))
                )
            ]
        );
    }
}
