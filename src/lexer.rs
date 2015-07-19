// TODO: remove below
#![allow(dead_code)]

//let leftBlockDelim: String = "{%".to_string();
//let rightBlockDelim: String = "%}".to_string();
const LEFT_VARIABLE_DELIM: &'static str  = "{{";
const RIGHT_VARIABLE_DELIM: &'static str  = "}}";


// List of token types to emit to the parser.
// Different from the state enum despite some identical members
#[derive(PartialEq, Debug)]
pub enum TokenType {
  Text, // HTML text etc
  Space,
  //BlockStart,
  //BlockEnd,
  VariableStart, // {{
  VariableEnd, // }}
  Variable, // variable name, tera keywords
}

// List of different states the scanner can be in
#[derive(Debug)]
enum State {
  Text,
  VariableStart,
  VariableEnd,
  InsideBlock,
  Variable,
  Operator, // an operator in a block, such as * in {{ price * 100 }}
  Number, // a number in a block, such as 100 in {{ price * 100 }}
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
pub struct Scanner {
  name: String,
  input: String,
  chars: Vec<(usize, char)>, // (bytes index, char)
  index: usize, // where we are in the chars vec
  state: State,
  start_position: usize, // where the current state started
}

impl Scanner {
  pub fn new(input: &str) -> Scanner {
    Scanner {
      name: "test".to_string(),
      input: input.to_string(),
      chars: input.char_indices().collect(),
      index: 0,
      state: State::Text,
      start_position: 0,
    }
  }

  // We know we have {{ with self.index being on the first
  fn lex_left_variable_delimiter(&mut self) -> Option<Token> {
    self.index += 2;
    self.state = State::InsideBlock;

    Some(Token::new(TokenType::VariableStart, "{{"))
  }

  // TODO: merge with the one above?
  fn lex_right_variable_delimiter(&mut self) -> Option<Token> {
    self.index += 2;
    self.state = State::Text;

    Some(Token::new(TokenType::VariableEnd, "}}"))
  }

  // TODO: see if we can simplify below
  fn lex_text(&mut self) -> Option<Token> {
    // Gets all chars until we reach {{ (only that for now)
    self.start_position = self.chars[self.index].0;
    let mut next_index = self.chars[self.index].0;

    loop {
      if self.input[next_index..].starts_with(LEFT_VARIABLE_DELIM) {
        self.state = State::VariableStart;

        // If we have a delim right at the beginning
        if next_index == 0 {
          return self.lex_left_variable_delimiter();
        }
        break;
      }
      // TODO: add leftBlockDelim as the variable one

      // got to EOF
      if self.index == self.chars.len() - 1 {
        next_index = self.input.len();
        break;
      }
      self.index += 1;
    }

    Some(Token::new(
      TokenType::Text,
      &self.input[self.start_position..next_index]
    ))
  }

  // We know we have a space, we need to figure out how many
  fn lex_space(&mut self) -> Option<Token> {
    self.start_position = self.chars[self.index].0;

    loop {
      if !self.chars[self.index].1.is_whitespace() {
        break;
      }

      self.index += 1;
    }

    Some(Token::new(
      TokenType::Space,
      &self.input[self.start_position..self.chars[self.index].0]
    ))
  }

  fn lex_variable(&mut self) -> Option<Token> {
    self.start_position = self.chars[self.index].0;

    loop {
      let current = self.chars[self.index].1;
      if current == '.' || current.is_whitespace() {
        break;
      }
      self.index += 1;
    }

    Some(Token::new(
      TokenType::Variable,
      &self.input[self.start_position..self.chars[self.index].0]
    ))
  }

  // Works for both {{ }} and {% %}
  fn lex_inside_block(&mut self) -> Option<Token> {
    match self.chars[self.index].1 {
      x if x.is_whitespace() => return self.lex_space(),
      x if x.is_alphabetic() || x == '_' => return self.lex_variable(),
      '}' => return self.lex_right_variable_delimiter(),
      _ => None,
    }

  }
}

impl Iterator for Scanner {
  type Item = Token;

  fn next(&mut self) -> Option<Token> {
    // Empty template
    // TODO: maybe put it in lex_text if we return an Option?
    if self.input.len() == 0 {
      return None;
    }

    // Got to the end
    if self.index >= self.chars.len() - 1 {
      return None;
    }

    match self.state {
      State::Text => self.lex_text(),
      State::VariableStart => self.lex_left_variable_delimiter(),
      State::InsideBlock => self.lex_inside_block(),
      _ => None,
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

  fn check_if_correct(expected: &Vec<Token>, obtained: &Vec<Token>) -> bool {
    if expected.len() != obtained.len() {
      return false;
    }

    for i in 0..expected.len() {
      if expected[i].kind != obtained[i].kind {
        return false;
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
      LexerTest::new("only variable", "{{ greeting }}", vec![
        variable_start_token(),
        space_token(),
        variable_token("greeting"),
        space_token(),
        variable_end_token()
      ]),
      // LexerTest::new("variable and text", "{{ greeting }} 世界", vec![
      //   variable_start_token(),
      //   space_token(),
      //   variable_token("greeting"),
      //   space_token(),
      //   variable_end_token(),
      //   text_token(" 世界")
      // ]),
    ];

    for test in tests {
      let tokens: Vec<Token> = Scanner::new(&test.input).collect();
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
