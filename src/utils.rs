/// Contains helper functions

/// From https://www.owasp.org/index.php/XSS_(Cross_Site_Scripting)_Prevention_Cheat_Sheet
/// Escape the following characters with HTML entity encoding to prevent switching
/// into any execution context, such as script, style, or event handlers. Using
/// hex entities is recommended in the spec. In addition to the 5 characters
/// significant in XML (&, <, >, ", '), the forward slash is included as it helps
/// to end an HTML entity.
///
/// & --> &amp;
/// < --> &lt;
/// > --> &gt;
/// " --> &quot;
/// ' --> &#x27;     &apos; is not recommended
/// / --> &#x2F;     forward slash is included as it helps end an HTML entity
pub fn escape_html(input: &str) -> String {
    let mut output = String::with_capacity(input.len() * 2);
    for c in input.as_bytes() {
        match *c as char {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&#x27;"),
            '/' => output.push_str("&#x2F;"),
            _ => output.push(*c as char)
        }
    }

    output.shrink_to_fit();
    output
}
