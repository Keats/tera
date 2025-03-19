use crate::errors::Error;

/// Escape text for inclusion in HTML or XML body text or quoted attribute values.
///
/// This escapes more than is ever necessary in any given place, so that one method can be used for
/// almost forms of escaping ever needed in both HTML and XML. Here’s all that you actually *need*
/// to escape:
///
/// - In HTML body text: `<` and `&`;
/// - In HTML quoted attribute values: `&` and the quote (`'` or `"`);
/// - In XML body text: `<`, `>` and `&`;
/// - In XML quoted attribute values: `<`, `>`, `&` and the quote (`'` or `"`).
///
/// This method is only certified for use in these contexts. It may not be suitable in other
/// contexts; for example, inside a `<script>` tag’s body you need to do something else altogether,
/// as entity encoding won’t work but there are some sequences you need to avoid (e.g. `</script>`,
/// `<!--`).
///
/// In total, this method performs the following escapes:
///
/// - `<` → `&lt;`
/// - `>` → `&gt;`
/// - `&` → `&amp;`
/// - `"` → `&quot;`
/// - `'` → `&apos;`
#[inline]
pub fn escape_html(input: &str) -> String {
    let mut output = String::with_capacity(input.len() * 2);
    for c in input.chars() {
        match c {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&apos;"),
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
