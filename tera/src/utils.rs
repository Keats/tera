use std::fmt;
use std::ops::{Deref, Range};

#[derive(Clone, PartialEq)]
pub struct Spanned<T: fmt::Debug> {
    node: Box<T>,
    span: Span,
}

impl<T: fmt::Debug> Spanned<T> {
    pub fn new(node: T, span: Span) -> Self {
        Self {
            node: Box::new(node),
            span,
        }
    }

    pub fn span(&self) -> &Span {
        &self.span
    }

    pub fn span_mut(&mut self) -> &mut Span {
        &mut self.span
    }

    pub(crate) fn node(&self) -> &T {
        &self.node
    }

    pub fn into_parts(self) -> (T, Span) {
        (*self.node, self.span)
    }
}

impl<T: fmt::Debug> Deref for Spanned<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}

impl<T: fmt::Debug> fmt::Debug for Spanned<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.node, f)?;
        write!(f, "{:?}", self.span)
    }
}

/// The position of a node in the template source
#[derive(Clone, PartialEq, Eq, Default)]
pub struct Span {
    /// Line the span starts on, 1-based
    pub start_line: usize,
    /// Column the span starts on, 0-based
    pub start_col: usize,
    /// Line the span ends on, 1-based
    pub end_line: usize,
    /// Column the span ends on, 0-based
    pub end_col: usize,
    /// Byte range of the span in the template source
    pub range: Range<usize>,
}

impl fmt::Debug for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            " @ {}:{}-{}:{} ({:?})",
            self.start_line, self.start_col, self.end_line, self.end_col, self.range,
        )
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            " @ {}:{}-{}:{}",
            self.start_line, self.start_col, self.end_line, self.end_col,
        )
    }
}
impl Span {
    pub(crate) fn expand(&mut self, other: &Span) {
        self.end_line = other.end_line;
        self.end_col = other.end_col;
        self.range = self.range.start..other.range.end;
    }
}

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
/// ' --> &#39;
/// ```
#[inline]
pub fn escape_html(input: &str, buf: &mut dyn std::io::Write) -> std::io::Result<()> {
    #[cfg(feature = "fast_escape")]
    {
        use pulldown_cmark_escape::IoWriter;
        pulldown_cmark_escape::escape_html(IoWriter(buf), input)?;
        Ok(())
    }

    #[cfg(not(feature = "fast_escape"))]
    {
        for c in input.as_bytes() {
            match c {
                b'&' => buf.write_all(b"&amp;")?,
                b'<' => buf.write_all(b"&lt;")?,
                b'>' => buf.write_all(b"&gt;")?,
                b'"' => buf.write_all(b"&quot;")?,
                b'\'' => buf.write_all(b"&#39;")?,
                _ => buf.write_all(&[*c])?,
            };
        }
        Ok(())
    }
}
