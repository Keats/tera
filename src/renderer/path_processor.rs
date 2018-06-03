use errors::{Result, ResultExt};
use renderer::ref_or_owned::RefOrOwned;
use std::result;

/// Character Types to discern when parsing lookup identifiers
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

/// Matches on single `char` classifying its `CharType`
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

/// Trait to separate requirements of a path processor from its processing.
///
/// Path processing: Parsing and looking up the values for
/// rendering as well as indexing into those values.
///
/// Sample Paths Strings
///     - foo
///     - foo.bar
///     - foo.bar[goo]
///     - foo.0.1
///     - foo['bar']
///
/// Given a path string, the `process_path` will parse the path performing
/// required lookups along the way. A simple bareword `foo` as a path means
/// look in the `Value` for `goo`. A dotted path means look sequentially
/// in for each bareword in path. For example, `foo.bar.boo` means
/// lookup `foo` then look in `foo` for `bar`, then look in `bar` for `boo`.
///
/// Open brackets also indicate a desire to look in a `Value` for another
/// `Value`. However, it may recursively require another lookup. For example,
/// `foo[bar]` means, lookup `bar` and use to index into `foo`.
///
///
pub trait Accessor<'a, Node: 'a> {
    /// Look up `key` and return the `Result<Node>`.
    fn lookup(&self, key: &str) -> Result<Node>;

    /// Given `node` index into it with `pointer`.
    ///
    /// For example: foo['goo'] means
    ///   - Lookup `foo`
    ///   - Index into `foo` with `goo` using `index_pointer`
    ///
    fn index_pointer(&self, node: Node, pointer: &str) -> Result<Node>;

    /// Given `node`, index into it with `index_node`
    ///
    /// For example: foo[goo] means
    ///   - Lookup `foo` node
    ///   - Lookup `goo`. Turn resulting `Node` into `String`
    ///   - Index into `foo` node with that `String`
    fn index_by_node(&self, node: Node, index_node: Node, index_text: &str) -> Result<Node>;
}

/// Parses `path`, performing required lookups and indexing
#[inline]
pub fn process_path<'a, Node: 'a, A: 'a>(path: &str, accessor: &A) -> Result<Node>
where
    A: Accessor<'a, Node>,
{
    if path.is_empty() {
        bail!("Empty path")
    } else {
        let (node, rest) = process(path, accessor)?;
        debug_assert!(rest.is_empty());
        Ok(node)
    }
}

/// Indexes into `node` with `index_str` and returns indexed `Node` and remaining text
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
                            node = accessor.index_by_node(node, index_node, index_str)?;
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

/// Parses `path`, performing required lookups and indexing
///
/// Starting point for recursive processing of path
#[inline]
fn process<'a, 'b, Node: 'a, A: 'a>(path: &'b str, accessor: &A) -> Result<(Node, &'b str)>
where
    A: Accessor<'a, Node>,
{
    let c = first_char(path);
    match char_type(c) {
        CharType::Alpha => {
            let (ident, rest) = eat_to_bracket(path);
            debug_assert!(!ident.is_empty());
            process_node_index(accessor.lookup(ident)?, rest, accessor)
        }
        _ => bail!(format!("Paths may not begin with `{}` -> `{}`", c, path)),
    }
}

#[cfg(test)]
mod tests {
    extern crate serde_json;
    use super::*;
    use serde_json::Value;

    /// Converts a dotted path to a json pointer one
    #[inline]
    pub fn get_json_pointer(key: &str) -> String {
        ["/", &key.replace(".", "/")].join("")
    }

    struct TestAccessor {
        value: Value,
    }

    impl TestAccessor {
        fn new() -> TestAccessor {
            TestAccessor {
                value: deep_object(),
            }
        }
    }

    impl<'a> Accessor<'a, Value> for TestAccessor {
        fn lookup(&self, s: &str) -> Result<Value> {
            match self.value.pointer(&get_json_pointer(s)).map(|v| v.clone()) {
                Some(found) => Ok(found),
                None => bail!("Could not find `{}`", s),
            }
        }

        fn index_pointer(&self, value: Value, pointer: &str) -> Result<Value> {
            match value.pointer(&get_json_pointer(pointer)).map(|v| v.clone()) {
                Some(found) => Ok(found),
                None => bail!("Could not index `{}` into value `{:?}`", pointer, value),
            }
        }

