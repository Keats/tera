/// Filters operating on string
use std::collections::HashMap;

use lazy_static::lazy_static;
use regex::{Captures, Regex};
use serde_json::value::{to_value, Value};
use unic_segment::GraphemeIndices;

#[cfg(feature = "urlencode")]
use percent_encoding::{percent_encode, AsciiSet, NON_ALPHANUMERIC};

use crate::errors::{Error, Result};
use crate::utils;

/// https://url.spec.whatwg.org/#fragment-percent-encode-set
#[cfg(feature = "urlencode")]
const FRAGMENT_ENCODE_SET: &AsciiSet =
    &percent_encoding::CONTROLS.add(b' ').add(b'"').add(b'<').add(b'>').add(b'`');

/// https://url.spec.whatwg.org/#path-percent-encode-set
#[cfg(feature = "urlencode")]
const PATH_ENCODE_SET: &AsciiSet = &FRAGMENT_ENCODE_SET.add(b'#').add(b'?').add(b'{').add(b'}');

/// https://url.spec.whatwg.org/#userinfo-percent-encode-set
#[cfg(feature = "urlencode")]
const USERINFO_ENCODE_SET: &AsciiSet = &PATH_ENCODE_SET
    .add(b'/')
    .add(b':')
    .add(b';')
    .add(b'=')
    .add(b'@')
    .add(b'[')
    .add(b'\\')
    .add(b']')
    .add(b'^')
    .add(b'|');

/// Same as Python quote
/// https://github.com/python/cpython/blob/da27d9b9dc44913ffee8f28d9638985eaaa03755/Lib/urllib/parse.py#L787
/// with `/` not escaped
#[cfg(feature = "urlencode")]
const PYTHON_ENCODE_SET: &AsciiSet = &USERINFO_ENCODE_SET
    .remove(b'/')
    .add(b':')
    .add(b'?')
    .add(b'#')
    .add(b'[')
    .add(b']')
    .add(b'@')
    .add(b'!')
    .add(b'$')
    .add(b'&')
    .add(b'\'')
    .add(b'(')
    .add(b')')
    .add(b'*')
    .add(b'+')
    .add(b',')
    .add(b';')
    .add(b'=');

lazy_static! {
    static ref STRIPTAGS_RE: Regex = Regex::new(r"(<!--.*?-->|<[^>]*>)").unwrap();
    static ref WORDS_RE: Regex = Regex::new(r"\b(?P<first>[\w'])(?P<rest>[\w']*)\b").unwrap();
    static ref SPACELESS_RE: Regex = Regex::new(r">\s+<").unwrap();
}

/// Convert a value to uppercase.
pub fn upper(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("upper", "value", String, value);

    Ok(to_value(s.to_uppercase()).unwrap())
}

/// Convert a value to lowercase.
pub fn lower(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("lower", "value", String, value);

    Ok(to_value(s.to_lowercase()).unwrap())
}

/// Strip leading and trailing whitespace.
pub fn trim(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("trim", "value", String, value);

    Ok(to_value(s.trim()).unwrap())
}

/// Strip leading whitespace.
pub fn trim_start(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("trim_start", "value", String, value);

    Ok(to_value(s.trim_start()).unwrap())
}

/// Strip trailing whitespace.
pub fn trim_end(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("trim_end", "value", String, value);

    Ok(to_value(s.trim_end()).unwrap())
}

/// Strip leading characters that match the given pattern.
pub fn trim_start_matches(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("trim_start_matches", "value", String, value);

    let pat = match args.get("pat") {
        Some(pat) => {
            let p = try_get_value!("trim_start_matches", "pat", String, pat);
            // When reading from a file, it will escape `\n` to `\\n` for example so we need
            // to replace double escape. In practice it might cause issues if someone wants to split
            // by `\\n` for real but that seems pretty unlikely
            p.replace("\\n", "\n").replace("\\t", "\t")
        }
        None => return Err(Error::msg("Filter `trim_start_matches` expected an arg called `pat`")),
    };

    Ok(to_value(s.trim_start_matches(&pat)).unwrap())
}

