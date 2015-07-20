const LEFT_VARIABLE_DELIM: &'static str  = "{{";
//const RIGHT_VARIABLE_DELIM: &'static str  = "}}";


// List of token types to emit to the parser.
// Different from the state enum despite some identical members
#[derive(PartialEq, Debug)]
pub enum TokenType {
  Text, // HTML text etc
  Space,
  VariableStart, // {{
  VariableEnd, // }}
  Variable, // variable name, tera keywords
  Int,
  Float,
  Operator, // + - * / .
  Error, // errors uncountered while lexing, such as 1.2.3 number
}

// List of different states the Tokenizer can be in
#[derive(Debug)]
enum State {
  Text,
  VariableStart,
  InsideBlock,
}

#[derive(PartialEq, Debug)]
pub struct Token {
  kind: TokenType,
  value: String,
}

impl Token {
  pub fn new(kind: TokenType, value: &str) -> Token {
    Token {
      kind: kind,
      value: value.to_string()
    }
  }
}

#[derive(Debug)]
pub struct Tokenizer {
  name: String,
  input: String,
  chars: Vec<(usize, char)>, // (bytes index, char)
  index: usize, // where we are in the chars vec
  state: State,
}

impl Tokenizer {
  pub fn new(input: &str) -> Tokenizer {
    // We want to figure out what's the initial state so we check the first
    // 2 chars
    let first_chars = input.chars().take(2).collect::<String>();
    // TODO: &* is pretty ugly, any way to fix that?
    let state = match &*first_chars {
      "{{" => State::VariableStart,
      _ => State::Text
    };

    Tokenizer {
      name: "test".to_string(),
      input: input.to_string(),
      chars: input.char_indices().collect(),
      index: 0,
      state: state,
    }
  }

  // Gets the substring a start index and self.index
  // Substring is non-inclusive
  fn get_substring(&self, start: usize) -> &str {
    let start_bytes = self.chars[start].0;
    let end_bytes = self.chars[self.index].0;

    // special case if the end index is the last char
    if self.is_over() {
      return &self.input[start_bytes..];
    }

    &self.input[start_bytes..end_bytes]
  }

  // We'll want to make sure we don't continue after the end in several
  // lexer methods
  fn is_over(&self) -> bool {
    self.index >= self.chars.len() - 1
  }

  // We know we have {{ with self.index being on the first
  fn lex_left_variable_delimiter(&mut self) -> Token {
    self.index += 2;
    self.state = State::InsideBlock;

    Token::new(TokenType::VariableStart, "{{")
  }

  // TODO: merge with the one above?
  fn lex_right_variable_delimiter(&mut self) -> Token {
    self.index += 2;
    self.state = State::Text;

    Token::new(TokenType::VariableEnd, "}}")
  }

  fn lex_text(&mut self) -> Token {
    let start_index = self.index;

    loop {
      if self.input[self.chars[self.index].0..].starts_with(LEFT_VARIABLE_DELIM) {
        self.state = State::VariableStart;
        break;
      }

      if self.is_over() {
        break;
      }

      self.index += 1;
    }

    Token::new(TokenType::Text, self.get_substring(start_index))
  }

  // We know we have a space, we need to figure out how many
  fn lex_space(&mut self) -> Token {
    let start_index = self.index;

    loop {
      if !self.chars[self.index].1.is_whitespace() {
        break;
      }

      self.index += 1;
    }

    Token::new(TokenType::Space, self.get_substring(start_index))
  }

  fn lex_number(&mut self) -> Token {
    let start_index = self.index;
    let mut number_type = TokenType::Int;
    let mut error = "";

    loop {
      match self.chars[self.index].1 {
        x if x.is_whitespace() || x == '}' => break,
        '.' => {
          match number_type {
            TokenType::Int => number_type = TokenType::Float,
            TokenType::Float => error = "A number has 2 dots",
            _ => {}
          }
        },
        x if !x.is_numeric() => error = "A number has unallowed chars",
        _ => {}
      }
      self.index += 1;
    }

    if error.len() > 0 {
      return Token::new(TokenType::Error, error);
    }

    Token::new(number_type, self.get_substring(start_index))
  }

  fn lex_variable(&mut self) -> Token {
    let start_index = self.index;

    loop {
      match self.chars[self.index].1 {
        x if x.is_whitespace() || x == '.' => break,
        _ => {}
      }
      self.index += 1;
    }

    Token::new(TokenType::Variable, self.get_substring(start_index))
  }

