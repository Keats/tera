use crate::errors::Error;

/// Escape HTML following [OWASP](https://www.owasp.org/index.php/XSS_(Cross_Site_Scripting)_Prevention_Cheat_Sheet)
///
/// Escape the following characters with HTML entity encoding to prevent switching
/// into any execution context, such as script, style, or event handlers. Using
/// hex entities is recommended in the spec. In addition to the 5 characters
/// significant in XML (&, <, >, ", '), the forward slash is included as it helps
/// to end an HTML entity.
///
/// ```text
/// & --> &amp;
/// < --> &lt;
/// > --> &gt;
/// " --> &quot;
/// ' --> &#x27;     &apos; is not recommended
/// / --> &#x2F;     forward slash is included as it helps end an HTML entity
/// ```
#[inline]
pub fn escape_html(input: &str) -> String {
    let mut output = String::with_capacity(input.len() * 2);
    for c in input.chars() {
        match c {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&#x27;"),
            '/' => output.push_str("&#x2F;"),
            _ => output.push(c),
        }
    }

    // Not using shrink_to_fit() on purpose
    output
}

pub(crate) fn render_to_string<C, F, E>(context: C, render: F) -> Result<String, Error>
where
    C: FnOnce() -> String,
    F: FnOnce(&mut Vec<u8>) -> Result<(), E>,
    Error: From<E>,
{
    let mut buffer = Vec::new();
    render(&mut buffer).map_err(Error::from)?;
    buffer_to_string(context, buffer)
}

pub(crate) fn buffer_to_string<F>(context: F, buffer: Vec<u8>) -> Result<String, Error>
where
    F: FnOnce() -> String,
{
    String::from_utf8(buffer).map_err(|error| Error::utf8_conversion_error(error, context()))
}

#[cfg(test)]
mod tests {
    use super::escape_html;
    use super::render_to_string;

    #[test]
    fn test_escape_html() {
        let tests = vec![
            (r"", ""),
            (r"a&b", "a&amp;b"),
            (r"<a", "&lt;a"),
            (r">a", "&gt;a"),
            (r#"""#, "&quot;"),
            (r#"'"#, "&#x27;"),
            (r#"大阪"#, "大阪"),
        ];
        for (input, expected) in tests {
            assert_eq!(escape_html(input), expected);
        }
        let empty = String::new();
        assert_eq!(escape_html(&empty), empty);
    }

    #[test]
    fn test_render_to_string() {
        use std::io::Write;
        let string = render_to_string(|| panic!(), |w| write!(w, "test")).unwrap();
        assert_eq!(string, "test".to_owned());
    }
}
