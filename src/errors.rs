use serde_json;
use std::convert::Into;
use std::error::Error as StdError;
use std::fmt;

/// The kind of an error (non-exhaustive)
#[derive(Debug)]
pub enum ErrorKind {
    /// Generic error
    Msg(String),
    /// A loop was found while looking up the inheritance chain
    CircularExtend {
        /// Name of the template with the loop
        tpl: String,
        /// All the parents templates we found so far
        inheritance_chain: Vec<String>,
    },
    /// A template is extending a template that wasn't found in the Tera instance
    MissingParent {
        /// The template we are currently looking at
        current: String,
        /// The missing template
        parent: String,
    },
    /// A template was missing (more generic version of MissingParent)
    TemplateNotFound(String),
    /// A filter wasn't found
    FilterNotFound(String),
    /// A test wasn't found
    TestNotFound(String),
    /// A macro was defined in a normal template
    InvalidMacroDefinition(String),
    /// A function wasn't found
    FunctionNotFound(String),
    /// An error happened while serializing JSON
    Json(serde_json::Error),
    /// This enum may grow additional variants, so this makes sure clients
    /// don't count on exhaustive matching. (Otherwise, adding a new variant
    /// could break existing code.)
    #[doc(hidden)]
    __Nonexhaustive,
}

/// The Error type
#[derive(Debug)]
pub struct Error {
    /// Kind of error
    pub kind: ErrorKind,
    source: Option<Box<dyn StdError>>,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.kind {
            ErrorKind::Msg(ref message) => write!(f, "{}", message),
            ErrorKind::CircularExtend { ref tpl, ref inheritance_chain } => write!(
                f,
                "Circular extend detected for template '{}'. Inheritance chain: `{:?}`",
                tpl, inheritance_chain
            ),
            ErrorKind::MissingParent { ref current, ref parent } => write!(
                f,
                "Template '{}' is inheriting from '{}', which doesn't exist or isn't loaded.",
                current, parent
            ),
            ErrorKind::TemplateNotFound(ref name) => write!(f, "Template '{}' not found", name),
            ErrorKind::FilterNotFound(ref name) => write!(f, "Filter '{}' not found", name),
            ErrorKind::TestNotFound(ref name) => write!(f, "Test '{}' not found", name),
            ErrorKind::FunctionNotFound(ref name) => write!(f, "Function '{}' not found", name),
            ErrorKind::InvalidMacroDefinition(ref info) => write!(f, "Invalid macro definition: `{}`", info),
            ErrorKind::Json(ref e) => write!(f, "{}", e),
            ErrorKind::__Nonexhaustive => write!(f, "Nonexhaustive"),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source.as_ref().map(|c| &**c)
    }
}

impl Error {
    /// Creates generic error
    pub fn msg(value: impl ToString) -> Self {
        Self { kind: ErrorKind::Msg(value.to_string()), source: None }
    }

    /// Creates a circular extend error
    pub fn circular_extend(tpl: impl ToString, inheritance_chain: Vec<String>) -> Self {
        Self {
            kind: ErrorKind::CircularExtend { tpl: tpl.to_string(), inheritance_chain },
            source: None,
        }
    }

    /// Creates a missing parent error
    pub fn missing_parent(current: impl ToString, parent: impl ToString) -> Self {
        Self {
            kind: ErrorKind::MissingParent {
                current: current.to_string(),
                parent: parent.to_string(),
            },
            source: None,
        }
    }

    /// Creates a template not found error
    pub fn template_not_found(tpl: impl ToString) -> Self {
        Self { kind: ErrorKind::TemplateNotFound(tpl.to_string()), source: None }
    }

    /// Creates a filter not found error
    pub fn filter_not_found(name: impl ToString) -> Self {
        Self { kind: ErrorKind::FilterNotFound(name.to_string()), source: None }
    }

    /// Creates a test not found error
    pub fn test_not_found(name: impl ToString) -> Self {
        Self { kind: ErrorKind::TestNotFound(name.to_string()), source: None }
    }

    /// Creates a function not found error
    pub fn function_not_found(name: impl ToString) -> Self {
        Self { kind: ErrorKind::FunctionNotFound(name.to_string()), source: None }
    }

    /// Creates generic error with a source
    pub fn chain(value: impl ToString, source: impl Into<Box<dyn StdError>>) -> Self {
        Self { kind: ErrorKind::Msg(value.to_string()), source: Some(source.into()) }
    }

    /// Creates JSON error
    pub fn json(value: serde_json::Error) -> Self {
        Self { kind: ErrorKind::Json(value), source: None }
    }

    /// Creates an invalid macro definition error
    pub fn invalid_macro_def(name: impl ToString) -> Self {
        Self { kind: ErrorKind::InvalidMacroDefinition(name.to_string()), source: None }
    }
}

impl From<&str> for Error {
    fn from(e: &str) -> Self {
        Self::msg(e)
    }
}
impl From<String> for Error {
    fn from(e: String) -> Self {
        Self::msg(e)
    }
}
impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::json(e)
    }
}
/// Convenient wrapper around std::Result.
pub type Result<T> = ::std::result::Result<T, Error>;
