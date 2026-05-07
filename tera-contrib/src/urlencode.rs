use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, percent_encode};
use tera::{Kwargs, State};

/// https://url.spec.whatwg.org/#fragment-percent-encode-set
const FRAGMENT_ENCODE_SET: &AsciiSet = &percent_encoding::CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'<')
    .add(b'>')
    .add(b'`');

/// https://url.spec.whatwg.org/#path-percent-encode-set
const PATH_ENCODE_SET: &AsciiSet = &FRAGMENT_ENCODE_SET.add(b'#').add(b'?').add(b'{').add(b'}');

/// https://url.spec.whatwg.org/#userinfo-percent-encode-set
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
const PYTHON_ENCODE_SET: &AsciiSet = &USERINFO_ENCODE_SET
    .remove(b'/')
    .add(b'%')
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

/// Percent-encodes reserved URI characters.
/// Matches Python's `urllib.parse.quote` behavior with `/` not escaped.
///
/// ```text
/// {{ value | urlencode }}
/// ```
pub fn urlencode(val: &str, _: Kwargs, _: &State) -> String {
    percent_encode(val.as_bytes(), PYTHON_ENCODE_SET).to_string()
}

/// Percent-encodes all non-alphanumeric characters.
/// Stricter than `urlencode` - also encodes `/` and other typically safe characters.
///
/// ```text
/// {{ value | urlencode_strict }}
/// ```
pub fn urlencode_strict(val: &str, _: Kwargs, _: &State) -> String {
    percent_encode(val.as_bytes(), NON_ALPHANUMERIC).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tera::{Context, Kwargs, State};

    #[test]
    fn test_urlencode() {
        let ctx = Context::new();
        let state = State::new(&ctx);
        assert_eq!(
            urlencode("hello world", Kwargs::default(), &state),
            "hello%20world"
        );
        assert_eq!(
            urlencode("foo/bar?baz=1", Kwargs::default(), &state),
            "foo/bar%3Fbaz%3D1"
        );
    }

    #[test]
    fn test_urlencode_strict() {
        let ctx = Context::new();
        let state = State::new(&ctx);
        assert_eq!(
            urlencode_strict("hello", Kwargs::default(), &state),
            "hello"
        );
        assert_eq!(urlencode_strict("a/b", Kwargs::default(), &state), "a%2Fb");
    }

    #[test]
    fn test_register() {
        let mut tera = tera::Tera::default();
        tera.register_filter("urlencode", urlencode);
        tera.register_filter("urlencode_strict", urlencode_strict);
    }
}
