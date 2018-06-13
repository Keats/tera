use errors::{Result, ResultExt};
use renderer::ref_or_owned::RefOrOwned;
use std::result;

#[derive(Debug, PartialEq)]
enum CharType {
    Numeric,
    Alpha,
    Dot,
    DoubleQuote,
    SingleQuote,
    Open,
    Close,
    Unexpected(char),
}

#[inline]
fn char_type(c: char) -> CharType {
    match c {
        '.' => CharType::Dot,
        '"' => CharType::DoubleQuote,
        '\'' => CharType::SingleQuote,
        '[' => CharType::Open,
        ']' => CharType::Close,
        _ => {
            if c.is_numeric() {
                CharType::Numeric
            } else if c.is_alphabetic() {
                CharType::Alpha
            } else {
                CharType::Unexpected(c)
            }
        }
    }
}

pub trait Accessor<'a, Node: 'a> {
    fn lookup(&self, key: &str) -> Result<Node>;
    fn index_pointer(&self, node: Node, pointer: &str) -> Result<Node>;
    fn index_by_node(&self, node: Node, index_node: Node) -> Result<Node>;
}

pub fn process_path<'a, Node: 'a, A: 'a>(s: &str, accessor: &A) -> Result<Node>
where
    A: Accessor<'a, Node>,
{
    if s.is_empty() {
        bail!("Empty path")
    } else {
        let (node, rest) = process(s, accessor)?;
        debug_assert!(rest.is_empty());
        Ok(node)
    }
}

