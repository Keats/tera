/// Filters operating on string
use std::collections::HashMap;

use serde_json::value::{to_value, Value};
use slug;
use url::percent_encoding::{EncodeSet, utf8_percent_encode};
use regex::{Captures, Regex};

use errors::Result;
use utils;

lazy_static! {
    static ref STRIPTAGS_RE: Regex = Regex::new(r"(<!--.*?-->|<[^>]*>)").unwrap();
    static ref WORDS_RE: Regex = Regex::new(r"\b(?P<first>\w)(?P<rest>\w*)\b").unwrap();
}

/// Convert a value to uppercase.
pub fn upper(value: Value, _: HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("upper", "value", String, value);

    Ok(to_value(&s.to_uppercase()).unwrap())
}

/// Convert a value to lowercase.
pub fn lower(value: Value, _: HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("lower", "value", String, value);

    Ok(to_value(&s.to_lowercase()).unwrap())
}

/// Strip leading and trailing whitespace.
pub fn trim(value: Value, _: HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("trim", "value", String, value);

    Ok(to_value(&s.trim()).unwrap())
}

/// Truncates a string to the indicated length
pub fn truncate(value: Value, args: HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("truncate", "value", String, value);
    let length = match args.get("length") {
        Some(l) => try_get_value!("truncate", "length", usize, l),
        None => 255,
    };

    // Nothing to truncate?
    if length > s.len() {
        return Ok(to_value(&s).unwrap());
    }

    let result = s[..s.char_indices().nth(length).unwrap().0].to_string() + "…";
    Ok(to_value(&result).unwrap())
}

/// Gets the number of words in a string.
pub fn wordcount(value: Value, _: HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("wordcount", "value", String, value);

    Ok(to_value(&s.split_whitespace().count()).unwrap())
}

/// Replaces given `from` substring with `to` string.
pub fn replace(value: Value, args: HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("replace", "value", String, value);

    let from = match args.get("from") {
        Some(val) => try_get_value!("replace", "from", String, val),
        None => bail!("Filter `replace` expected an arg called `from`"),
    };

    let to = match args.get("to") {
        Some(val) => try_get_value!("replace", "to", String, val),
        None => bail!("Filter `replace` expected an arg called `to`"),
    };

    Ok(to_value(&s.replace(&from, &to)).unwrap())
}

/// First letter of the string is uppercase rest is lowercase
pub fn capitalize(value: Value, _: HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("capitalize", "value", String, value);
    let mut chars = s.chars();
    match chars.next() {
        None => Ok(to_value("").unwrap()),
        Some(f) => {
            let res = f.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase();
            Ok(to_value(&res).unwrap())
        }
    }
}

#[derive(Clone)]
struct UrlEncodeSet(String);

impl UrlEncodeSet {
    fn safe_bytes(&self) -> &[u8] {
        let &UrlEncodeSet(ref safe) = self;
        safe.as_bytes()
    }
}

impl EncodeSet for UrlEncodeSet {
    fn contains(&self, byte: u8) -> bool {
        if byte >= 48 && byte <= 57 {
            // digit
            false
        } else if byte >= 65 && byte <= 90 {
            // uppercase character
            false
        } else if byte >= 97 && byte <= 122 {
            // lowercase character
            false
        } else if byte == 45 || byte == 46 || byte == 95 {
            // -, . or _
            false
        } else {
            !self.safe_bytes().contains(&byte)
        }
    }
}

/// Percent-encodes reserved URI characters
pub fn urlencode(value: Value, args: HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("urlencode", "value", String, value);
    let safe = match args.get("safe") {
        Some(l) => try_get_value!("urlencode", "safe", String, l),
        None => "/".to_string(),
    };

    let encoded = utf8_percent_encode(s.as_str(), UrlEncodeSet(safe)).collect::<String>();
    Ok(to_value(&encoded).unwrap())
}

/// Escapes quote characters
pub fn addslashes(value: Value, _: HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("addslashes", "value", String, value);
    Ok(to_value(&s.replace("\\", "\\\\")
        .replace("\"", "\\\"")
        .replace("\'", "\\\'"))
        .unwrap())
}

/// Transform a string into a slug
pub fn slugify(value: Value, _: HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("slugify", "value", String, value);
    Ok(to_value(&slug::slugify(s)).unwrap())
}

