use std::fmt;

// Still missing strings, () and []
// List of token types to emit to the parser.
// Different from the state enum despite some identical members
#[derive(Clone, PartialEq, Debug)]
pub enum TokenType {
    Text, // HTML text
    Space,
    VariableStart, // {{
    VariableEnd, // }}
    Identifier, // variable name for example
    TagStart, // {%
    TagEnd, // %}
    Int,
    Float,
    Bool,
    Add, // +
    Substract, // -
    Multiply, // *
    Divide, // /
    Greater, // >
    GreaterOrEqual, // >=
    Lower, // <,
    LowerOrEqual, // <=
    Equal, // ==
    NotEqual, // !=
    And, // &&
    Or, // ||
    Pipe, // |
    Error, // errors uncountered while lexing, such as 1.2.3 number
    Eof,
    // And now tera keywords
    If,
    Else,
    Elif,
    Endif,
    For,
    In,
    Endfor,
}

#[derive(Clone, Debug)]
pub struct Token {
    pub kind: TokenType,
    pub value: String,
    pub line: usize,
    pub position: usize // byte position in the input
}

impl Token {
    pub fn new(kind: TokenType, input: &str, line: usize, position: usize) -> Token {
        Token {
            kind: kind,
            value: input.to_owned(),
            line: line,
            position: position
        }
    }

    // Precedence for a token. We need to know that in order for the
    // parser to do its job correctly when it comes to math and comparisons
    pub fn precedence(&self) -> usize {
        match self.kind {
            TokenType::Multiply | TokenType::Divide => 5,
            TokenType::Add | TokenType::Substract => 4,
            TokenType::Equal | TokenType::GreaterOrEqual | TokenType::Greater
            | TokenType::NotEqual | TokenType::LowerOrEqual | TokenType::Lower => {
                3
            },
            TokenType::And => 2,
            TokenType::Or => 1,
            _ => 0
        }
    }
}

// can't use cyclic references in a type so we use a newtype struct where it
// works for some reason
struct StateFn(Option<fn(&mut Lexer) -> StateFn>);
impl fmt::Debug for StateFn {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // TODO: can we get a function name?
        write!(f, "")
    }
}

/// which kind of block are we currently in (to know which type of token type to emit)
/// We only have 2 types (3 if we add comments): {{ }} and {% %}
#[derive(Debug)]
enum BlockType {
    Variable,
    Block,
}

/// We need to keep track of which side of a delimiter we need to add
/// when lexing inside of a block
#[derive(Debug)]
enum DelimiterSide {
    Left,
    Right,
}

/// Ok we're using the Right-Facing Armenian Eternity Sign as EOF char since it
/// looks pretty and doesn't seem used at all by a google search and the code is neater if
/// we don't use Option (U+058D)
const EOF: char = '֍';

/// Lexer based on the one used in go templates (https://www.youtube.com/watch?v=HxaD_trXwRE)
#[derive(Debug)]
pub struct Lexer {
    name: String, // name of input, to report errors
    input: String, // template being lexed
    chars: Vec<(usize, char)>, // (bytes index, char)
    start: usize, // where the current item started in the input (in bytes)
    position: usize, // current position in the input (in bytes)
    last_position: usize, // last position in the input (in bytes)
    current_char: usize, // current index in the chars vec
    state: StateFn, // current state fn
    current_block_type: BlockType, // whether we are in a {{ or {% block
    pub tokens: Vec<Token> // tokens found
}

impl Lexer {
    pub fn new(name: &str, input: &str) -> Lexer {
        Lexer {
            name: name.to_owned(),
            input: input.to_owned(),
            chars: input.char_indices().collect(),
            start: 0,
            position: 0,
            last_position: 0,
            current_char: 0,
            tokens: vec![],
            current_block_type: BlockType::Variable, // we don't care about default one
            state: StateFn(Some(lex_text))
        }
    }

