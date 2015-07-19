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
  Number,
  Variable,
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
  chars: Vec<(usize, char)>,
  state: State,
  position: usize, // where we are in the chars vec
  start_position: usize, // where the current state started

  input_bytes_position: u32, // where we are in the input by bytes
}

impl Scanner {
  pub fn new(input: &str) -> Scanner {
    Scanner {
      name: "test".to_string(),
      input: input.to_string(),
      chars: input.char_indices().collect(),
      state: State::Text,
      position: 0,
      start_position: 0,
      input_bytes_position: 0,
    }
  }

  // We know we have {{ with self.position being on the first
  fn lex_left_variable_delimiter(&mut self) -> Token {
    self.position += 2;
    self.state = State::InsideBlock;

    Token::new(TokenType::VariableStart, "{{")
  }

  // TODO: see if we can simplify below
  fn lex_text(&mut self) -> Token {
    // Gets all chars until we reach {{ (only that for now)
    self.start_position = self.chars[self.position].0;
    let mut next_char_pos = self.chars[self.position].0;

    loop {
      if self.input[next_char_pos..].starts_with(LEFT_VARIABLE_DELIM) {
        self.state = State::VariableStart;
        break;
      }
      // TODO: add leftBlockDelim as the variable one

      // got to EOF
      if self.position == self.chars.len() - 1 {
        next_char_pos = self.input.len();
        break;
      }
      self.position += 1;
    }

    Token::new(
      TokenType::Text,
      &self.input[self.start_position..next_char_pos]
    )
  }
}

impl Iterator for Scanner {
  type Item = Token;

  fn next(&mut self) -> Option<Token> {
    // Empty template
    if self.input.len() == 0 {
      return None;
    }

    // Got to the end
    if self.position == self.chars.len() - 1 {
      return None;
    }

    match self.state {
      State::Text => Some(self.lex_text()),
      State::VariableStart => Some(self.lex_left_variable_delimiter()),
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

  fn check_if_correct(expected: Vec<Token>, obtained: Vec<Token>) -> bool {
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
      println!("{:?}", test.name);
      let tokens: Vec<Token> = Scanner::new(&test.input).collect();
      if tokens.len() != test.expected.len() {
        println!("Test {} failed: different number of tokens.", test.name);
        assert!(false);
      }

      if check_if_correct(test.expected, tokens) {
        assert!(true);
      } else {
        println!("Test {} failed: different tokens", test.name);
        assert!(false);
      }
    }
  }
}
