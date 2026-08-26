use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};

use regex::Regex;
use tera::{Filter, Kwargs, State, StringInput, TeraResult, Test, Value};

static STRIPTAGS_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(<!--.*?-->|<[^>]*>)").unwrap());

static SPACELESS_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r">\s+<").unwrap());

/// Tries to remove HTML tags from input. Does not guarantee well-formed output if input is not valid HTML.
///
/// If value is "<b>Joel</b>", the output will be "Joel".
/// Note that if the template you are using it in is automatically escaped, you will need to call the safe filter after striptags.
///
/// ```text
/// {{ value | striptags }}
/// ```
pub fn striptags(val: &str, _: Kwargs, _: &State) -> String {
    STRIPTAGS_RE.replace_all(val, "").into_owned()
}

/// Remove space ( ) and line breaks (\n or \r\n) between HTML tags.
///
/// If the value is "<p>\n<a> </a>\r\n </p>", the output will be "<p><a></a></p>".
/// Note that only whitespace between successive opening tags and successive closing tags is removed.
/// Also note that if the template you are using it in is automatically escaped, you will need to call the safe filter after spaceless.
///
/// ```text
/// {{ value | spaceless }}
/// ```
pub fn spaceless(val: StringInput, _: Kwargs, _: &State) -> Value {
    // We're removing spaces between HTML tags so if it was safe before, it should still be safe
    // Of course if it's used outside of HTML content it might be wrong.
    val.inherit_safety(SPACELESS_RE.replace_all(val.as_str(), "><").into_owned())
}

fn get_or_create_regex(cache: &RwLock<HashMap<String, Regex>>, pattern: &str) -> TeraResult<Regex> {
    if let Some(r) = cache.read().unwrap().get(pattern) {
        return Ok(r.clone());
    }

    let mut cache = cache.write().unwrap();

    let regex = match Regex::new(pattern) {
        Ok(regex) => regex,
        Err(e) => return Err(tera::Error::message(format!("Invalid regex: {e}"))),
    };

    cache.insert(String::from(pattern), regex.clone());
    Ok(regex)
}

/// Returns true if the given variable is a string and matches the regex in the `pat` argument.
/// The regex will only be compiled once.
///
/// ```text
/// {% if value is matching(pat="^hello") %}...{% endif %}
/// ```
#[derive(Debug, Default)]
pub struct Matching {
    cache: RwLock<HashMap<String, Regex>>,
}

impl Test<&str, TeraResult<bool>> for Matching {
    fn call(&self, val: &str, kwargs: Kwargs, _: &State) -> TeraResult<bool> {
        let pat = kwargs.must_get::<&str>("pat")?;
        let regex = get_or_create_regex(&self.cache, pat)?;
        Ok(regex.is_match(val))
    }
}

/// Takes 2 mandatory string named arguments: `pattern` (regex pattern) and `rep`.
/// This will replace all occurrences of `pattern` with `rep`.
/// The regex will only be compiled once.
///
/// ```text
/// {{ value | regex_replace(pattern="\d+", rep="") }}
/// ```
#[derive(Debug, Default)]
pub struct RegexReplace {
    cache: RwLock<HashMap<String, Regex>>,
}

impl Filter<&str, TeraResult<String>> for RegexReplace {
    fn call(&self, val: &str, kwargs: Kwargs, _: &State) -> TeraResult<String> {
        let pattern = kwargs.must_get::<&str>("pattern")?;
        let rep = kwargs.must_get::<&str>("rep")?;
        let regex = get_or_create_regex(&self.cache, pattern)?;
        Ok(regex.replace_all(val, rep).into_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tera::value::Map;
    use tera::{ArgFromValue, Context, StringInput};

    #[test]
    fn test_striptags() {
        let tests = vec![
            (
                r"<b>Joel</b> <button>is</button> a <span>slug</span>",
                "Joel is a slug",
            ),
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
            (
                r#"<strong>foo</strong><a href="http://example.com">bar</a>"#,
                "foobar",
            ),
            ("a &amp; b", "a &amp; b"),
        ];
        for (input, expected) in tests {
            let ctx = Context::new();
            let state = State::new(&ctx);
            let res = striptags(input, Kwargs::default(), &state);
            assert_eq!(expected, res);
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
            let ctx = Context::new();
            let state = State::new(&ctx);
            let val = Value::from(input);
            let res = spaceless(
                StringInput::from_value(&val).unwrap(),
                Kwargs::default(),
                &state,
            );
            assert_eq!(expected, res.as_str().unwrap());
        }
    }

    #[test]
    fn test_matching() {
        let inputs = vec![
            ("abc", "b", true),
            ("abc", "^b$", false),
            ("Hello, World!", r"(?i)(hello\W\sworld\W)", true),
            ("The date was 2018-06-28", r"\d{4}-\d{2}-\d{2}$", true),
        ];

        for (input, pat, expected) in inputs {
            let matching = Matching::default();
            let mut map = Map::new();
            map.insert("pat".into(), pat.into());
            let kwargs = Kwargs::new(Arc::new(map));
            let ctx = Context::new();
            let res = matching.call(input, kwargs, &State::new(&ctx)).unwrap();
            assert_eq!(expected, res);
        }
    }

    #[test]
    fn test_regex_replace() {
        let regex_replace = RegexReplace::default();
        let ctx = Context::new();
        let state = State::new(&ctx);

        // Basic replacement with capture groups
        let mut map = Map::new();
        map.insert(
            "pattern".into(),
            r"(?P<last>[^,\s]+),\s+(?P<first>\S+)".into(),
        );
        map.insert("rep".into(), "$first $last".into());
        let kwargs = Kwargs::new(Arc::new(map));
        let result = regex_replace
            .call("Springsteen, Bruce", kwargs, &state)
            .unwrap();
        assert_eq!(result, "Bruce Springsteen");

        // Simple replacement
        let mut map = Map::new();
        map.insert("pattern".into(), r"\d+".into());
        map.insert("rep".into(), "X".into());
        let kwargs = Kwargs::new(Arc::new(map));
        let result = regex_replace.call("abc123def456", kwargs, &state).unwrap();
        assert_eq!(result, "abcXdefX");

        // No match returns original
        let mut map = Map::new();
        map.insert("pattern".into(), r"zzz".into());
        map.insert("rep".into(), "X".into());
        let kwargs = Kwargs::new(Arc::new(map));
        let result = regex_replace.call("hello world", kwargs, &state).unwrap();
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_regex_replace_invalid_pattern() {
        let regex_replace = RegexReplace::default();
        let ctx = Context::new();
        let state = State::new(&ctx);

        let mut map = Map::new();
        map.insert("pattern".into(), r"[invalid".into());
        map.insert("rep".into(), "X".into());
        let kwargs = Kwargs::new(Arc::new(map));
        let result = regex_replace.call("test", kwargs, &state);
        assert!(result.is_err());
    }

    #[test]
    fn test_register() {
        let mut tera = tera::Tera::default();
        tera.register_filter("striptags", striptags);
        tera.register_filter("spaceless", spaceless);
        tera.register_filter("regex_replace", RegexReplace::default());
        tera.register_test("matching", Matching::default());
    }
}
