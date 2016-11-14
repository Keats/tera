/// Contains helper functions

// Escape HTML entity following https://www.owasp.org/index.php/XSS_(Cross_Site_Scripting)_Prevention_Cheat_Sheet
pub fn escape_html(input: &str) -> String {
    let mut output = String::with_capacity(input.len() * 2);
    for c in input.as_bytes() {
        match *c as char {
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '&' => output.push_str("&amp;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&#39;"),
            '`' => output.push_str("&#96;"),
            '/' => output.push_str("&#x2F;"),
            _ => output.push(*c as char)
        }
    }

    output.shrink_to_fit();
    output
}
