/// Filters operating on string
use std::collections::HashMap;

use serde_json::value::{Value, to_value};
use slug;

use errors::{TeraResult, TeraError};


/// Convert a value to uppercase.
pub fn upper(value: Value, _: HashMap<String, Value>) -> TeraResult<Value> {
    let s = try_get_value!("upper", "value", String, value);

    Ok(to_value(&s.to_uppercase()))
}

/// Convert a value to lowercase.
pub fn lower(value: Value, _: HashMap<String, Value>) -> TeraResult<Value> {
    let s = try_get_value!("lower", "value", String, value);

    Ok(to_value(&s.to_lowercase()))
}

/// Strip leading and trailing whitespace.
pub fn trim(value: Value, _: HashMap<String, Value>) -> TeraResult<Value> {
    let s = try_get_value!("trim", "value", String, value);

    Ok(to_value(&s.trim()))
}

/// Truncates a string to the indicated length
pub fn truncate(value: Value, args: HashMap<String, Value>) -> TeraResult<Value> {
    let s = try_get_value!("truncate", "value", String, value);
    let length = match args.get("length") {
        Some(l) => try_get_value!("truncate", "length", usize, l.clone()),
        None => 255
    };

    // Nothing to truncate?
    if length > s.len() {
        return Ok(to_value(&s));
    }

    let result = s[..s.char_indices().nth(length).unwrap().0].to_string() + "…";
    Ok(to_value(&result))
}

/// Gets the number of words in a string.
pub fn wordcount(value: Value, _: HashMap<String, Value>) -> TeraResult<Value> {
    let s = try_get_value!("wordcount", "value", String, value);

    Ok(to_value(&s.split_whitespace().count()))
}

/// Replaces given `from` substring with `to` string.
pub fn replace(value: Value, args: HashMap<String, Value>) -> TeraResult<Value> {
    let s = try_get_value!("replace", "value", String, value);

    let from = match args.get("from") {
        Some(val) => try_get_value!("replace", "from", String, val.clone()),
        None => {
            return Err(TeraError::FilterMissingArg("replace".to_string(), "from".to_string()));
        }
    };

    let to = match args.get("to") {
        Some(val) => try_get_value!("replace", "to", String, val.clone()),
        None => {
            return Err(TeraError::FilterMissingArg("replace".to_string(), "to".to_string()));
        }
    };

    Ok(to_value(&s.replace(&from, &to)))
}

/// First letter of the string is uppercase rest is lowercase
pub fn capitalize(value: Value, _: HashMap<String, Value>) -> TeraResult<Value> {
    let s = try_get_value!("capitalize", "value", String, value);
    let mut chars = s.chars();
    match chars.next() {
        None => Ok(to_value("")),
        Some(f) => {
            let res = f.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase();
            Ok(to_value(&res))
        }
    }
}

/// Escapes quote characters
pub fn addslashes(value: Value, _: HashMap<String, Value>) -> TeraResult<Value> {
    let s = try_get_value!("addslashes", "value", String, value);
    Ok(to_value(&s.replace("\\","\\\\").replace("\"", "\\\"").replace("\'", "\\\'")))
}

/// Transform a string into a slug
pub fn slugify(value: Value, _: HashMap<String, Value>) -> TeraResult<Value> {
    let s = try_get_value!("slugify", "value", String, value);
    Ok(to_value(&slug::slugify(s)))
}

/// Capitalizes each word in the string
pub fn title(value: Value, _: HashMap<String, Value>) -> TeraResult<Value> {
    let s = try_get_value!("title", "value", String, value);
    let words = s.split_whitespace().collect::<Vec<_>>();
    let result = words.iter().map(|&word|{
                    let mut characters = word.chars();
                    match characters.next() {
                        None => "".to_string(),
                        Some(f) => f.to_uppercase().collect::<String>() + &characters.as_str().to_lowercase(),
                    }   
                }).collect::<Vec<_>>().join(" ");
    Ok(to_value(&result))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serde_json::value::{to_value};

    use errors::TeraError::*;

    use super::*;

    #[test]
    fn test_upper() {
        let result = upper(to_value("hello"), HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("HELLO"));
    }

    #[test]
    fn test_upper_error() {
        let result = upper(to_value(&50), HashMap::new());
        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap(),
            FilterIncorrectArgType("upper".to_string(), "value".to_string(), to_value(&50), "String".to_string())
        );
    }

    #[test]
    fn test_trim() {
        let result = trim(to_value("  hello  "), HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("hello"));
    }

    #[test]
    fn test_truncate_smaller_than_length() {
        let mut args = HashMap::new();
        args.insert("length".to_string(), to_value(&255));
        let result = truncate(to_value("hello"), args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("hello"));
    }

    #[test]
    fn test_truncate_when_required() {
        let mut args = HashMap::new();
        args.insert("length".to_string(), to_value(&2));
        let result = truncate(to_value("日本語"), args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("日本…"));
    }

    #[test]
    fn test_lower() {
        let result = lower(to_value("HELLO"), HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("hello"));
    }

    #[test]
    fn test_wordcount() {
        let result = wordcount(to_value("Joel is a slug"), HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(&4));
    }

    #[test]
    fn test_replace() {
        let mut args = HashMap::new();
        args.insert("from".to_string(), to_value(&"Hello"));
        args.insert("to".to_string(), to_value(&"Goodbye"));
        let result = replace(to_value(&"Hello world!"), args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("Goodbye world!"));
    }

    #[test]
    fn test_replace_missing_arg() {
        let mut args = HashMap::new();
        args.insert("from".to_string(), to_value(&"Hello"));
        let result = replace(to_value(&"Hello world!"), args);
        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap(),
            FilterMissingArg("replace".to_string(), "to".to_string())
        );
    }

    #[test]
    fn test_capitalize() {
        let tests = vec![
            ("CAPITAL IZE", "Capital ize"),
            ("capital ize", "Capital ize"),
        ];
        for (input, expected) in tests {
            let result = capitalize(to_value(input), HashMap::new());
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), to_value(expected));
        }
    }

    #[test]
    fn test_addslashes() {
        let tests = vec![
            (r#"I'm so happy"#, r#"I\'m so happy"#),
            (r#"Let "me" help you"#, r#"Let \"me\" help you"#),
            (r#"<a>'"#, r#"<a>\'"#),
            (r#""double quotes" and \'single quotes\'"#, r#"\"double quotes\" and \\\'single quotes\\\'"#),
            (r#"\ : backslashes too"#, r#"\\ : backslashes too"#)
        ];
        for (input, expected) in tests {
            let result = addslashes(to_value(input), HashMap::new());
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), to_value(expected));
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
            let result = slugify(to_value(input), HashMap::new());
            println!("{:?} - {:?}", input, result);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), to_value(expected));
        }
    }

    #[test]
    fn test_title() {
        let tests = vec![
            ("foo bar", "Foo Bar"),
            ("foo\tbar", "Foo Bar"),
        ];
        for (input, expected) in tests {
            let result = title(to_value(input), HashMap::new());
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), to_value(expected));
        }
    }
}