fn process_node_index<'a, 'b, Node: 'a, A: 'a>(
    mut node: Node,
    index_str: &'b str,
    accessor: &A,
) -> Result<(Node, &'b str)>
where
    A: Accessor<'a, Node>,
{
    let mut rest = index_str;

    if !index_str.is_empty() {
        match char_type(first_char(rest)) {
            CharType::Open => {
                rest = &rest[1..];
                if rest.is_empty() {
                    bail!(format!("Index ends early `{}`", index_str))
                } else {
                    match char_type(first_char(rest)) {
                        CharType::SingleQuote | CharType::DoubleQuote => {
                            // Special case the string literals
                            let (literal, remaining) = eat_string(rest)?;
                            node = accessor.index_pointer(node, literal)?;
                            rest = &remaining[1..];
                        }
                        CharType::Numeric => {
                            // Special numbers
                            let (literal, remaining) = eat_number(rest);
                            node = accessor.index_pointer(node, literal)?;
                            rest = &remaining[1..];
                        }
                        _ => {
                            let (index_node, remaining) = process(rest, accessor)?;
                            node = accessor.index_by_node(node, index_node)?;
                            rest = remaining;
                        }
                    }
                }
            }
            CharType::Close => {
                rest = &rest[1..];
            }
            _ => bail!(format!(
                "Index operations must begin with `[` -> `{}`",
                index_str
            )),
        }

        process_node_index(node, rest, accessor)
    } else {
        Ok((node, rest))
    }
}

fn process<'a, 'b, Node: 'a, A: 'a>(s: &'b str, accessor: &A) -> Result<(Node, &'b str)>
where
    A: Accessor<'a, Node>,
{
    let c = first_char(s);
    match char_type(c) {
        CharType::Alpha => {
            let (ident, rest) = eat_to_bracket(s);
            debug_assert!(!ident.is_empty());
            process_node_index(accessor.lookup(ident)?, rest, accessor)
        }
        _ => bail!(format!("Paths may not begin with `{}` -> `{}`", c, s)),
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    struct TestAccessor {}

    impl<'a> Accessor<'a, i32> for TestAccessor {
        fn lookup(&self, s: &str) -> Result<i32> {
            println!("Lookup `{}`", s);
            Ok(42)
        }

        fn index_pointer(&self, i: i32, pointer: &str) -> Result<i32> {
            println!("Index pointer `{}` into i(`{}`)", pointer, i);
            Ok(45)
        }

        fn index_by_node(&self, node: i32, index: i32) -> Result<i32> {
            println!("Index into node(`{}`) by node(`{}`)", node, index);
            Ok(43)
        }
    }

    #[test]
    fn test_bareword() {
        process_path("a", &TestAccessor {});
        process_path("ab", &TestAccessor {});
        process_path("abcdef", &TestAccessor {});
    }

    #[test]
    fn test_dotted_word() {
        process_path("ab.c.def", &TestAccessor {});
    }

    #[test]
    fn test_nested() {
        process_path("a1[a2[a3]]", &TestAccessor {});
    }

    #[test]
    fn test_serial_indexes() {
        process_path("a[b][c]", &TestAccessor {});
    }

    #[test]
    fn test_literal_indexes() {
        process_path("a['b']", &TestAccessor {}).unwrap();
        process_path("a[\"b\"]", &TestAccessor {});
    }

    #[test]
    fn test_gnarley() {
        process_path("a['b'][\"c\"][d][e]", &TestAccessor {}).unwrap();
    }

    #[test]
    fn test_err_paths() {
        //TODO
        /*
        assert_eq!(
            process_path(".a", &TestAccessor {}),
            bail!("Paths may not begin with `.` -> `.a`")
        );

        assert_eq!(
            process_path("[abc]", &TestAccessor {}),
            bail!("Paths may not begin with `[` -> `[abc]`")
        );

        assert_eq!(
            process_path("][abc]", &TestAccessor {}),
            bail!("Paths may not begin with `]` -> `][abc]`")
        );

        assert_eq!(
            process_path("'foo'", &TestAccessor {}),
            bail!("Paths may not begin with `'` -> `'foo'`")
        );

        assert_eq!(
            process_path("\"foo\"", &TestAccessor {}),
            bail!("Paths may not begin with `\"` -> `\"foo\"`")
        );

        assert_eq!(
            process_path("42", &TestAccessor {}),
            bail!("Paths may not begin with `4` -> `42`")
        );
        */
    }

}

#[inline]
fn first_char(s: &str) -> char {
    debug_assert!(!s.is_empty());
    s.chars().next().unwrap()
}

#[inline]
fn eat_to_bracket(text: &str) -> (&str, &str) {
    debug_assert!(text.is_empty() || first_char(text).is_alphabetic());
    for (i, c) in text.chars().enumerate() {
        if c == '[' || c == ']' {
            return (&text[0..i], &text[i..]);
        }
        debug_assert!(c.is_alphabetic() || c.is_numeric() || c == '.' || c == '_');
    }
    (text, "")
}

#[inline]
fn eat_string(text: &str) -> Result<(&str, &str)> {
    if let Some(start) = text.find('"') {
        if start == 0 {
            return match_closing(&text[start + 1..], '"');
        }
    }

    if let Some(start) = text.find('\'') {
        if start == 0 {
            return match_closing(&text[start + 1..], '\'');
        }
    }

    bail!(format!("Non-terminating string in path: `{}`", text))
}

#[inline]
fn eat_number(text: &str) -> (&str, &str) {
    for (i, c) in text.chars().enumerate() {
        if !c.is_numeric() {
            return (&text[0..i], &text[i..]);
        }
    }
    ("", text)
}

#[inline]
fn match_closing(text: &str, closing: char) -> Result<(&str, &str)> {
    if !text.is_empty() {
        let mut prev = first_char(text);
        for (i, c) in text.chars().enumerate() {
            if c == closing && prev != '\\' {
                return Ok((&text[0..i], &text[(i + 1)..]));
            }
            prev = c;
        }
    }
    bail!(format!("Missing matching `{}` in `{}`:", closing, text))
}

#[inline]
fn match_dot_pointer(text: &str) -> (&str, &str, &str) {
    let mut ident = text;
    let mut pointer = "";

    for (i, c) in text.chars().enumerate() {
        match c {
            '.' | '[' | ']' => {
                ident = &text[0..i];

                if c == ']' || c == '[' {
                    return (ident, pointer, &text[i..]);
                } else {
                    if text.len() > i + 1 {
                        pointer = &text[(i + 1)..];
                    }
                }
                break;
            }
            _ => (),
        }
    }

    for (i, c) in pointer.chars().enumerate() {
        match c {
            '[' | ']' => {
                return (ident, &pointer[..i], &pointer[i..]);
            }
            _ => (),
        }
    }

    (ident, pointer, "")
}