/// Capitalizes each word in the string
pub fn title(value: Value, _: HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("title", "value", String, value);

    Ok(to_value(&WORDS_RE.replace_all(&s, |caps: &Captures| {
        let first = caps["first"].to_uppercase();
        let rest = caps["rest"].to_lowercase();
        format!("{}{}", first, rest)
    })).unwrap())
}

/// Removes html tags from string
pub fn striptags(value: Value, _: HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("striptags", "value", String, value);
    Ok(to_value(&STRIPTAGS_RE.replace_all(&s, "")).unwrap())
}

/// Returns the given text with ampersands, quotes and angle brackets encoded
/// for use in HTML.
pub fn escape_html(value: Value, _: HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("escape_html", "value", String, value);
    Ok(to_value(utils::escape_html(&s)).unwrap())
}

/// Split the given string by the given pattern.
pub fn split(value: Value, args: HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("split", "value", String, value);

    let pat = match args.get("pat") {
        Some(pat) => try_get_value!("split", "pat", String, pat),
        None => bail!("Filter `split` expected an arg called `pat`"),
    };

    Ok(to_value(s.split(&pat).collect::<Vec<_>>()).unwrap())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serde_json::value::to_value;

    use super::*;

    #[test]
    fn test_upper() {
        let result = upper(to_value("hello").unwrap(), HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("HELLO").unwrap());
    }

    #[test]
    fn test_upper_error() {
        let result = upper(to_value(&50).unwrap(), HashMap::new());
        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap().description(),
            "Filter `upper` was called on an incorrect value: got `50` but expected a String"
        );
    }

    #[test]
    fn test_trim() {
        let result = trim(to_value("  hello  ").unwrap(), HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("hello").unwrap());
    }

    #[test]
    fn test_truncate_smaller_than_length() {
        let mut args = HashMap::new();
        args.insert("length".to_string(), to_value(&255).unwrap());
        let result = truncate(to_value("hello").unwrap(), args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("hello").unwrap());
    }

    #[test]
    fn test_truncate_when_required() {
        let mut args = HashMap::new();
        args.insert("length".to_string(), to_value(&2).unwrap());
        let result = truncate(to_value("日本語").unwrap(), args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("日本…").unwrap());
    }

    #[test]
    fn test_lower() {
        let result = lower(to_value("HELLO").unwrap(), HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("hello").unwrap());
    }

    #[test]
    fn test_wordcount() {
        let result = wordcount(to_value("Joel is a slug").unwrap(), HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(&4).unwrap());
    }

    #[test]
    fn test_replace() {
        let mut args = HashMap::new();
        args.insert("from".to_string(), to_value(&"Hello").unwrap());
        args.insert("to".to_string(), to_value(&"Goodbye").unwrap());
        let result = replace(to_value(&"Hello world!").unwrap(), args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("Goodbye world!").unwrap());
    }

    #[test]
    fn test_replace_missing_arg() {
        let mut args = HashMap::new();
        args.insert("from".to_string(), to_value(&"Hello").unwrap());
        let result = replace(to_value(&"Hello world!").unwrap(), args);
        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap().description(),
            "Filter `replace` expected an arg called `to`"
        );
    }

    #[test]
    fn test_capitalize() {
        let tests = vec![
            ("CAPITAL IZE", "Capital ize"),
            ("capital ize", "Capital ize"),
        ];
        for (input, expected) in tests {
            let result = capitalize(to_value(input).unwrap(), HashMap::new());
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), to_value(expected).unwrap());
        }
    }

    #[test]
    fn test_addslashes() {
        let tests = vec![
            (r#"I'm so happy"#, r#"I\'m so happy"#),
            (r#"Let "me" help you"#, r#"Let \"me\" help you"#),
            (r#"<a>'"#, r#"<a>\'"#),
            (
                r#""double quotes" and \'single quotes\'"#,
                r#"\"double quotes\" and \\\'single quotes\\\'"#,
            ),
            (r#"\ : backslashes too"#, r#"\\ : backslashes too"#),
        ];
        for (input, expected) in tests {
            let result = addslashes(to_value(input).unwrap(), HashMap::new());
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), to_value(expected).unwrap());
        }
    }

    #[test]
    fn test_slugify() {
        // slug crate already has tests for general slugification so we just
        // check our function works
        let tests = vec![
            (r#"Hello world"#, r#"hello-world"#),
            (r#"Hello 世界"#, r#"hello-shi-jie"#),
        ];
        for (input, expected) in tests {
            let result = slugify(to_value(input).unwrap(), HashMap::new());
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), to_value(expected).unwrap());
        }
    }

    #[test]
    fn test_urlencode() {
        let tests = vec![
            (
                r#"https://www.example.org/foo?a=b&c=d"#,
                None,
                r#"https%3A//www.example.org/foo%3Fa%3Db%26c%3Dd"#,
            ),
            (
                r#"https://www.example.org/"#,
                Some(""),
                r#"https%3A%2F%2Fwww.example.org%2F"#,
            ),
            (r#"/test&"/me?/"#, None, r#"/test%26%22/me%3F/"#),
            (r#"escape/slash"#, Some(""), r#"escape%2Fslash"#),
        ];
        for (input, safe, expected) in tests {
            let mut args = HashMap::new();
            if let Some(safe) = safe {
                args.insert("safe".to_string(), to_value(&safe).unwrap());
            }
            let result = urlencode(to_value(input).unwrap(), args);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), to_value(expected).unwrap());
        }
    }

    #[test]
    fn test_title() {
        let tests = vec![
            ("foo bar", "Foo Bar"),
            ("foo\tbar", "Foo\tBar"),
            ("foo  bar", "Foo  Bar"),
            ("f bar f", "F Bar F"),
            ("foo-bar", "Foo-Bar"),
            ("FOO\tBAR", "Foo\tBar"),
            ("foo (bar)", "Foo (Bar)"),
            ("foo (bar) ", "Foo (Bar) "),
            ("foo {bar}", "Foo {Bar}"),
            ("foo [bar]", "Foo [Bar]"),
            ("foo <bar>", "Foo <Bar>"),
            ("  foo  bar", "  Foo  Bar"),
            ("\tfoo\tbar\t", "\tFoo\tBar\t"),
            ("foo bar ", "Foo Bar "),
            ("foo bar\t", "Foo Bar\t"),
        ];
        for (input, expected) in tests {
            let result = title(to_value(input).unwrap(), HashMap::new());
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), to_value(expected).unwrap());
        }
    }

    #[test]
    fn test_striptags() {
        let tests = vec![
            (r"<b>Joel</b> <button>is</button> a <span>slug</span>", "Joel is a slug"),
            (r#"<p>just a small   \n <a href="x"> example</a> link</p>\n<p>to a webpage</p><!-- <p>and some commented stuff</p> -->"#,
            r#"just a small   \n  example link\nto a webpage"#),
            (r"<p>See: &#39;&eacute; is an apostrophe followed by e acute</p>",r"See: &#39;&eacute; is an apostrophe followed by e acute"),
            (r"<adf>a", "a"),
            (r"</adf>a", "a"),
            (r"<asdf><asdf>e", "e"),
            (r"hi, <f x", "hi, <f x"),
            ("234<235, right?", "234<235, right?"),
            ("a4<a5 right?", "a4<a5 right?"),
            ("b7>b2!", "b7>b2!"),
            ("</fe", "</fe"),
            ("<x>b<y>", "b"),
            (r#"a<p a >b</p>c"#, "abc"),
            (r#"d<a:b c:d>e</p>f"#, "def"),
            (r#"<strong>foo</strong><a href="http://example.com">bar</a>"#, "foobar"),
        ];
        for (input, expected) in tests {
            let result = striptags(to_value(input).unwrap(), HashMap::new());
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), to_value(expected).unwrap());
        }
    }

    #[test]
    fn test_split() {
        let tests: Vec<(_, _, &[&str])> = vec![
            ("a/b/cde", "/", &["a", "b", "cde"]),
            ("hello, world", ", ", &["hello", "world"]),
        ];
        for (input, pat, expected) in tests {
            let mut args = HashMap::new();
            args.insert("pat".to_string(), to_value(pat).unwrap());
            let result = split(to_value(input).unwrap(), args).unwrap();
            let result = result.as_array().unwrap();
            assert_eq!(result.len(), expected.len());
            for (result, expected) in result.iter().zip(expected.iter()) {
                assert_eq!(result, expected);
            }
        }
    }
}
