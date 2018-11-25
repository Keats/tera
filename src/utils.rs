use std::str;

// From https://github.com/djc/askama/tree/master/askama_escape
// Adapted for use in Tera
macro_rules! escaping_body {
    ($start:ident, $i:ident, $fmt:ident, $bytes:ident, $quote:expr) => {{
        if $start < $i {
            $fmt.push_str(unsafe { str::from_utf8_unchecked(&$bytes[$start..$i]) });
        }
        $fmt.push_str($quote);
        $start = $i + 1;
    }};
}

const FLAG: u8 = b'>' - b'"';

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
    let mut start = 0;
    let mut output = String::with_capacity(input.len() + input.len()/2);
    let bytes = input.as_bytes();
    for (i, b) in bytes.iter().enumerate() {
        if b.wrapping_sub(b'"') <= FLAG {
            match *b {
                b'<' => escaping_body!(start, i, output, bytes, "&lt;"),
                b'>' => escaping_body!(start, i, output, bytes, "&gt;"),
                b'&' => escaping_body!(start, i, output, bytes, "&amp;"),
                b'"' => escaping_body!(start, i, output, bytes, "&quot;"),
                b'\'' => escaping_body!(start, i, output, bytes, "&#x27;"),
                b'/' => escaping_body!(start, i, output, bytes, "&#x2f;"),
                _ => (),
            }
        }
    }
    output.push_str(unsafe { str::from_utf8_unchecked(&bytes[start..]) });
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