    // Do the whole lexing thingy
    pub fn run(&mut self) {
        loop {
            // It's a bit weird how we get the value of a newtype struct
            let StateFn(state_fn) = self.state;
            if state_fn.is_none() {
                break;
            }
            self.state = state_fn.unwrap()(self);
        }
    }

    // Gets the next char in the input. Note that input is utf8 therefore
    // with of a character can be > 1
    fn next_char(&mut self) -> char {
        if self.is_over() {
            return EOF;
        }

        let current_char = self.chars[self.current_char];
        // There's no way to get a char width in rust afaik so we calculate
        // it by comparing with the next char
        let width = if self.current_char < self.chars.len() - 1 {
            self.chars[self.current_char + 1].0 - current_char.0
        } else {
            self.input.len() - current_char.0
        };
        self.last_position = self.position;
        self.position += width;
        self.current_char += 1;

        current_char.1
    }

    fn backup(&mut self) {
        self.position = self.last_position;
        self.current_char -= 1;
    }

    fn peek(&mut self) -> char {
        let next_char = self.next_char();
        self.backup();

        next_char
    }

    // Get the line number of the position by counting the number
    // of '\n' in it
    // TODO: actually check if that works
    fn get_line_number(&self) -> usize {
        1 + self.get_substring(0, self.last_position)
              .chars()
              .filter(|&c| c == '\n')
              .collect::<Vec<_>>()
              .len()
    }

    // Text tokens are a bit special as we start as a text token
    // but the input might be empty
    fn add_text_token(&mut self) {
        if self.position > self.start {
            self.add_token(TokenType::Text);
        }
    }

    // Need to get a substring on the bytes
    fn get_substring(&self, start: usize, end: usize) -> String {
        String::from_utf8(self.input.as_bytes()[start..end].to_vec()).unwrap()
    }

    fn add_token(&mut self, kind: TokenType) {
        let line = self.get_line_number();
        let substring = self.get_substring(self.start, self.position);

        self.tokens.push(Token::new(kind, &substring, line, self.start));
        self.start = self.position;
    }

    // Returns whether the next char is the char we expected to see
    fn accept(&mut self, valid: char) -> bool {
        if self.next_char() == valid {
            return true;
        }
        self.backup();

        false
    }

    fn starts_with(&self, pattern: &str) -> bool {
        self.get_substring(self.position, self.input.len()).starts_with(pattern)
    }

    // Errors are slighty different as we give them a value
    fn error(&mut self, message: &str) -> StateFn {
        let line = self.get_line_number();
        self.tokens.push(
            Token::new(TokenType::Error, message, line, self.position)
        );

        StateFn(None)
    }

    fn is_over(&self) -> bool {
        self.position >= self.input.len()
    }

    // Easy way to handle delimiters while lexing rather than duplicating
    // the logic in 2 almost identical lexing functions
    fn add_delimiter(&mut self, side: DelimiterSide) -> StateFn {
        self.position += 2;
        match self.current_block_type {
            BlockType::Block => match side {
                DelimiterSide::Left => self.add_token(TokenType::TagStart),
                DelimiterSide::Right => self.add_token(TokenType::TagEnd),
            },
            BlockType::Variable => match side {
                DelimiterSide::Left => self.add_token(TokenType::VariableStart),
                DelimiterSide::Right => self.add_token(TokenType::VariableEnd),
            }
        }
        self.start = self.position;
        self.current_char += 2;

        match side {
            DelimiterSide::Left => StateFn(Some(lex_inside_block)),
            DelimiterSide::Right => StateFn(Some(lex_text)),
        }
    }
}