  fn lex_operator(&mut self) -> Token {
    self.index += 1;
    self.state = State::InsideBlock;

    Token::new(TokenType::Operator, self.get_substring(self.index - 1))
  }

  fn lex_inside_variable_block(&mut self) -> Token {
    match self.chars[self.index].1 {
      x if x.is_whitespace() => self.lex_space(),
      x if x.is_alphabetic() || x == '_' => self.lex_variable(),
      '}' => self.lex_right_variable_delimiter(),
      x if x.is_numeric() => self.lex_number(),
      '*' | '+' | '-' | '/' | '.' => self.lex_operator(),
      _ => Token::new(TokenType::Error, "Unknown char in variable block"),
    }
  }
}

impl Iterator for Tokenizer {
  type Item = Token;

  fn next(&mut self) -> Option<Token> {
    // Empty template or we got to the end
    if self.input.len() == 0 || self.is_over() {
      return None;
    }

    match self.state {
      State::Text => Some(self.lex_text()),
      State::VariableStart => Some(self.lex_left_variable_delimiter()),
      State::InsideBlock => Some(self.lex_inside_variable_block()),
    }
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  #[derive(Debug)]
  struct LexerTest {
    name: String,
    input: String,
    expected: Vec<Token>
  }

  impl LexerTest {
    pub fn new(name: &str, input: &str, expected: Vec<Token>) -> LexerTest {
      LexerTest {
        name: name.to_string(),
        input: input.to_string(),
        expected: expected
      }
    }
  }

  fn text_token(value: &str) -> Token {
    Token::new(TokenType::Text, value)
  }

  fn variable_token(value: &str) -> Token {
    Token::new(TokenType::Variable, value)
  }

  fn variable_start_token() -> Token {
    Token::new(TokenType::VariableStart, "{{")
  }

  fn variable_end_token() -> Token {
    Token::new(TokenType::VariableEnd, "}}")
  }

  fn space_token() -> Token {
    Token::new(TokenType::Space, " ")
  }

  fn int_token(value: &str) -> Token {
    Token::new(TokenType::Int, value)
  }

  fn float_token(value: &str) -> Token {
    Token::new(TokenType::Float, value)
  }

  fn operator_token(value: &str) -> Token {
    Token::new(TokenType::Operator, value)
  }

  fn error_token() -> Token {
    Token::new(TokenType::Error, "")
  }

  fn check_if_correct(expected: &Vec<Token>, obtained: &Vec<Token>) -> bool {
    if expected.len() != obtained.len() {
      return false;
    }

    for i in 0..expected.len() {
      if expected[i].kind != obtained[i].kind {
        return false;
      }

      // Do not check error values as i'll probably change them often
      if expected[i].kind == TokenType::Error {
        continue;
      }

      if expected[i].value != obtained[i].value {
        return false;
      }
    }
    return true;
  }

  #[test]
  fn test_lexer() {
    let tests: Vec<LexerTest> = vec![
      LexerTest::new("empty", "", vec![]),
      LexerTest::new("only text", "Hello 世界", vec![text_token("Hello 世界")]),
      LexerTest::new("variable and text", "{{ greeting }} 世界", vec![
        variable_start_token(),
        space_token(),
        variable_token("greeting"),
        space_token(),
        variable_end_token(),
        text_token(" 世界"),
      ]),
      LexerTest::new("numbers", "{{1 3.14}}", vec![
        variable_start_token(),
        int_token("1"),
        space_token(),
        float_token("3.14"),
        variable_end_token(),
      ]),
      LexerTest::new("invalid numbers", "{{1up 3.14.15}}", vec![
        variable_start_token(),
        error_token(),
        space_token(),
        error_token(),
        variable_end_token(),
      ]),
      LexerTest::new("operators", "{{+ - * / .}}", vec![
        variable_start_token(),
        operator_token("+"),
        space_token(),
        operator_token("-"),
        space_token(),
        operator_token("*"),
        space_token(),
        operator_token("/"),
        space_token(),
        operator_token("."),
        variable_end_token(),
      ])
    ];

    for test in tests {
      let tokens: Vec<Token> = Tokenizer::new(&test.input).collect();
      if tokens.len() != test.expected.len() {
        println!("Test {} failed: different number of tokens.", test.name);
        println!("Expected: {:?}", test.expected);
        println!("Got: {:?}", tokens);
        assert!(false);
      }

      if check_if_correct(&test.expected, &tokens) {
        assert!(true);
      } else {
        println!("Test {} failed: different tokens", test.name);
        println!("Expected: {:?}", test.expected);
        println!("Got: {:?}", tokens);
        assert!(false);
      }
    }
  }
}
