const LEFT_VARIABLE_DELIM: &'static str = "{{";
const RIGHT_VARIABLE_DELIM: &'static str = "}}";


// List of Item types to emit to the parser.
// Different from the state enum despite some identical members
#[derive(PartialEq, Debug)]
pub enum ItemType {
    Text, // HTML text etc
    Space,
    VariableStart, // {{
    VariableEnd, // }}
    Variable, // variable name
    Int,
    Float,
    Operator, // + - * / .
    Error, // errors uncountered while lexing, such as 1.2.3 number
    Eof
}

// All the potential errors we can encouter
#[derive(Debug)]
enum LexerError {
    InvalidNumber,
    UnclosedVariableBlock,
    NestedDelimiter,
    UnknownChar
}


impl ToString for LexerError {
    fn to_string(&self) -> String {
        match *self {
            LexerError::InvalidNumber => "Invalid Number".to_owned(),
            LexerError::UnclosedVariableBlock => "Unclosed variable block".to_owned(),
            LexerError::UnknownChar => "Unknown char".to_owned(),
            LexerError::NestedDelimiter => "Nesting delimiter is not allowed".to_owned(),
        }
    }
}

// List of different states the Lexer can be in
#[derive(Debug)]
enum State {
    Text,
    VariableStart,
    InsideBlock,
    Over,
}

#[derive(PartialEq, Debug)]
pub struct Item {
    kind: ItemType,
    value: String,
    position: usize,
}

impl Item {
    pub fn new(kind: ItemType, value: &str, position: usize) -> Item {
        Item {
            kind: kind,
            value: value.to_string(),
            position: position,
        }
    }
}

#[derive(Debug)]
pub struct Lexer {
    name: String,
    input: String,
    chars: Vec<(usize, char)>, // (bytes index, char)
    index: usize, // where we are in the chars vec
    state: State,
}