/// Strip trailing characters that match the given pattern.
pub fn trim_end_matches(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("trim_end_matches", "value", String, value);

    let pat = match args.get("pat") {
        Some(pat) => {
            let p = try_get_value!("trim_end_matches", "pat", String, pat);
            // When reading from a file, it will escape `\n` to `\\n` for example so we need
            // to replace double escape. In practice it might cause issues if someone wants to split
            // by `\\n` for real but that seems pretty unlikely
            p.replace("\\n", "\n").replace("\\t", "\t")
        }
        None => return Err(Error::msg("Filter `trim_end_matches` expected an arg called `pat`")),
    };

    Ok(to_value(s.trim_end_matches(&pat)).unwrap())
}

/// Truncates a string to the indicated length.
///
/// # Arguments
///
/// * `value`   - The string that needs to be truncated.
/// * `args`    - A set of key/value arguments that can take the following
///   keys.
/// * `length`  - The length at which the string needs to be truncated. If
///   the length is larger than the length of the string, the string is
///   returned untouched. The default value is 255.
/// * `end`     - The ellipsis string to be used if the given string is
///   truncated. The default value is "…".
///
/// # Remarks
///
/// The return value of this function might be longer than `length`: the `end`
/// string is *added* after the truncation occurs.
///
pub fn truncate(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("truncate", "value", String, value);
    let length = match args.get("length") {
        Some(l) => try_get_value!("truncate", "length", usize, l),
        None => 255,
    };
    let end = match args.get("end") {
        Some(l) => try_get_value!("truncate", "end", String, l),
        None => "…".to_string(),
    };

    let graphemes = GraphemeIndices::new(&s).collect::<Vec<(usize, &str)>>();

    // Nothing to truncate?
    if length >= graphemes.len() {
        return Ok(to_value(&s).unwrap());
    }

    let result = s[..graphemes[length].0].to_string() + &end;
    Ok(to_value(result).unwrap())
}

/// Gets the number of words in a string.
pub fn wordcount(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("wordcount", "value", String, value);

    Ok(to_value(s.split_whitespace().count()).unwrap())
}

/// Replaces given `from` substring with `to` string.
pub fn replace(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("replace", "value", String, value);

    let from = match args.get("from") {
        Some(val) => try_get_value!("replace", "from", String, val),
        None => return Err(Error::msg("Filter `replace` expected an arg called `from`")),
    };

    let to = match args.get("to") {
        Some(val) => try_get_value!("replace", "to", String, val),
        None => return Err(Error::msg("Filter `replace` expected an arg called `to`")),
    };

    Ok(to_value(s.replace(&from, &to)).unwrap())
}

/// First letter of the string is uppercase rest is lowercase
pub fn capitalize(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("capitalize", "value", String, value);
    let mut chars = s.chars();
    match chars.next() {
        None => Ok(to_value("").unwrap()),
        Some(f) => {
            let res = f.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase();
            Ok(to_value(res).unwrap())
        }
    }
}

/// Percent-encodes reserved URI characters
#[cfg(feature = "urlencode")]
pub fn urlencode(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("urlencode", "value", String, value);
    let encoded = percent_encode(s.as_bytes(), PYTHON_ENCODE_SET).to_string();
    Ok(Value::String(encoded))
}

/// Percent-encodes all non-alphanumeric characters
#[cfg(feature = "urlencode")]
pub fn urlencode_strict(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("urlencode_strict", "value", String, value);
    let encoded = percent_encode(s.as_bytes(), NON_ALPHANUMERIC).to_string();
    Ok(Value::String(encoded))
}

/// Escapes quote characters
pub fn addslashes(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("addslashes", "value", String, value);
    Ok(to_value(s.replace('\\', "\\\\").replace('\"', "\\\"").replace('\'', "\\\'")).unwrap())
}

/// Transform a string into a slug
#[cfg(feature = "builtins")]
pub fn slugify(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("slugify", "value", String, value);
    Ok(to_value(slug::slugify(s)).unwrap())
}