fn lex_text(lexer: &mut Lexer) -> StateFn {
    while !lexer.is_over() {
        match lexer.chars[lexer.current_char].1 {
            '{' => {
                if lexer.starts_with("{{") {
                    lexer.add_text_token();
                    lexer.current_block_type = BlockType::Variable;
                    return lexer.add_delimiter(DelimiterSide::Left);
                } else if lexer.starts_with("{%") {
                    lexer.add_text_token();
                    lexer.current_block_type = BlockType::Block;
                    return lexer.add_delimiter(DelimiterSide::Left);
                }
            },
            _ => {
                if lexer.next_char() == EOF {
                    break;
                }
            }
        }
    }

    lexer.add_text_token();
    lexer.add_token(TokenType::Eof);

    StateFn(None)
}

fn lex_space(lexer: &mut Lexer) -> StateFn {
    while lexer.peek().is_whitespace() {
        lexer.next_char();
    }

    lexer.add_token(TokenType::Space);
    StateFn(Some(lex_inside_block))
}

fn lex_number(lexer: &mut Lexer) -> StateFn {
    let mut token_type = TokenType::Int;

    loop {
        match lexer.next_char() {
            x if x.is_numeric() => continue,
            '.' => {
                if token_type == TokenType::Int {
                    token_type = TokenType::Float;
                } else {
                    return lexer.error("Two dots in a number");
                }
            },
            _ => {
                lexer.backup();
                lexer.add_token(token_type);
                return StateFn(Some(lex_inside_block));
            }
        }
    }
}

// Lexing a word inside a block
// could be a variable lookup or a tera keyword
fn lex_identifier(lexer: &mut Lexer) -> StateFn {
    loop {
        match lexer.next_char() {
            x if x.is_alphabetic() ||x == '_' => continue,
            _ => {
                lexer.backup();
                match lexer.get_substring(lexer.start, lexer.position).as_ref() {
                    "if" => lexer.add_token(TokenType::If),
                    "else" => lexer.add_token(TokenType::Else),
                    "elif" => lexer.add_token(TokenType::Elif),
                    "endif" => lexer.add_token(TokenType::Endif),
                    "for" => lexer.add_token(TokenType::For),
                    "in" => lexer.add_token(TokenType::In),
                    "endfor" => lexer.add_token(TokenType::Endfor),
                    "true" | "false" => lexer.add_token(TokenType::Bool),
                    _ => lexer.add_token(TokenType::Identifier)
                }

                return StateFn(Some(lex_inside_block));
            }
        }
    }
}

fn lex_inside_block(lexer: &mut Lexer) -> StateFn {
    while !lexer.is_over() {
        // Check if we are at the end of the block
        if lexer.starts_with("}}") || lexer.starts_with("%}") {
            return lexer.add_delimiter(DelimiterSide::Right);
        }

        match lexer.next_char() {
            x if x.is_whitespace() => { return StateFn(Some(lex_space)); }
            x if x.is_numeric() => { return StateFn(Some(lex_number)); }
            x if x.is_alphabetic() || x == '_' => { return StateFn(Some(lex_identifier)); }
            '-' => lexer.add_token(TokenType::Substract),
            '+' => lexer.add_token(TokenType::Add),
            '*' => lexer.add_token(TokenType::Multiply),
            '/' => lexer.add_token(TokenType::Divide),
            '=' =>  {
                if lexer.accept('=') {
                    lexer.add_token(TokenType::Equal);
                } else {
                    lexer.error("Unknown token");
                }
            },
            '&' =>  {
                if lexer.accept('&') {
                    lexer.add_token(TokenType::And);
                } else {
                    lexer.error("Unknown token");
                }
            },
            '|' =>  {
                if lexer.accept('|') {
                    lexer.add_token(TokenType::Or);
                } else {
                    lexer.add_token(TokenType::Pipe);
                }
            },
            '!' =>  {
                if lexer.accept('=') {
                    lexer.add_token(TokenType::NotEqual);
                } else {
                    lexer.error("Unknown token");
                }
            },
            '<' =>  {
                if lexer.accept('=') {
                    lexer.add_token(TokenType::LowerOrEqual);
                } else {
                    lexer.add_token(TokenType::Lower);
                }
            },
            '>' =>  {
                if lexer.accept('=') {
                    lexer.add_token(TokenType::GreaterOrEqual);
                } else {
                    lexer.add_token(TokenType::Greater);
                }
            },
            _ => { return StateFn(None); }
        };
    }

    lexer.error("Unclosed Delimiter")
}