impl Lexer {
    fn new(input: &str) -> Lexer {
        // We want to figure out what's the initial state so we check the first
        // 2 chars, we need that since every method returns an item
        let first_chars = input.chars().take(2).collect::<String>();
        let state = match &*first_chars {
            LEFT_VARIABLE_DELIM => State::VariableStart,
            _ => State::Text
        };

        Lexer {
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

    fn lex_left_variable_delimiter(&mut self) -> Item {
        let start_index = self.index;
        if self.is_over() {
            self.state = State::Over;
            return Item::new(ItemType::Eof, "", start_index);
        }
        self.index += 2;
        self.state = State::InsideBlock;
        Item::new(ItemType::VariableStart, LEFT_VARIABLE_DELIM, start_index)
    }

    fn lex_right_variable_delimiter(&mut self) -> Item {
        let start_index = self.index;
        if self.is_over() {
            self.state = State::Over;
            return Item::new(ItemType::Eof, "", start_index);
        }

        self.index += 2;
        self.state = State::Text;

        Item::new(ItemType::VariableEnd, RIGHT_VARIABLE_DELIM, start_index)
    }

    fn lex_text(&mut self) -> Item {
        let start_index = self.index;
        if self.is_over() {
            self.state = State::Over;
            return Item::new(ItemType::Eof, "", start_index);
        }

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

        Item::new(ItemType::Text, self.get_substring(start_index), start_index)
    }

    // We know we have a space, we need to figure out how many
    fn lex_space(&mut self) -> Item {
        let start_index = self.index;
        if self.is_over() {
            self.state = State::Over;
            return Item::new(ItemType::Eof, "", start_index);
        }

        loop {
            if !self.chars[self.index].1.is_whitespace() {
                break;
            }
            self.index += 1;
        }

        Item::new(ItemType::Space, self.get_substring(start_index), start_index)
    }

    fn lex_number(&mut self) -> Item {
        let start_index = self.index;
        let mut number_type = ItemType::Int;
        let mut is_error = false;

        if self.is_over() {
            self.state = State::Over;
            return Item::new(ItemType::Eof, "", start_index);
        }

        loop {
            match self.chars[self.index].1 {
                x if x.is_whitespace() || x == '}' => break,
                '.' => {
                    match number_type {
                        ItemType::Int => number_type = ItemType::Float,
                        ItemType::Float => {
                            is_error = true;
                            break;
                        },
                        _ => {}
                    }
                },
                x if !x.is_numeric() => {
                    is_error = true;
                    break;
                },
                _ => {}
            }
            self.index += 1;
        }

        if is_error {
            self.state = State::Over;
            return Item::new(
                ItemType::Error,
                &LexerError::InvalidNumber.to_string(),
                start_index
            );
        }

        Item::new(number_type, self.get_substring(start_index), start_index)
    }

    fn lex_variable(&mut self) -> Item {
        let start_index = self.index;

        loop {
            match self.chars[self.index].1 {
                x if x.is_whitespace() || x == '.' => break,
                _ => {}
            }
            self.index += 1;
        }

        Item::new(ItemType::Variable, self.get_substring(start_index), start_index)
    }

    fn lex_operator(&mut self) -> Item {
        let start_index = self.index;
        self.index += 1;
        self.state = State::InsideBlock;
        Item::new(ItemType::Operator, self.get_substring(self.index - 1), start_index)
    }

    fn lex_inside_variable_block(&mut self) -> Item {
        let start_index = self.index;

        if self.is_over() {
            self.state = State::Over;
            return Item::new(
                ItemType::Error,
                &LexerError::UnclosedVariableBlock.to_string(),
                start_index
            )
        }

        match self.chars[self.index].1 {
            x if x.is_whitespace() => self.lex_space(),
            x if x.is_alphabetic() || x == '_' => self.lex_variable(),
            '}' => self.lex_right_variable_delimiter(),
            x if x.is_numeric() => self.lex_number(),
            '*' | '+' | '-' | '/' | '.' => self.lex_operator(),
            '{' => {
                self.state = State::Over;
                Item::new(
                    ItemType::Error,
                    &LexerError::NestedDelimiter.to_string(),
                    start_index
                )
            }
            _ => {
                self.state = State::Over;
                Item::new(
                    ItemType::Error,
                    &LexerError::UnknownChar.to_string(),
                    start_index
                )
            },
        }
    }
}

impl Iterator for Lexer {
  type Item = Item;

    fn next(&mut self) -> Option<Item> {
        // Empty template or we got to the end
        if self.input.len() == 0 {
            return None;
        }

        match self.state {
            State::Text => Some(self.lex_text()),
            State::VariableStart => Some(self.lex_left_variable_delimiter()),
            State::InsideBlock => Some(self.lex_inside_variable_block()),
            State::Over => None
        }
    }
}


#[cfg(test)]
mod tests {
    use super::{
        Item,
        ItemType,
        LexerError,
        Lexer,
        LEFT_VARIABLE_DELIM, RIGHT_VARIABLE_DELIM
    };

    macro_rules! pretty_assert_eq {
        ($left:expr , $right:expr) => ({
            match (&($left), &($right)) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        panic!("assertion failed: `(left == right)` \
                         (left: `{:#?}`, right: `{:#?}`)", left_val, right_val)
                    }
                }
            }
        })
    }

    fn text_item(value: &str, position: usize) -> Item {
        Item::new(ItemType::Text, value, position)
    }

    fn variable_item(value: &str, position: usize) -> Item {
        Item::new(ItemType::Variable, value, position)
    }

    fn variable_start_item(position: usize) -> Item {
        Item::new(ItemType::VariableStart, "{{", position)
    }

    fn variable_end_item(position: usize) -> Item {
        Item::new(ItemType::VariableEnd, "}}", position)
    }

    fn space_item(value: &str, position: usize) -> Item {
        Item::new(ItemType::Space, value, position)
    }

    fn int_item(value: &str, position: usize) -> Item {
        Item::new(ItemType::Int, value, position)
    }

    fn float_item(value: &str, position: usize) -> Item {
        Item::new(ItemType::Float, value, position)
    }

    fn operator_item(value: &str, position: usize) -> Item {
        Item::new(ItemType::Operator, value, position)
    }

    fn error_item(value: &str, position: usize) -> Item {
        Item::new(ItemType::Error, value, position)
    }

    fn eof_item(position: usize) -> Item {
        Item::new(ItemType::Eof, "", position)
    }

    #[test]
    fn test_empty() {
        let items: Vec<Item> = Lexer::new("").collect();
        let expected = vec![];
        pretty_assert_eq!(items, expected);
    }

    #[test]
    fn test_only_text() {
        let items: Vec<Item> = Lexer::new("Hello 世界").collect();
        let expected = vec![text_item("Hello 世界", 0), eof_item(7)];
        pretty_assert_eq!(items, expected);
    }

    #[test]
    fn test_text_new_line() {
        let items: Vec<Item> = Lexer::new("Hello\n 世界").collect();
        let expected = vec![text_item("Hello\n 世界", 0), eof_item(8)];
        pretty_assert_eq!(items, expected);
    }

    #[test]
    fn test_variable_block_and_text() {
        let items: Vec<Item> = Lexer::new("{{ greeting }} 世界").collect();
        let expected = vec![
            variable_start_item(0),
            space_item(" ",2),
            variable_item("greeting", 3),
            space_item(" ",11),
            variable_end_item(12),
            text_item(" 世界", 14),
            eof_item(16)
        ];
        pretty_assert_eq!(items, expected);
    }

    #[test]
    fn test_unclosed_opening_variable_delimiter() {
        let items: Vec<Item> = Lexer::new("{{").collect();
        let expected = vec![
            variable_start_item(0),
            error_item(&LexerError::UnclosedVariableBlock.to_string(), 2),
        ];
        pretty_assert_eq!(items, expected);
    }

    #[test]
    fn test_invalid_nested_variable_delimiter() {
        let items: Vec<Item> = Lexer::new("{{{{").collect();
        let expected = vec![
            variable_start_item(0),
            error_item(&LexerError::NestedDelimiter.to_string(), 2),
        ];
        pretty_assert_eq!(items, expected);
    }

    #[test]
    fn test_numbers() {
        let items: Vec<Item> = Lexer::new("{{1 3.14  }}").collect();
        let expected = vec![
            variable_start_item(0),
            int_item("1", 2),
            space_item(" ",3),
            float_item("3.14", 4),
            space_item("  ",8),
            variable_end_item(10),
            eof_item(12)
        ];
        pretty_assert_eq!(items, expected);
    }

    #[test]
    fn test_invalid_numbers() {
        let items: Vec<Item> = Lexer::new("{{1up}}").collect();
        let expected = vec![
            variable_start_item(0),
            error_item(&LexerError::InvalidNumber.to_string(), 2),
        ];
        pretty_assert_eq!(items, expected);
    }

    #[test]
    fn test_operators() {
        let items: Vec<Item> = Lexer::new("{{+ - * / .}}").collect();
        let expected = vec![
            variable_start_item(0),
            operator_item("+", 2),
            space_item(" ", 3),
            operator_item("-", 4),
            space_item(" ", 5),
            operator_item("*", 6),
            space_item(" ", 7),
            operator_item("/", 8),
            space_item(" ", 9),
            operator_item(".", 10),
            variable_end_item(11),
            eof_item(13)
        ];
        pretty_assert_eq!(items, expected);
    }
}
