use v_htmlescape::escape;
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
    escape(input).to_string()
}

#[cfg(test)]
mod tests {
    use super::escape_html;

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
}
