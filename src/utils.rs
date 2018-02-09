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
#[inline] // TODO: check if the inline matters in benches
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
            // Additional one for old IE (unpatched IE8 and below)
            // See https://github.com/OWASP/owasp-java-encoder/wiki/Grave-Accent-Issue
            '`' => output.push_str("&#96;"),
            _ => output.push(c),
        }
    }

    // Not using shrink_to_fit() on purpose
    output
}

#[cfg(test)]
mod tests {
    use super::escape_html;

    #[test]
    fn test_escape_html() {
        let tests = vec![
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
    }
}
