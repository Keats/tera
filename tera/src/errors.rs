//! The Tera error type, with optional nice terminal error reporting.
use std::fmt::{self};

use crate::reporting::generate_report;
use std::error::Error as StdError;

use crate::utils::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Note {
    pub(crate) filename: String,
    pub(crate) source: String,
    pub(crate) span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportError {
    pub(crate) message: String,
    pub(crate) filename: String,
    pub(crate) source: String,
    pub(crate) span: Span,
    pub(crate) notes: Vec<Note>,
}

impl ReportError {
    pub(crate) fn new(message: String, filename: &str, source: &str, span: &Span) -> Self {
        Self {
            message,
            filename: filename.to_string(),
            source: source.to_string(),
            span: span.clone(),
            notes: Vec::new(),
        }
    }

    /// Create a ReportError without filename/source - must call set_source before generating report
    pub(crate) fn new_without_source(message: String, span: &Span) -> Self {
        Self {
            message,
            filename: String::new(),
            source: String::new(),
            span: span.clone(),
            notes: Vec::new(),
        }
    }

    pub(crate) fn set_source(&mut self, filename: &str, source: &str) {
        self.filename = filename.to_string();
        self.source = source.to_string();
    }

    pub(crate) fn add_note(&mut self, filename: &str, source: &str, span: &Span) {
        self.notes.push(Note {
            filename: filename.to_string(),
            source: source.to_string(),
            span: span.clone(),
        });
    }

    pub fn generate_report(&self) -> String {
        generate_report(self)
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn span(&self) -> &Span {
        &self.span
    }

    pub fn filename(&self) -> &str {
        &self.filename
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub(crate) fn unexpected_end_of_input(span: &Span) -> Self {
        Self::new_without_source("Unexpected end of input".to_string(), span)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorKind {
    /// Generic error
    Msg(String),
    /// Both lexer and parser errors. Will point to the source file
    SyntaxError(Box<ReportError>),
    /// An error that happens while rendering a template. Will point to the source file
    RenderingError(Box<ReportError>),
    /// A loop was found while looking up the inheritance chain
    CircularExtend {
        /// Name of the template with the loop
        tpl: String,
        /// All the parents templates we found so far
        inheritance_chain: Vec<String>,
    },
    /// A loop was found while following `{% include %}` calls between templates
    CircularInclude {
        /// Name of the template where the cycle was detected
        tpl: String,
        /// Ordered chain of templates from `tpl` back to `tpl`
        include_chain: Vec<String>,
    },
    /// A template is extending a template that wasn't found in the Tera instance
    MissingParent {
        /// The template we are currently looking at
        current: String,
        /// The missing template
        parent: String,
    },
    /// A template is calling a macro namespace that is not loaded
    NamespaceNotLoaded {
        /// Name of the template with the issue
        tpl: String,
        /// The namespace causing problems
        namespace: String,
    },
    /// The template is calling a macro which isn't found in the namespace
    MacroNotFound {
        /// Name of the template with the issue
        tpl: String,
        /// The namespace used
        namespace: String,
        /// The name of the macro that cannot be found
        name: String,
    },
    /// A template was missing
    TemplateNotFound(String),
    /// A component was missing
    /// Only raised if you're trying to render a specific component
    ComponentNotFound(String),
    /// A filter/test main value was not the expected type
    InvalidArgument {
        expected_type: String,
        actual_type: String,
    },
    /// A function/test/filter was expecting an argument but it wasn't found
    MissingArgument { arg_name: String },
    /// An IO error occurred
    Io(std::io::ErrorKind),
    /// UTF-8 conversion error when converting output to UTF-8
    ///
    /// This should not occur unless invalid UTF8 chars are rendered
    Utf8Conversion,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorKind::Msg(message) => write!(f, "{message}"),
            ErrorKind::SyntaxError(s) | ErrorKind::RenderingError(s) => {
                write!(f, "{}", s.generate_report())
            }
            ErrorKind::CircularExtend {
                tpl,
                inheritance_chain,
            } => write!(
                f,
                "Circular extend detected for template '{tpl}'. Inheritance chain: `{inheritance_chain:?}`",
            ),
            ErrorKind::CircularInclude { tpl, include_chain } => write!(
                f,
                "Circular include detected for template '{tpl}'. Include chain: `{include_chain:?}`",
            ),
            ErrorKind::MissingParent { current, parent } => write!(
                f,
                "Template '{current}' is inheriting from '{parent}', which doesn't exist or isn't loaded.",
            ),
            ErrorKind::TemplateNotFound(name) => write!(f, "Template '{name}' not found"),
            ErrorKind::ComponentNotFound(name) => write!(f, "Component '{name}' not found"),
            ErrorKind::NamespaceNotLoaded { tpl, namespace } => write!(
                f,
                "Template '{tpl}' is trying to use namespace `{namespace}` which is not loaded",
            ),
            ErrorKind::MacroNotFound {
                tpl,
                namespace,
                name,
            } => write!(
                f,
                "Template '{tpl}' is using macro `{namespace}::{name}` which is not found in the namespace",
            ),
            ErrorKind::InvalidArgument {
                expected_type,
                actual_type,
            } => write!(
                f,
                "Invalid type for the value, expected `{expected_type}` but got `{actual_type}`"
            ),
            ErrorKind::MissingArgument { arg_name } => {
                write!(f, "Missing keyword argument `{arg_name}`")
            }
            ErrorKind::Io(io_error) => {
                write!(
                    f,
                    "Io error while writing rendered value to output: {:?}",
                    io_error
                )
            }
            ErrorKind::Utf8Conversion => {
                write!(f, "Invalid UTF-8 characters found while rendering.")
            }
        }
    }
}

#[derive(Debug)]
pub struct Error {
    pub(crate) kind: ErrorKind,
    // If the error comes from some third party libs, TODO we need that?
    pub(crate) source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)
    }
}

impl Error {
    pub fn new(kind: ErrorKind) -> Self {
        Self { kind, source: None }
    }

    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    /// Creates generic error with a source
    pub fn chain(value: impl ToString, source: impl Into<Box<dyn StdError + Send + Sync>>) -> Self {
        Self {
            kind: ErrorKind::Msg(value.to_string()),
            source: Some(source.into()),
        }
    }

    pub fn message(message: impl ToString) -> Self {
        Self {
            kind: ErrorKind::Msg(message.to_string()),
            source: None,
        }
    }

    pub(crate) fn syntax_error(message: String, span: &Span) -> Self {
        Self {
            kind: ErrorKind::SyntaxError(Box::new(ReportError::new_without_source(message, span))),
            source: None,
        }
    }

    pub(crate) fn circular_extend(tpl: impl ToString, inheritance_chain: Vec<String>) -> Self {
        Self {
            kind: ErrorKind::CircularExtend {
                tpl: tpl.to_string(),
                inheritance_chain,
            },
            source: None,
        }
    }

    pub(crate) fn circular_include(tpl: impl ToString, include_chain: Vec<String>) -> Self {
        Self {
            kind: ErrorKind::CircularInclude {
                tpl: tpl.to_string(),
                include_chain,
            },
            source: None,
        }
    }

    pub(crate) fn missing_parent(current: impl ToString, parent: impl ToString) -> Self {
        Self {
            kind: ErrorKind::MissingParent {
                current: current.to_string(),
                parent: parent.to_string(),
            },
            source: None,
        }
    }

    pub(crate) fn io_error(error: std::io::Error) -> Self {
        Self {
            kind: ErrorKind::Io(error.kind()),
            source: Some(Box::new(error)),
        }
    }

    pub(crate) fn template_not_found(tpl: impl ToString) -> Self {
        Self {
            kind: ErrorKind::TemplateNotFound(tpl.to_string()),
            source: None,
        }
    }

    pub(crate) fn component_not_found(name: impl ToString) -> Self {
        Self {
            kind: ErrorKind::ComponentNotFound(name.to_string()),
            source: None,
        }
    }

    pub(crate) fn invalid_arg_type(
        expected_type: impl ToString,
        actual_type: impl ToString,
    ) -> Self {
        Self {
            kind: ErrorKind::InvalidArgument {
                expected_type: expected_type.to_string(),
                actual_type: actual_type.to_string(),
            },
            source: None,
        }
    }

    pub(crate) fn missing_arg(arg_name: impl ToString) -> Self {
        Self {
            kind: ErrorKind::MissingArgument {
                arg_name: arg_name.to_string(),
            },
            source: None,
        }
    }

    pub(crate) fn invalid_utf8(error: std::string::FromUtf8Error) -> Self {
        Self {
            kind: ErrorKind::Utf8Conversion,
            source: Some(Box::new(error)),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_ref().map(|e| e.as_ref() as _)
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::io_error(error)
    }
}

impl From<std::string::FromUtf8Error> for Error {
    fn from(error: std::string::FromUtf8Error) -> Self {
        Self::invalid_utf8(error)
    }
}

pub type TeraResult<T> = Result<T, Error>;

#[cfg(test)]
mod tests {
    #[test]
    fn test_error_is_send_and_sync() {
        fn test_send_sync<T: Send + Sync>() {}

        test_send_sync::<super::Error>();
    }
}