/// Capitalizes each word in the string
pub fn title(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("title", "value", String, value);

    Ok(to_value(WORDS_RE.replace_all(&s, |caps: &Captures| {
        let first = caps["first"].to_uppercase();
        let rest = caps["rest"].to_lowercase();
        format!("{}{}", first, rest)
    }))
    .unwrap())
}

/// Convert line breaks (`\n` or `\r\n`) to HTML linebreaks (`<br>`).
///
/// Example: The input "Hello\nWorld" turns into "Hello<br>World".
pub fn linebreaksbr(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("linebreaksbr", "value", String, value);
    Ok(to_value(s.replace("\r\n", "<br>").replace('\n', "<br>")).unwrap())
}

/// Indents a string by the specified width.
///
/// # Arguments
///
/// * `value`   - The string to indent.
/// * `args`    - A set of key/value arguments that can take the following
///   keys.
/// * `prefix`  - The prefix used for indentation. The default value is 4 spaces.
/// * `first`  - True indents the first line.  The default is false.
/// * `blank`  - True indents blank lines.  The default is false.
///
pub fn indent(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("indent", "value", String, value);

    let prefix = match args.get("prefix") {
        Some(p) => try_get_value!("indent", "prefix", String, p),
        None => "    ".to_string(),
    };
    let first = match args.get("first") {
        Some(f) => try_get_value!("indent", "first", bool, f),
        None => false,
    };
    let blank = match args.get("blank") {
        Some(b) => try_get_value!("indent", "blank", bool, b),
        None => false,
    };

    // Attempt to pre-allocate enough space to prevent additional allocations/copies
    let mut out = String::with_capacity(
        s.len() + (prefix.len() * (s.chars().filter(|&c| c == '\n').count() + 1)),
    );
    let mut first_pass = true;

    for line in s.lines() {
        if first_pass {
            if first {
                out.push_str(&prefix);
            }
            first_pass = false;
        } else {
            out.push('\n');
            if blank || !line.trim_start().is_empty() {
                out.push_str(&prefix);
            }
        }
        out.push_str(line);
    }

    Ok(to_value(&out).unwrap())
}

/// Removes html tags from string
pub fn striptags(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("striptags", "value", String, value);
    Ok(to_value(STRIPTAGS_RE.replace_all(&s, "")).unwrap())
}

/// Removes spaces between html tags from string
pub fn spaceless(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("spaceless", "value", String, value);
    Ok(to_value(SPACELESS_RE.replace_all(&s, "><")).unwrap())
}

/// Returns the given text with all special HTML characters encoded
pub fn escape_html(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("escape_html", "value", String, value);
    Ok(Value::String(utils::escape_html(&s)))
}

/// Returns the given text with all special XML characters encoded
/// Very similar to `escape_html`, just a few characters less are encoded
pub fn escape_xml(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("escape_html", "value", String, value);

    let mut output = String::with_capacity(s.len() * 2);
    for c in s.chars() {
        match c {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&apos;"),
            _ => output.push(c),
        }
    }
    Ok(Value::String(output))
}

/// Split the given string by the given pattern.
pub fn split(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("split", "value", String, value);

    let pat = match args.get("pat") {
        Some(pat) => {
            let p = try_get_value!("split", "pat", String, pat);
            // When reading from a file, it will escape `\n` to `\\n` for example so we need
            // to replace double escape. In practice it might cause issues if someone wants to split
            // by `\\n` for real but that seems pretty unlikely
            p.replace("\\n", "\n").replace("\\t", "\t")
        }
        None => return Err(Error::msg("Filter `split` expected an arg called `pat`")),
    };

    Ok(to_value(s.split(&pat).collect::<Vec<_>>()).unwrap())
}