#[cfg(test)]
mod tests {
    use super::{TokenType, Lexer};
    use super::TokenType::*;

    #[derive(Debug)]
    struct TokenTest<'a> {
        kind: TokenType,
        value: &'a str,
    }
    impl<'a> TokenTest<'a> {
        fn new(kind: TokenType, value: &'a str) -> TokenTest<'a> {
            TokenTest { kind: kind, value: value }
        }
    }
    const T_TAG_START: TokenTest<'static> = TokenTest { kind: TagStart, value: "{%"};
    const T_TAG_END: TokenTest<'static> = TokenTest { kind: TagEnd, value: "%}"};
    const T_VARIABLE_START: TokenTest<'static> = TokenTest { kind: VariableStart, value: "{{"};
    const T_VARIABLE_END: TokenTest<'static> = TokenTest { kind: VariableEnd, value: "}}"};
    const T_EOF: TokenTest<'static> = TokenTest { kind: Eof, value: ""};
    const T_ADD: TokenTest<'static> = TokenTest { kind: Add, value: "+"};
    const T_SUBSTRACT: TokenTest<'static> = TokenTest { kind: Substract, value: "-"};
    const T_MULTIPLY: TokenTest<'static> = TokenTest { kind: Multiply, value: "*"};
    const T_DIVIDE: TokenTest<'static> = TokenTest { kind: Divide, value: "/"};
    const T_SPACE: TokenTest<'static> = TokenTest { kind: Space, value: " "};
    const T_IF: TokenTest<'static> = TokenTest { kind: If, value: "if"};
    const T_ELSE: TokenTest<'static> = TokenTest { kind: Else, value: "else"};
    const T_ELIF: TokenTest<'static> = TokenTest { kind: Elif, value: "elif"};
    const T_ENDIF: TokenTest<'static> = TokenTest { kind: Endif, value: "endif"};
    const T_FOR: TokenTest<'static> = TokenTest { kind: For, value: "for"};
    const T_IN: TokenTest<'static> = TokenTest { kind: In, value: "in"};
    const T_ENDFOR: TokenTest<'static> = TokenTest { kind: Endfor, value: "endfor"};
    const T_GREATER: TokenTest<'static> = TokenTest { kind: Greater, value: ">"};
    const T_GREATER_OR_EQUAL: TokenTest<'static> = TokenTest { kind: GreaterOrEqual, value: ">="};
    const T_LOWER: TokenTest<'static> = TokenTest { kind: Lower, value: "<"};
    const T_LOWER_OR_EQUAL: TokenTest<'static> = TokenTest { kind: LowerOrEqual, value: "<="};
    const T_EQUAL: TokenTest<'static> = TokenTest { kind: Equal, value: "=="};
    const T_NOTEQUAL: TokenTest<'static> = TokenTest { kind: NotEqual, value: "!="};
    const T_AND: TokenTest<'static> = TokenTest { kind: And, value: "&&"};
    const T_OR: TokenTest<'static> = TokenTest { kind: Or, value: "||"};

    fn identifier_token(ident: &str) -> TokenTest {
        TokenTest::new(Identifier, ident)
    }

    fn text_token(text: &str) -> TokenTest {
        TokenTest::new(Text, text)
    }

    fn int_token(value: &str) -> TokenTest {
        TokenTest::new(Int, value)
    }

    fn float_token(value: &str) -> TokenTest {
        TokenTest::new(Float, value)
    }

    fn error_token(msg: &str) -> TokenTest {
        TokenTest::new(Error, msg)
    }

    fn test_tokens(input: &str, test_tokens: Vec<TokenTest>) {
        let mut lexer = Lexer::new("test", input);
        lexer.run();

        if test_tokens.len() != lexer.tokens.len() {
            println!("Number of tokens not matching: expected {}, got {}", test_tokens.len(), lexer.tokens.len());
            println!("{:#?}", lexer.tokens);
            assert!(false);
        }

        for (i, t) in test_tokens.iter().enumerate() {
            let lexer_token = &lexer.tokens[i];
            // Should always start at position 0
            if i == 0 {
                assert_eq!(lexer_token.position, 0);
            }
            if t.kind != lexer_token.kind {
                println!("Wrong kind. Expected: {:?}. \n Got: {:?}", t, lexer_token);
                assert!(false);
            }
            if t.value != lexer_token.value {
                println!("Wrong value. Expected: {:?}. \n Got: {:?}", t, lexer_token);
                assert!(false);
            }

        }
    }

    #[test]
    fn test_empty() {
        let expected = vec![T_EOF];
        test_tokens("", expected);
    }

    #[test]
    fn test_only_text() {
        let expected = vec![text_token("Hello\n 世界"), T_EOF];
        test_tokens("Hello\n 世界", expected);
    }

    #[test]
    fn test_variable_block_and_text() {
        let expected = vec![
            T_VARIABLE_START,
            T_SPACE,
            identifier_token("greeting"),
            T_SPACE,
            T_VARIABLE_END,
            text_token(" 世界"),
            T_EOF
        ];
        test_tokens("{{ greeting }} 世界", expected);
    }

    #[test]
    fn test_valid_numbers() {
        let expected = vec![
            T_VARIABLE_START,
            T_SPACE,
            int_token("1"),
            T_SPACE,
            float_token("3.14"),
            T_SPACE,
            T_VARIABLE_END,
            T_EOF
        ];
        test_tokens("{{ 1 3.14 }}", expected);
    }

    #[test]
    fn test_numbers_and_variable() {
        let expected = vec![
            T_VARIABLE_START,
            T_SPACE,
            int_token("1"),
            T_SPACE,
            T_MULTIPLY,
            T_SPACE,
            identifier_token("vat_rate"),
            T_SPACE,
            T_VARIABLE_END,
            T_EOF
        ];
        test_tokens("{{ 1 * vat_rate }}", expected);
    }

    #[test]
    fn test_operators() {
        let expected = vec![
            T_VARIABLE_START,
            T_SUBSTRACT,
            T_ADD,
            T_MULTIPLY,
            T_DIVIDE,
            T_EQUAL,
            T_AND,
            T_LOWER_OR_EQUAL,
            T_GREATER_OR_EQUAL,
            T_NOTEQUAL,
            T_OR,
            T_VARIABLE_END,
            T_EOF
        ];
        test_tokens("{{-+*/==&&<=>=!=||}}", expected);
    }

    #[test]
    fn test_block() {
        let expected = vec![
            text_token("Hello "),
            T_TAG_START, T_SPACE,
            T_IF,
            T_SPACE,
            identifier_token("japanese"),
            T_SPACE,
            T_TAG_END,
            text_token("世界"),
            T_TAG_START,
            T_SPACE,
            T_ELSE,
            T_SPACE,
            T_TAG_END,
            text_token("world"),
            T_TAG_START,
            T_SPACE,
            T_ENDIF,
            T_SPACE,
            T_TAG_END,
            T_EOF
        ];
        test_tokens("Hello {% if japanese %}世界{% else %}world{% endif %}", expected);
    }

    #[test]
    fn test_unclosed_block() {
        let expected = vec![T_VARIABLE_START, error_token("Unclosed Delimiter")];
        test_tokens("{{", expected);
    }

    #[test]
    fn test_invalid_number() {
        let expected = vec![T_VARIABLE_START, error_token("Two dots in a number")];
        test_tokens("{{1.2.2", expected);
    }
}