        fn index_by_node(&self, node: Value, index: Value, index_text: &str) -> Result<Value> {
            let index_str = index.as_str().expect("Need string for test");
            self.index_pointer(node, index_str)
        }
    }

    #[test]
    fn test_path_bareword() {
        let sample = &TestAccessor::new();
        assert_eq!(process_path("ab", &TestAccessor::new()).unwrap(), "AB");
        assert_eq!(
            process_path("abcdef", &TestAccessor::new()).unwrap(),
            "ABCDEF"
        );
        assert_eq!(
            sample
                .index_pointer(
                    sample
                        .index_pointer(
                            sample
                                .index_pointer(
                                    sample
                                        .index_pointer(process_path("a", sample).unwrap(), "b")
                                        .unwrap(),
                                    "c"
                                )
                                .unwrap(),
                            "d"
                        )
                        .unwrap(),
                    "e"
                )
                .unwrap(),
            "A.B.C.D.E"
        );
    }

    #[test]
    fn test_path_dotted_word() {
        assert_eq!(
            process_path("a1.a2.a3", &TestAccessor::new()).unwrap(),
            "A1.A2.A3"
        );
    }

    #[test]
    fn test_path_nested() {
        assert_eq!(
            process_path("a1[a2[a3]]", &TestAccessor::new()).unwrap(),
            process_path("a1.a2", &TestAccessor::new()).unwrap()
        );
    }

    #[test]
    fn test_path_serial_indexes() {
        process_path("a[b][c]", &TestAccessor::new());
    }

    #[test]
    fn test_path_numeric_index() {
        assert_eq!(
            process_path("foo.1.moo.0", &TestAccessor::new()).unwrap(),
            "voo"
        );
    }

    #[test]
    fn test_path_numeric_bracket_index() {
        assert_eq!(
            process_path("foo[1]['moo'][0]", &TestAccessor::new()).unwrap(),
            "voo"
        );
    }

    #[test]
    fn test_path_literal_indexes() {
        assert_eq!(
            process_path("a['b']", &TestAccessor::new()).unwrap(),
            process_path("a.b", &TestAccessor::new()).unwrap()
        );

        assert_eq!(
            process_path("a[\"b\"]", &TestAccessor::new()).unwrap(),
            process_path("a.b", &TestAccessor::new()).unwrap()
        );
    }

    // #[test]
    // fn test_path_gnarley() {
    //     assert_eq!(
    //         process_path("a['b'][\"c\"][d][e]", &TestAccessor::new()).unwrap(),
    //         process_path("a['b'][\"c\"][d][e]", &TestAccessor::new()).unwrap()
    //     );
    // }

    #[test]
    fn test_err_paths() {
        assert_eq!(
            process_path(".a", &TestAccessor::new())
                .unwrap_err()
                .to_string(),
            "Paths may not begin with `.` -> `.a`"
        );

        assert_eq!(
            process_path("[abc]", &TestAccessor::new())
                .unwrap_err()
                .to_string(),
            "Paths may not begin with `[` -> `[abc]`"
        );

        assert_eq!(
            process_path("][abc]", &TestAccessor::new())
                .unwrap_err()
                .to_string(),
            "Paths may not begin with `]` -> `][abc]`"
        );

        assert_eq!(
            process_path("'foo'", &TestAccessor::new())
                .unwrap_err()
                .to_string(),
            "Paths may not begin with `'` -> `'foo'`"
        );

        assert_eq!(
            process_path("\"foo\"", &TestAccessor::new())
                .unwrap_err()
                .to_string(),
            "Paths may not begin with `\"` -> `\"foo\"`"
        );

        assert_eq!(
            process_path("42", &TestAccessor::new())
                .unwrap_err()
                .to_string(),
            "Paths may not begin with `4` -> `42`"
        );
    }

    fn deep_object() -> Value {
        let data = r#"{
                    "a": {
                        "b": {
                            "c": {
                                "d": {
                                    "e": "A.B.C.D.E"
                                }
                            }
                        }
                    },
                    "ab":"AB",
                    "abcdef": "ABCDEF",
                    "a1": {
                        "a2": {
                            "a3": "A1.A2.A3"
                        }
                    },
                    "a2": {
                        "a3": "a2"
                    },
                    "a3": "a3",
                    "b": "b",
                    "foo": [
                        { "goo": "goo" },
                        { "moo": [ "voo" ] }
                    ]
                  }"#;

        serde_json::from_str(data).unwrap()
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
                } else if text.len() > i + 1 {
                    pointer = &text[(i + 1)..];
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