/// Convert the value to a signed integer number
pub fn int(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
    let default = match args.get("default") {
        Some(d) => try_get_value!("int", "default", i64, d),
        None => 0,
    };
    let base = match args.get("base") {
        Some(b) => try_get_value!("int", "base", u32, b),
        None => 10,
    };

    let v = match value {
        Value::String(s) => {
            let s = s.trim();
            let s = match base {
                2 => s.trim_start_matches("0b"),
                8 => s.trim_start_matches("0o"),
                16 => s.trim_start_matches("0x"),
                _ => s,
            };

            match i64::from_str_radix(s, base) {
                Ok(v) => v,
                Err(_) => {
                    if s.contains('.') {
                        match s.parse::<f64>() {
                            Ok(f) => f as i64,
                            Err(_) => default,
                        }
                    } else {
                        default
                    }
                }
            }
        }
        Value::Number(n) => match n.as_f64() {
            Some(f) => f as i64,
            None => match n.as_i64() {
                Some(i) => i,
                None => default,
            },
        },
        _ => return Err(Error::msg("Filter `int` received an unexpected type")),
    };

    Ok(to_value(v).unwrap())
}

/// Convert the value to a floating point number
pub fn float(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
    let default = match args.get("default") {
        Some(d) => try_get_value!("float", "default", f64, d),
        None => 0.0,
    };

    let v = match value {
        Value::String(s) => {
            let s = s.trim();
            s.parse::<f64>().unwrap_or(default)
        }
        Value::Number(n) => match n.as_f64() {
            Some(f) => f,
            None => match n.as_i64() {
                Some(i) => i as f64,
                None => default,
            },
        },
        _ => return Err(Error::msg("Filter `float` received an unexpected type")),
    };

    Ok(to_value(v).unwrap())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serde_json::value::to_value;

    use super::*;

    #[test]
    fn test_upper() {
        let result = upper(&to_value("hello").unwrap(), &HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("HELLO").unwrap());
    }

    #[test]
    fn test_upper_error() {
        let result = upper(&to_value(50).unwrap(), &HashMap::new());
        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap().to_string(),
            "Filter `upper` was called on an incorrect value: got `50` but expected a String"
        );
    }

    #[test]
    fn test_trim() {
        let result = trim(&to_value("  hello  ").unwrap(), &HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("hello").unwrap());
    }

    #[test]
    fn test_trim_start() {
        let result = trim_start(&to_value("  hello  ").unwrap(), &HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("hello  ").unwrap());
    }

    #[test]
    fn test_trim_end() {
        let result = trim_end(&to_value("  hello  ").unwrap(), &HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("  hello").unwrap());
    }

    #[test]
    fn test_trim_start_matches() {
        let tests: Vec<(_, _, _)> = vec![
            ("/a/b/cde/", "/", "a/b/cde/"),
            ("\nhello\nworld\n", "\n", "hello\nworld\n"),
            (", hello, world, ", ", ", "hello, world, "),
        ];
        for (input, pat, expected) in tests {
            let mut args = HashMap::new();
            args.insert("pat".to_string(), to_value(pat).unwrap());
            let result = trim_start_matches(&to_value(input).unwrap(), &args);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), to_value(expected).unwrap());
        }
    }

    #[test]
    fn test_trim_end_matches() {
        let tests: Vec<(_, _, _)> = vec![
            ("/a/b/cde/", "/", "/a/b/cde"),
            ("\nhello\nworld\n", "\n", "\nhello\nworld"),
            (", hello, world, ", ", ", ", hello, world"),
        ];
        for (input, pat, expected) in tests {
            let mut args = HashMap::new();
            args.insert("pat".to_string(), to_value(pat).unwrap());
            let result = trim_end_matches(&to_value(input).unwrap(), &args);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), to_value(expected).unwrap());
        }
    }

    #[test]
    fn test_truncate_smaller_than_length() {
        let mut args = HashMap::new();
        args.insert("length".to_string(), to_value(255).unwrap());
        let result = truncate(&to_value("hello").unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("hello").unwrap());
    }

    #[test]
    fn test_truncate_when_required() {
        let mut args = HashMap::new();
        args.insert("length".to_string(), to_value(2).unwrap());
        let result = truncate(&to_value("日本語").unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("日本…").unwrap());
    }

    #[test]
    fn test_truncate_custom_end() {
        let mut args = HashMap::new();
        args.insert("length".to_string(), to_value(2).unwrap());
        args.insert("end".to_string(), to_value("").unwrap());
        let result = truncate(&to_value("日本語").unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("日本").unwrap());
    }

    #[test]
    fn test_truncate_multichar_grapheme() {
        let mut args = HashMap::new();
        args.insert("length".to_string(), to_value(5).unwrap());
        args.insert("end".to_string(), to_value("…").unwrap());
        let result = truncate(&to_value("👨‍👩‍👧‍👦 family").unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("👨‍👩‍👧‍👦 fam…").unwrap());
    }

    #[test]
    fn test_lower() {
        let result = lower(&to_value("HELLO").unwrap(), &HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("hello").unwrap());
    }

    #[test]
    fn test_wordcount() {
        let result = wordcount(&to_value("Joel is a slug").unwrap(), &HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(4).unwrap());
    }

    #[test]
    fn test_replace() {
        let mut args = HashMap::new();
        args.insert("from".to_string(), to_value("Hello").unwrap());
        args.insert("to".to_string(), to_value("Goodbye").unwrap());
        let result = replace(&to_value("Hello world!").unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("Goodbye world!").unwrap());
    }

    // https://github.com/Keats/tera/issues/435
    #[test]
    fn test_replace_newline() {
        let mut args = HashMap::new();
        args.insert("from".to_string(), to_value("\n").unwrap());
        args.insert("to".to_string(), to_value("<br>").unwrap());
        let result = replace(&to_value("Animal Alphabets\nB is for Bee-Eater").unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("Animal Alphabets<br>B is for Bee-Eater").unwrap());
    }

    #[test]
    fn test_replace_missing_arg() {
        let mut args = HashMap::new();
        args.insert("from".to_string(), to_value("Hello").unwrap());
        let result = replace(&to_value("Hello world!").unwrap(), &args);
        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap().to_string(),
            "Filter `replace` expected an arg called `to`"
        );
    }

    #[test]
    fn test_capitalize() {
        let tests = vec![("CAPITAL IZE", "Capital ize"), ("capital ize", "Capital ize")];
        for (input, expected) in tests {
            let result = capitalize(&to_value(input).unwrap(), &HashMap::new());
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
            let result = addslashes(&to_value(input).unwrap(), &HashMap::new());
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), to_value(expected).unwrap());
        }
    }

    #[cfg(feature = "builtins")]
    #[test]
    fn test_slugify() {
        // slug crate already has tests for general slugification so we just
        // check our function works
        let tests =
            vec![(r#"Hello world"#, r#"hello-world"#), (r#"Hello 世界"#, r#"hello-shi-jie"#)];
        for (input, expected) in tests {
            let result = slugify(&to_value(input).unwrap(), &HashMap::new());
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), to_value(expected).unwrap());
        }
    }

    #[cfg(feature = "urlencode")]
    #[test]
    fn test_urlencode() {
        let tests = vec![
            (
                r#"https://www.example.org/foo?a=b&c=d"#,
                r#"https%3A//www.example.org/foo%3Fa%3Db%26c%3Dd"#,
            ),
            (
                r#"https://www.example.org/apples-&-oranges/"#,
                r#"https%3A//www.example.org/apples-%26-oranges/"#,
            ),
            (r#"https://www.example.org/"#, r#"https%3A//www.example.org/"#),
            (r#"/test&"/me?/"#, r#"/test%26%22/me%3F/"#),
            (r#"escape/slash"#, r#"escape/slash"#),
        ];
        for (input, expected) in tests {
            let args = HashMap::new();
            let result = urlencode(&to_value(input).unwrap(), &args);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), to_value(expected).unwrap());
        }
    }

    #[cfg(feature = "urlencode")]
    #[test]
    fn test_urlencode_strict() {
        let tests = vec![
            (
                r#"https://www.example.org/foo?a=b&c=d"#,
                r#"https%3A%2F%2Fwww%2Eexample%2Eorg%2Ffoo%3Fa%3Db%26c%3Dd"#,
            ),
            (
                r#"https://www.example.org/apples-&-oranges/"#,
                r#"https%3A%2F%2Fwww%2Eexample%2Eorg%2Fapples%2D%26%2Doranges%2F"#,
            ),
            (r#"https://www.example.org/"#, r#"https%3A%2F%2Fwww%2Eexample%2Eorg%2F"#),
            (r#"/test&"/me?/"#, r#"%2Ftest%26%22%2Fme%3F%2F"#),
            (r#"escape/slash"#, r#"escape%2Fslash"#),
        ];
        for (input, expected) in tests {
            let args = HashMap::new();
            let result = urlencode_strict(&to_value(input).unwrap(), &args);
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
            ("foo's bar", "Foo's Bar"),
        ];
        for (input, expected) in tests {
            let result = title(&to_value(input).unwrap(), &HashMap::new());
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), to_value(expected).unwrap());
        }
    }

    #[test]
    fn test_indent_defaults() {
        let args = HashMap::new();
        let result = indent(&to_value("one\n\ntwo\nthree").unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("one\n\n    two\n    three").unwrap());
    }

    #[test]
    fn test_indent_args() {
        let mut args = HashMap::new();
        args.insert("first".to_string(), to_value(true).unwrap());
        args.insert("prefix".to_string(), to_value(" ").unwrap());
        args.insert("blank".to_string(), to_value(true).unwrap());
        let result = indent(&to_value("one\n\ntwo\nthree").unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(" one\n \n two\n three").unwrap());
    }

    #[test]
    fn test_striptags() {
        let tests = vec![
            (r"<b>Joel</b> <button>is</button> a <span>slug</span>", "Joel is a slug"),
            (
                r#"<p>just a small   \n <a href="x"> example</a> link</p>\n<p>to a webpage</p><!-- <p>and some commented stuff</p> -->"#,
                r#"just a small   \n  example link\nto a webpage"#,
            ),
            (
                r"<p>See: &#39;&eacute; is an apostrophe followed by e acute</p>",
                r"See: &#39;&eacute; is an apostrophe followed by e acute",
            ),
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
            let result = striptags(&to_value(input).unwrap(), &HashMap::new());
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), to_value(expected).unwrap());
        }
    }

    #[test]
    fn test_spaceless() {
        let tests = vec![
            ("<p>\n<a>test</a>\r\n </p>", "<p><a>test</a></p>"),
            ("<p>\n<a> </a>\r\n </p>", "<p><a></a></p>"),
            ("<p> </p>", "<p></p>"),
            ("<p> <a>", "<p><a>"),
            ("<p> test</p>", "<p> test</p>"),
            ("<p>\r\n</p>", "<p></p>"),
        ];
        for (input, expected) in tests {
            let result = spaceless(&to_value(input).unwrap(), &HashMap::new());
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), to_value(expected).unwrap());
        }
    }

    #[test]
    fn test_split() {
        let tests: Vec<(_, _, &[&str])> = vec![
            ("a/b/cde", "/", &["a", "b", "cde"]),
            ("hello\nworld", "\n", &["hello", "world"]),
            ("hello, world", ", ", &["hello", "world"]),
        ];
        for (input, pat, expected) in tests {
            let mut args = HashMap::new();
            args.insert("pat".to_string(), to_value(pat).unwrap());
            let result = split(&to_value(input).unwrap(), &args).unwrap();
            let result = result.as_array().unwrap();
            assert_eq!(result.len(), expected.len());
            for (result, expected) in result.iter().zip(expected.iter()) {
                assert_eq!(result, expected);
            }
        }
    }

    #[test]
    fn test_xml_escape() {
        let tests = vec![
            (r"hey-&-ho", "hey-&amp;-ho"),
            (r"hey-'-ho", "hey-&apos;-ho"),
            (r"hey-&'-ho", "hey-&amp;&apos;-ho"),
            (r#"hey-&'"-ho"#, "hey-&amp;&apos;&quot;-ho"),
            (r#"hey-&'"<-ho"#, "hey-&amp;&apos;&quot;&lt;-ho"),
            (r#"hey-&'"<>-ho"#, "hey-&amp;&apos;&quot;&lt;&gt;-ho"),
        ];
        for (input, expected) in tests {
            let result = escape_xml(&to_value(input).unwrap(), &HashMap::new());
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), to_value(expected).unwrap());
        }
    }

    #[test]
    fn test_int_decimal_strings() {
        let tests: Vec<(&str, i64)> = vec![
            ("0", 0),
            ("-5", -5),
            ("9223372036854775807", i64::max_value()),
            ("0b1010", 0),
            ("1.23", 1),
        ];
        for (input, expected) in tests {
            let args = HashMap::new();
            let result = int(&to_value(input).unwrap(), &args);

            assert!(result.is_ok());
            assert_eq!(result.unwrap(), to_value(expected).unwrap());
        }
    }

    #[test]
    fn test_int_others() {
        let mut args = HashMap::new();

        let result = int(&to_value(1.23).unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(1).unwrap());

        let result = int(&to_value(-5).unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(-5).unwrap());

        args.insert("default".to_string(), to_value(5).unwrap());
        args.insert("base".to_string(), to_value(2).unwrap());
        let tests: Vec<(&str, i64)> =
            vec![("0", 0), ("-3", 5), ("1010", 10), ("0b1010", 10), ("0xF00", 5)];
        for (input, expected) in tests {
            let result = int(&to_value(input).unwrap(), &args);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), to_value(expected).unwrap());
        }

        args.insert("default".to_string(), to_value(-4).unwrap());
        args.insert("base".to_string(), to_value(8).unwrap());
        let tests: Vec<(&str, i64)> =
            vec![("21", 17), ("-3", -3), ("9OO", -4), ("0o567", 375), ("0b101", -4)];
        for (input, expected) in tests {
            let result = int(&to_value(input).unwrap(), &args);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), to_value(expected).unwrap());
        }

        args.insert("default".to_string(), to_value(0).unwrap());
        args.insert("base".to_string(), to_value(16).unwrap());
        let tests: Vec<(&str, i64)> = vec![("1011", 4113), ("0xC3", 195)];
        for (input, expected) in tests {
            let result = int(&to_value(input).unwrap(), &args);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), to_value(expected).unwrap());
        }

        args.insert("default".to_string(), to_value(0).unwrap());
        args.insert("base".to_string(), to_value(5).unwrap());
        let tests: Vec<(&str, i64)> = vec![("4321", 586), ("-100", -25), ("0b100", 0)];
        for (input, expected) in tests {
            let result = int(&to_value(input).unwrap(), &args);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), to_value(expected).unwrap());
        }
    }

    #[test]
    fn test_float() {
        let mut args = HashMap::new();

        let tests: Vec<(&str, f64)> = vec![("0", 0.0), ("-5.3", -5.3)];
        for (input, expected) in tests {
            let result = float(&to_value(input).unwrap(), &args);

            assert!(result.is_ok());
            assert_eq!(result.unwrap(), to_value(expected).unwrap());
        }

        args.insert("default".to_string(), to_value(3.18).unwrap());
        let result = float(&to_value("bad_val").unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(3.18).unwrap());

        let result = float(&to_value(1.23).unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(1.23).unwrap());
    }

    #[test]
    fn test_linebreaksbr() {
        let args = HashMap::new();
        let tests: Vec<(&str, &str)> = vec![
            ("hello world", "hello world"),
            ("hello\nworld", "hello<br>world"),
            ("hello\r\nworld", "hello<br>world"),
            ("hello\n\rworld", "hello<br>\rworld"),
            ("hello\r\n\nworld", "hello<br><br>world"),
            ("hello<br>world\n", "hello<br>world<br>"),
        ];
        for (input, expected) in tests {
            let result = linebreaksbr(&to_value(input).unwrap(), &args);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), to_value(expected).unwrap());
        }
    }
}
